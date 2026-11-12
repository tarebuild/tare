import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import anchor from '@coral-xyz/anchor'
import type { Idl } from '@coral-xyz/anchor'
import { PublicKey } from '@solana/web3.js'
import { anchorProgram } from './engines/indexer/anchor.js'
import { decodeTransaction } from './engines/indexer/decoder.js'
import { toBase58 } from './engines/core/encoding.js'
import { deriveFlagAddress, deriveRecordAddress, registryFacts, registryPdas } from './registry-decoder.js'

const idl = JSON.parse(readFileSync(new URL('../registry/target/idl/tare_registry.json', import.meta.url), 'utf8')) as Idl
const program = anchorProgram(idl)
const coder = new anchor.BorshCoder(idl)
const registry = new PublicKey(idl.address)
const pdas = registryPdas(idl.address)
const key = (seed: number) => new PublicKey(new Uint8Array(32).fill(seed))
const payer = key(1), application = key(9), stranger = key(5), launch = key(7)
const record = new PublicKey(deriveRecordAddress(idl.address, { program: application.toBase58(), reportHash: '0d'.repeat(32), attester: payer.toBase58() }))
const flag = new PublicKey(deriveFlagAddress(idl.address, { record: record.toBase58(), inputHash: '0e'.repeat(32) }))
const eventTag = Buffer.from('e445a52e51cb9a1d', 'hex')
const configData = { admin: payer, fee_recipient: launch, attest_fee_lamports: new anchor.BN(1000) }
const recordData = {
  program: application, stock_build_hash: new Array(32).fill(10), tare_build_hash: new Array(32).fill(11),
  source_repository: 'https://example.com/fixture/program', source_commit: 'a'.repeat(40), toolchain_version: 'fixture-0.1.0',
  corpus_hash: new Array(32).fill(12), tests_count: new anchor.BN('2147483648'), generated_count: new anchor.BN(10), mainnet_count: new anchor.BN(3),
  gross: new anchor.BN(9000), net: new anchor.BN(4000), report_hash: new Array(32).fill(13),
  report_location: '/v1/reports/' + '0d'.repeat(32), attester: payer, slot: new anchor.BN(101),
  fee_recipient: launch, fee_lamports: new anchor.BN(1000),
}
const flagData = { record, input_hash: new Array(32).fill(14), input_location: '/v1/flag-inputs/' + '0e'.repeat(32),
  flagger: payer, slot: new anchor.BN(101) }

function eventBytes(name: string, data: unknown) {
  const event = idl.events!.find((value) => value.name === name)!
  return Buffer.concat([Buffer.from(event.discriminator), coder.types.encode(name, data)])
}

function rawTransaction() {
  const accounts: Record<string, string> = {
    authority: payer.toBase58(), admin: payer.toBase58(), attester: payer.toBase58(), flagger: payer.toBase58(),
    registry_program: registry.toBase58(), program: registry.toBase58(), program_data: key(8).toBase58(),
    config: pdas.config, fee_recipient: launch.toBase58(), record: record.toBase58(), flag: flag.toBase58(),
    system_program: '11111111111111111111111111111111', event_authority: program.eventAuthority!,
  }
  const keys = [...new Set([payer.toBase58(), registry.toBase58(), ...Object.values(accounts), stranger.toBase58()])]
  const index = (address: string) => keys.indexOf(address)
  const specs = [
    { instruction: 'init_config', event: 'ConfigInitialized', value: configData,
      args: { admin: payer, fee_recipient: launch, attest_fee_lamports: configData.attest_fee_lamports } },
    { instruction: 'set_config', event: 'ConfigUpdated', value: { ...configData, admin: stranger },
      args: { new_admin: stranger, fee_recipient: launch, attest_fee_lamports: configData.attest_fee_lamports } },
    { instruction: 'attest', event: 'RecordAttested', value: recordData,
      args: { args: { ...recordData, source_commit: Buffer.from(recordData.source_commit, 'hex') } } },
    { instruction: 'flag', event: 'RecordFlagged', value: flagData,
      args: { input_hash: flagData.input_hash, input_location: flagData.input_location } },
  ]
  return {
    slot: 101, blockTime: 1794686400, version: 0,
    transaction: {
      signatures: [toBase58(new Uint8Array(64).fill(6))],
      message: {
        header: { numRequiredSignatures: 1, numReadonlySignedAccounts: 0, numReadonlyUnsignedAccounts: keys.length - 1 },
        accountKeys: keys,
        instructions: specs.map((spec) => ({
          programIdIndex: index(registry.toBase58()),
          accounts: idl.instructions.find((ix) => ix.name === spec.instruction)!.accounts.map((item) => index(accounts[item.name]!)),
          data: toBase58(coder.instruction.encode(spec.instruction, spec.args)),
        })),
      },
    },
    meta: {
      err: null as unknown, fee: 5000, logMessages: ['Log truncated'],
      innerInstructions: specs.map((spec, outer) => ({ index: outer, instructions: [{
        programIdIndex: index(registry.toBase58()), accounts: [index(program.eventAuthority!)], stackHeight: 2,
        data: toBase58(Buffer.concat([eventTag, eventBytes(spec.event, spec.value)])),
      }] })),
    },
  }
}

const empty = { records: [], flags: [], configs: [] }
test('stock registry IDL decodes all four CPI events and checks real PDA seeds', () => {
  const tx = decodeTransaction(rawTransaction(), [program])
  const facts = registryFacts(tx)
  assert.deepEqual(tx.issues, [])
  assert.equal(tx.logsTruncated, true)
  assert.equal(facts.records[0]!.address, record.toBase58())
  assert.equal(facts.records[0]!.sourceCommit, recordData.source_commit)
  assert.equal(facts.records[0]!.stockBuildHash, '0a'.repeat(32))
  assert.equal(facts.records[0]!.testsCount, 2147483648)
  assert.equal(facts.records[0]!.gross, '9000')
  assert.equal(facts.records[0]!.feeLamports, '1000')
  assert.equal(facts.flags[0]!.address, flag.toBase58())
  assert.equal(facts.flags[0]!.recordAddress, record.toBase58())
  assert.deepEqual(facts.configs.map((value) => [value.event, value.address, value.admin, value.attestFeeLamports]), [
    ['ConfigInitialized', pdas.config, payer.toBase58(), '1000'],
    ['ConfigUpdated', pdas.config, stranger.toBase58(), '1000'],
  ])
})

test('registry ignores log-only events and forged self-CPI authorities', () => {
  const raw = rawTransaction()
  raw.meta.innerInstructions = []
  raw.meta.logMessages = [`Program ${program.programId} invoke [1]`, `Program data: ${eventBytes('ConfigInitialized', configData).toString('base64')}`, `Program ${program.programId} success`]
  assert.deepEqual(registryFacts(decodeTransaction(raw, [program])), empty)
  const forged = rawTransaction()
  const strangerIndex = forged.transaction.message.accountKeys.indexOf(stranger.toBase58())
  for (const group of forged.meta.innerInstructions) group.instructions[0]!.accounts = [strangerIndex]
  const decoded = decodeTransaction(forged, [program])
  assert.deepEqual(registryFacts(decoded), empty)
  assert.equal(decoded.issues.filter((issue) => issue.code === 'EVENT_CPI_AUTHORITY').length, 4)
})

test('registry ignores failed transactions and unrelated invocation parents', () => {
  const failed = rawTransaction()
  failed.meta.err = { InstructionError: [1, { Custom: 6000 }] }
  assert.deepEqual(registryFacts(decodeTransaction(failed, [program])), empty)
  const forged = rawTransaction()
  const strangerIndex = forged.transaction.message.accountKeys.indexOf(stranger.toBase58())
  for (const instruction of forged.transaction.message.instructions) instruction.programIdIndex = strangerIndex
  assert.deepEqual(registryFacts(decodeTransaction(forged, [program])), empty)
})

test('registry rejects wrong config, record and flag PDAs', () => {
  const raw = rawTransaction()
  const strangerIndex = raw.transaction.message.accountKeys.indexOf(stranger.toBase58())
  for (const [outer, name] of [[0, 'config'], [1, 'config'], [2, 'record'], [3, 'flag']] as const) {
    const instruction = raw.transaction.message.instructions[outer]!
    const idlInstruction = idl.instructions.find((value) => value.name === ['init_config', 'set_config', 'attest', 'flag'][outer])!
    instruction.accounts[idlInstruction.accounts.findIndex((account) => account.name === name)] = strangerIndex
  }
  const tx = decodeTransaction(raw, [program])
  assert.deepEqual(registryFacts(tx), empty)
  assert.equal(tx.issues.filter((issue) => issue.code === 'REGISTRY_EVENT').length, 4)
})

test('registry rejects an attest whose fee went to another account than its event names', () => {
  const raw = rawTransaction()
  const strangerIndex = raw.transaction.message.accountKeys.indexOf(stranger.toBase58())
  const attest = raw.transaction.message.instructions[2]!
  const accounts = idl.instructions.find((value) => value.name === 'attest')!.accounts
  attest.accounts[accounts.findIndex((account) => account.name === 'fee_recipient')] = strangerIndex
  const tx = decodeTransaction(raw, [program])
  assert.deepEqual(registryFacts(tx).records, [])
  assert.equal(tx.issues.filter((issue) => issue.code === 'REGISTRY_EVENT').length, 1)
})
