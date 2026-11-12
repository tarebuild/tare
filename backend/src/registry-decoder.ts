import { z } from 'zod'
import { Address, Commit, Hash } from '@tare/contract'
import { PublicKey } from '@solana/web3.js'
import type { DecodedTransaction } from './engines/indexer/decoder.js'

const unsigned = z.union([z.string().regex(/^(0|[1-9][0-9]*)$/), z.number().int().nonnegative()])
  .transform(String).refine((value) => BigInt(value) <= BigInt(Number.MAX_SAFE_INTEGER), 'Must be a safe JSON integer.')
const count = unsigned.transform(Number)
const digest = z.union([Hash, z.array(z.number().int().min(0).max(255)).length(32)])
  .transform((value) => typeof value === 'string' ? value : Buffer.from(value).toString('hex'))

const recordEvent = z.object({
  program: Address,
  stock_build_hash: digest,
  tare_build_hash: digest,
  source_repository: z.url(),
  source_commit: Commit.regex(/^(?:[a-f0-9]{40}|[a-f0-9]{64})$/),
  toolchain_version: z.string().min(1),
  corpus_hash: digest,
  tests_count: count,
  generated_count: count,
  mainnet_count: count,
  gross: unsigned,
  net: unsigned,
  report_hash: digest,
  report_location: z.string().min(1),
  attester: Address,
  slot: unsigned,
  fee_recipient: Address,
  fee_lamports: unsigned,
})
const flagEvent = z.object({
  record: Address,
  input_hash: digest,
  input_location: z.string().min(1),
  flagger: Address,
  slot: unsigned,
})
const configEvent = z.object({
  admin: Address,
  fee_recipient: Address,
  attest_fee_lamports: unsigned,
})

const derive = (registry: string, seeds: Uint8Array[]) => PublicKey.findProgramAddressSync(seeds, new PublicKey(registry))[0].toBase58()
export function registryPdas(registry: string) {
  return { config: derive(registry, [Buffer.from('config')]) }
}
export function deriveRecordAddress(registry: string, value: { program: string; reportHash: string; attester: string }) {
  return derive(registry, [Buffer.from('record'), new PublicKey(value.program).toBytes(), Buffer.from(value.reportHash, 'hex'), new PublicKey(value.attester).toBytes()])
}
export function deriveFlagAddress(registry: string, value: { record: string; inputHash: string }) {
  return derive(registry, [Buffer.from('flag'), new PublicKey(value.record).toBytes(), Buffer.from(value.inputHash, 'hex')])
}
const requireMatch = (actual: string | null | undefined, expected: string) => {
  if (actual !== expected) throw new Error('Registry event account does not match its instruction or PDA.')
}

export type RegistryRecordFact = {
  address: string
  program: string
  attester: string
  sourceRepository: string
  sourceCommit: string
  toolchainVersion: string
  stockBuildHash: string
  tareBuildHash: string
  corpusHash: string
  testsCount: number
  generatedCount: number
  mainnetCount: number
  gross: string
  net: string
  reportHash: string
  reportLocation: string
  slot: string
  feeLamports: string
}
export type RegistryFlagFact = {
  address: string
  recordAddress: string
  inputHash: string
  inputLocation: string
  flagger: string
  slot: string
}
export type RegistryConfigFact = {
  address: string
  admin: string
  feeRecipient: string
  attestFeeLamports: string
  event: 'ConfigInitialized' | 'ConfigUpdated'
}
export type RegistryFacts = { records: RegistryRecordFact[]; flags: RegistryFlagFact[]; configs: RegistryConfigFact[] }

/** Stock-IDL events are accepted only from authenticated self-CPI instructions at their canonical PDAs. */
export function registryFacts(tx: DecodedTransaction): RegistryFacts {
  const facts: RegistryFacts = { records: [], flags: [], configs: [] }
  if (tx.failed) return facts
  for (const event of tx.events) {
    if (event.source !== 'cpi') continue
    const instruction = tx.instructions.find((ix) => ix.path.index === event.path.index)
    if (!instruction) continue
    try {
      const pdas = registryPdas(event.programId)
      if (event.name === 'RecordAttested') {
        const value = recordEvent.parse(event.data)
        const record = deriveRecordAddress(event.programId, { program: value.program, reportHash: value.report_hash, attester: value.attester })
        requireMatch(instruction.name, 'attest')
        requireMatch(instruction.namedAccounts.record, record)
        requireMatch(instruction.namedAccounts.attester, value.attester)
        requireMatch(instruction.namedAccounts.config, pdas.config)
        requireMatch(instruction.namedAccounts.fee_recipient, value.fee_recipient)
        requireMatch(value.slot, String(tx.slot))
        facts.records.push({
          address: record,
          program: value.program,
          attester: value.attester,
          sourceRepository: value.source_repository,
          sourceCommit: value.source_commit,
          toolchainVersion: value.toolchain_version,
          stockBuildHash: value.stock_build_hash,
          tareBuildHash: value.tare_build_hash,
          corpusHash: value.corpus_hash,
          testsCount: value.tests_count,
          generatedCount: value.generated_count,
          mainnetCount: value.mainnet_count,
          gross: value.gross,
          net: value.net,
          reportHash: value.report_hash,
          reportLocation: value.report_location,
          slot: value.slot,
          feeLamports: value.fee_lamports,
        })
      } else if (event.name === 'RecordFlagged') {
        const value = flagEvent.parse(event.data)
        const flag = deriveFlagAddress(event.programId, { record: value.record, inputHash: value.input_hash })
        requireMatch(instruction.name, 'flag')
        requireMatch(instruction.namedAccounts.flag, flag)
        requireMatch(instruction.namedAccounts.record, value.record)
        requireMatch(instruction.namedAccounts.flagger, value.flagger)
        requireMatch(value.slot, String(tx.slot))
        facts.flags.push({
          address: flag,
          recordAddress: value.record,
          inputHash: value.input_hash,
          inputLocation: value.input_location,
          flagger: value.flagger,
          slot: value.slot,
        })
      } else if (event.name === 'ConfigInitialized' || event.name === 'ConfigUpdated') {
        const value = configEvent.parse(event.data)
        requireMatch(instruction.name, event.name === 'ConfigInitialized' ? 'init_config' : 'set_config')
        requireMatch(instruction.namedAccounts.config, pdas.config)
        facts.configs.push({ address: pdas.config, admin: value.admin, feeRecipient: value.fee_recipient,
          attestFeeLamports: value.attest_fee_lamports, event: event.name })
      }
    } catch (error) {
      tx.issues.push({ code: 'REGISTRY_EVENT', programId: event.programId, path: event.path,
        message: error instanceof z.ZodError ? 'Registry event fields do not match the contract.' : 'Registry event could not be read.' })
    }
  }
  return facts
}
