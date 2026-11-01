import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import { toBase58 } from './engines/core/encoding.js'
import { normalizeTransaction } from './engines/indexer/transaction.js'
import { computeByInstruction } from './compute.js'

const key = (last: number) => toBase58(Uint8Array.from({ length: 32 }, (_, index) => index === 31 ? last : 7))
const payer = key(1), program = key(2), other = key(3), native = key(4)
const a = '0102030405060708', b = '1112131415161718'
const names = new Map([[a, 'deposit'], [b, 'withdraw']])
const instruction = (programIdIndex: number, hex: string, stackHeight = 1) => ({ programIdIndex, accounts: [0], data: toBase58(Buffer.from(hex, 'hex')), stackHeight })
function raw(instructions: unknown[], logs: string[], innerInstructions: unknown[] = [], err: unknown = null) {
  return {
    slot: 100, blockTime: 1730000000, version: 0,
    transaction: {
      signatures: ['fixture-signature'],
      message: { header: { numRequiredSignatures: 1, numReadonlySignedAccounts: 0, numReadonlyUnsignedAccounts: 3 }, accountKeys: [payer, program, other, native], instructions },
    },
    meta: { err, fee: 5000, loadedAddresses: { writable: [], readonly: [] }, innerInstructions, logMessages: logs, preTokenBalances: [], postTokenBalances: [] },
  }
}
const call = (id: string, depth: number) => `Program ${id} invoke [${depth}]`
const used = (id: string, units: number) => `Program ${id} consumed ${units} of 200000 compute units`
const success = (id: string) => `Program ${id} success`

test('groups top-level calls with integer nearest-rank quantiles and skips silent native instructions', () => {
  const transactions = [100, 300, 200, 900].map((units) => raw([instruction(3, '01'), instruction(1, a + '99')], [call(program, 1), 'Program log: deposit', used(program, units), success(program)]))
  const result = computeByInstruction(transactions, program, names)
  assert.equal(result.transactions, 4)
  assert.deepEqual(result.instructions, [{ name: 'deposit', discriminator: a, calls: 4, median: 200, p90: 900, totalUnits: 1500, share: 1 }])
})

test('attributes recursive same-program calls to their own discriminators', () => {
  const tx = raw([instruction(1, a)], [call(program, 1), call(program, 2), used(program, 400), success(program), used(program, 1000), success(program)], [{ index: 0, instructions: [instruction(1, b, 2)] }])
  const result = computeByInstruction([tx], program, names)
  assert.equal(result.transactions, 1)
  assert.deepEqual(result.instructions.map(({ discriminator, totalUnits }) => ({ discriminator, totalUnits })), [{ discriminator: a, totalUnits: 1000 }, { discriminator: b, totalUnits: 400 }])
  assert.equal(result.instructions[0]!.share, 1000 / 1400)
})

test('keeps outer inclusive compute without attributing another program CPI to it', () => {
  const tx = raw([instruction(1, a)], [call(program, 1), call(other, 2), used(other, 200), success(other), used(program, 700), success(program)], [{ index: 0, instructions: [instruction(2, b, 2)] }])
  const result = computeByInstruction([tx], program, names)
  assert.equal(result.transactions, 1)
  assert.equal(result.instructions.length, 1)
  assert.equal(result.instructions[0]!.totalUnits, 700)
})

test('omits a caught failed inner invocation while measuring the successful outer call', () => {
  const tx = raw([instruction(1, a)], [call(program, 1), call(program, 2), used(program, 50), `Program ${program} failed: custom program error: 0x1770`, 'Program log: failure caught', used(program, 500), success(program)], [{ index: 0, instructions: [instruction(1, b, 2)] }])
  const result = computeByInstruction([tx], program, names)
  assert.equal(result.transactions, 1)
  assert.deepEqual(result.instructions.map((row) => row.discriminator), [a])
  assert.equal(result.instructions[0]!.totalUnits, 500)
})

test('keeps complete measurements before truncation but skips failed transactions wholesale', () => {
  const logs = [call(program, 1), used(program, 100), success(program)]
  const truncated = raw([instruction(1, a)], [...logs, 'Log truncated'])
  const failed = raw([instruction(1, a)], logs, [], { InstructionError: [0, 'Custom'] })
  assert.deepEqual(computeByInstruction([truncated, failed], program, names), {
    instructions: [{ name: 'deposit', discriminator: a, calls: 1, median: 100, p90: 100, totalUnits: 100, share: 1 }], transactions: 1, anchor: false,
  })
})

test('keeps a consumed invocation without its success line when the trace is truncated', () => {
  const truncated = raw([instruction(1, a), instruction(1, b)], [call(program, 1), used(program, 250), 'Log truncated'])
  const beforeConsumption = raw([instruction(1, a)], [call(program, 1), 'Log truncated'])
  const result = computeByInstruction([truncated, beforeConsumption], program, names)
  assert.equal(result.transactions, 1)
  assert.equal(result.instructions[0]!.totalUnits, 250)
})

test('truncation keeps recursive calls once each and excludes a known failed inner invocation', () => {
  const recursive = raw([instruction(1, a)], [
    call(program, 1), call(program, 2), used(program, 100), success(program), used(program, 400), 'Log truncated',
  ], [{ index: 0, instructions: [instruction(1, b, 2)] }])
  const caughtFailure = raw([instruction(1, a)], [
    call(program, 1), call(program, 2), used(program, 50), `Program ${program} failed: custom program error: 0x1770`,
    used(program, 500), 'Log truncated',
  ], [{ index: 0, instructions: [instruction(1, b, 2)] }])
  const result = computeByInstruction([recursive, caughtFailure], program, names)
  assert.equal(result.transactions, 2)
  assert.deepEqual(result.instructions.map(({ discriminator, calls, totalUnits }) => ({ discriminator, calls, totalUnits })), [
    { discriminator: a, calls: 2, totalUnits: 900 }, { discriminator: b, calls: 1, totalUnits: 100 },
  ])
})

test('a truncated mismatched tail preserves an attributed prefix without guessing later calls', () => {
  const value = raw([instruction(1, a), instruction(1, b)], [
    call(program, 1), used(program, 120), success(program), call(other, 1), used(other, 300), 'Log truncated',
  ])
  const invalidTail = raw([instruction(1, a), instruction(999, b)], [
    call(program, 1), used(program, 130), success(program), call(program, 1), used(program, 900), 'Log truncated',
  ])
  const invalidPrefix = raw([instruction(999, a), instruction(1, b)], [call(program, 1), used(program, 900), 'Log truncated'])
  const result = computeByInstruction([value, invalidTail, invalidPrefix], program, names)
  assert.equal(result.transactions, 2)
  assert.deepEqual(result.instructions.map(({ discriminator, calls, totalUnits }) => ({ discriminator, calls, totalUnits })), [
    { discriminator: a, calls: 2, totalUnits: 250 },
  ])
})

test('skips malformed or unmatched execution trees instead of guessing attribution', () => {
  const noConsumption = raw([instruction(1, a)], [call(program, 1), success(program)])
  const unmatched = raw([instruction(1, a)], [call(other, 1), used(other, 30), success(other)])
  const badDepth = raw([instruction(1, a)], [call(program, 2), used(program, 30), success(program)])
  const unfinished = raw([instruction(1, a)], [call(program, 1), used(program, 30)])
  const partial = raw([instruction(1, a), instruction(1, b)], [call(program, 1), used(program, 20), success(program), call(other, 1), used(other, 30), success(other)])
  const skippedTarget = raw([instruction(1, a), instruction(2, b)], [call(other, 1), used(other, 30), success(other)])
  assert.deepEqual(computeByInstruction([null, noConsumption, unmatched, badDepth, unfinished, partial, skippedTarget], program, names), { instructions: [], transactions: 0, anchor: false })
})

test('skips missing metadata or logs without discarding a later measurable transaction', () => {
  const valid = raw([instruction(1, a)], [call(program, 1), used(program, 120), success(program)])
  const { meta: _meta, ...missingMeta } = valid
  const nullMeta = { ...valid, meta: null }
  const { logMessages: _logs, ...metaWithoutLogs } = valid.meta
  const missingLogs = { ...valid, meta: metaWithoutLogs }
  const nullLogs = { ...valid, meta: { ...valid.meta, logMessages: null } }
  const result = computeByInstruction([missingMeta, nullMeta, missingLogs, nullLogs, valid], program, names)
  assert.equal(result.transactions, 1)
  assert.deepEqual(result.instructions, [{ name: 'deposit', discriminator: a, calls: 1, median: 120, p90: 120, totalUnits: 120, share: 1 }])
})

test('accepts absent optional metadata lists when the complete target trace is present', () => {
  const value = raw([instruction(1, a)], [call(program, 1), used(program, 220), success(program)])
  const { loadedAddresses: _loaded, innerInstructions: _inner, preTokenBalances: _pre, postTokenBalances: _post, ...minimal } = value.meta
  const result = computeByInstruction([{ ...value, meta: minimal }], program, names)
  assert.equal(result.transactions, 1)
  assert.equal(result.instructions[0]!.totalUnits, 220)
})

test('resolves a target and inner accounts from writable and readonly lookup-table addresses', () => {
  const writableAccount = key(5), readonlyAccount = key(6)
  for (const targetList of ['writable', 'readonly'] as const) {
    const loadedAddresses = targetList === 'writable'
      ? { writable: [writableAccount, program], readonly: [readonlyAccount] }
      : { writable: [writableAccount], readonly: [readonlyAccount, program] }
    const targetIndex = targetList === 'writable' ? 3 : 4
    const readonlyIndex = targetList === 'writable' ? 4 : 3
    const inner = { ...instruction(targetIndex, a, 2), accounts: [0, 2, readonlyIndex] }
    const base = raw([instruction(1, b)], [
      call(other, 1), call(program, 2), used(program, 330), success(program), used(other, 500), success(other),
    ], [{ index: 0, instructions: [inner] }])
    const value = { ...base, meta: { ...base.meta, loadedAddresses } }
    value.transaction.message.accountKeys = [payer, other]
    value.transaction.message.header.numReadonlyUnsignedAccounts = 1
    const normalized = normalizeTransaction(value)
    assert.equal(normalized.valid, true)
    assert.deepEqual(normalized.instructions[1]!.accounts, [payer, writableAccount, readonlyAccount])
    assert.equal(normalized.instructions[1]!.programId, program)
    const result = computeByInstruction([value], program, names)
    assert.equal(result.transactions, 1)
    assert.deepEqual(result.instructions, [{ name: 'deposit', discriminator: a, calls: 1, median: 330, p90: 330, totalUnits: 330, share: 1 }])
  }
})

test('detects a scoped Anchor instruction marker without inventing IDL names', () => {
  const discriminator = createHash('sha256').update('global:initialize_pool').digest().subarray(0, 8).toString('hex')
  const value = raw([instruction(1, discriminator)], [call(program, 1), 'Program log: Instruction: InitializePool', used(program, 410), success(program)])
  const result = computeByInstruction([value], program, new Map())
  assert.equal(result.anchor, true)
  assert.equal(result.instructions[0]!.name, null)
  assert.equal(result.instructions[0]!.discriminator, discriminator)
})

test('rejects an Anchor marker for another discriminator or another program frame', () => {
  const discriminator = createHash('sha256').update('global:initialize_pool').digest().subarray(0, 8).toString('hex')
  const wrongData = raw([instruction(1, a)], [call(program, 1), 'Program log: Instruction: InitializePool', used(program, 410), success(program)])
  const nestedMarker = raw([instruction(1, discriminator)], [
    call(program, 1), call(other, 2), 'Program log: Instruction: InitializePool', used(other, 100), success(other), used(program, 410), success(program),
  ], [{ index: 0, instructions: [instruction(2, discriminator, 2)] }])
  assert.equal(computeByInstruction([wrongData, nestedMarker], program, new Map()).anchor, false)
})

test('groups the captured mainnet Whirlpools CPI through v0 lookup-table accounts', () => {
  const capture = JSON.parse(readFileSync(new URL('./fixtures/mainnet/whirlpools-transaction.json', import.meta.url), 'utf8'))
  const whirlpools = 'whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc'
  assert.equal(capture.response.result.transaction.message.accountKeys.includes(whirlpools), false)
  assert.equal(capture.response.result.meta.loadedAddresses.readonly.includes(whirlpools), true)
  // The captured runtime reports 35,974 CU for SwapV2, including its two token CPIs.
  assert.deepEqual(computeByInstruction([capture.response.result], whirlpools, new Map()), {
    transactions: 1,
    anchor: true,
    instructions: [{ name: null, discriminator: '2b04ed0b1ac91e62', calls: 1, median: 35974, p90: 35974, totalUnits: 35974, share: 1 }],
  })
})

test('captured Drift address-only transactions do not count as Drift invocations', () => {
  const capture = JSON.parse(readFileSync(new URL('./fixtures/mainnet/drift-address-only-transactions.json', import.meta.url), 'utf8'))
  for (const raw of capture.transactions) {
    const tx = normalizeTransaction(raw)
    assert.equal(tx.valid, true)
    assert(tx.keys.some((key) => key.address === capture.program))
    assert(tx.instructions.every((instruction) => instruction.programId !== capture.program))
  }
  assert.deepEqual(computeByInstruction(capture.transactions, capture.program, new Map()), { instructions: [], transactions: 0, anchor: false })
})
