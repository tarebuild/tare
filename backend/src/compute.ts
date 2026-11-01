import { createHash } from 'node:crypto'
import { normalizeTransaction, type NormalizedInstruction } from './engines/indexer/transaction.js'

export interface InstructionCompute {
  name: string | null
  discriminator: string
  calls: number
  median: number
  p90: number
  totalUnits: number
  share: number
}

type Frame = { instruction: NormalizedInstruction; units: number | null; anchor: boolean }
export type InstructionMeasurement = { discriminator: string; units: number; anchor: boolean }
const invoke = /^Program ([1-9A-HJ-NP-Za-km-z]{32,44}) invoke \[(\d+)\]$/
const consumed = /^Program ([1-9A-HJ-NP-Za-km-z]{32,44}) consumed (\d+) of (\d+) compute units$/
const completed = /^Program ([1-9A-HJ-NP-Za-km-z]{32,44}) (success|failed: .+)$/
const runtimeLine = /^Program [1-9A-HJ-NP-Za-km-z]{32,44} (?:invoke|consumed|success|failed)(?: |$)/
const anchorInstruction = /^Program log: Instruction: ([A-Za-z][A-Za-z0-9_]*)$/
const truncation = /^Log truncated$/i

function matchesAnchorDiscriminator(name: string, data: Uint8Array) {
  const snake = name.replace(/([A-Z])([A-Z][a-z])/g, '$1_$2').replace(/([a-z0-9])([A-Z])/g, '$1_$2').toLowerCase()
  return data.length >= 8 && createHash('sha256').update(`global:${snake}`).digest().subarray(0, 8).equals(data.subarray(0, 8))
}

export function measureTransaction(raw: unknown, program: string): InstructionMeasurement[] | null {
  const tx = normalizeTransaction(raw)
  if (!tx.valid || tx.failed) return null
  const truncated = tx.logs.some((line) => truncation.test(line))
  const invalidInstructions = tx.issues.filter((issue) => issue.code === 'INVALID_INSTRUCTION')
  if (!truncated && invalidInstructions.length) return null
  const firstInvalidInstruction = Math.min(...invalidInstructions.map((issue) => issue.path!.index))
  const stack: Frame[] = []
  const measured: InstructionMeasurement[] = []
  let next = 0
  const partial = () => {
    if (!truncated) return null
    // A consumed line is already the invocation's final CU total. The runtime
    // may truncate before its success line; known failed frames were popped.
    for (const frame of stack) {
      if (frame.instruction.programId === program && frame.units !== null && frame.instruction.data.length) {
        measured.push({ discriminator: Buffer.from(frame.instruction.data.subarray(0, 8)).toString('hex'), units: frame.units, anchor: frame.anchor })
      }
    }
    return measured.length ? measured : null
  }

  for (const line of tx.logs) {
    if (truncation.test(line)) return partial()
    const call = invoke.exec(line)
    if (call) {
      const depth = Number(call[2])
      if (depth !== stack.length + 1) return partial()
      // Native top-level instructions such as ComputeBudget may not produce invoke logs.
      // Never skip an unlogged target or inner instruction: attribution would be ambiguous.
      while (next < tx.instructions.length && tx.instructions[next]!.programId !== call[1]) {
        const skipped = tx.instructions[next]!
        if (skipped.programId === program || skipped.path.inner !== null || skipped.stackHeight !== 1) return partial()
        next++
      }
      const instruction = tx.instructions[next++]
      if (!instruction || instruction.path.index >= firstInvalidInstruction || instruction.stackHeight !== depth || instruction.parentIndex !== (stack.at(-1)?.instruction.path.index ?? null)) return partial()
      stack.push({ instruction, units: null, anchor: false })
      continue
    }
    const usage = consumed.exec(line)
    if (usage) {
      const frame = stack.at(-1)
      const units = Number(usage[2])
      const budget = Number(usage[3])
      if (!frame || frame.instruction.programId !== usage[1] || frame.units !== null || !Number.isSafeInteger(units) || !Number.isSafeInteger(budget)) return partial()
      frame.units = units
      continue
    }
    const end = completed.exec(line)
    if (end) {
      const frame = stack.at(-1)
      if (!frame || frame.instruction.programId !== end[1]) return partial()
      stack.pop()
      if (end[2] === 'success' && frame.instruction.programId === program) {
        if (frame.units === null || frame.instruction.data.length === 0) return partial()
        const discriminator = Buffer.from(frame.instruction.data.subarray(0, 8)).toString('hex')
        measured.push({ discriminator, units: frame.units, anchor: frame.anchor })
      }
      continue
    }
    const marker = anchorInstruction.exec(line)
    if (marker) {
      const frame = stack.at(-1)
      if (frame?.instruction.programId === program && matchesAnchorDiscriminator(marker[1]!, frame.instruction.data)) frame.anchor = true
    }
    if (runtimeLine.test(line)) return partial()
  }
  if (stack.length || tx.instructions.slice(next).some((instruction) => instruction.programId === program || instruction.path.inner !== null || instruction.stackHeight !== 1)) return null
  return measured.length ? measured : null
}

/**
 * Counts attributable invocations in successful transactions. A truncated trace keeps completed
 * calls and open calls with a consumed line; untruncated traces must match the full call tree.
 * Runtime consumed CU is inclusive of child CPIs; a recursive invocation also gets its own row.
 * Median and p90 use nearest rank: sorted[ceil(p * count) - 1], keeping CU values integral.
 * `transactions` counts transactions with at least one accepted target-program measurement.
 * `anchor` records a target-scoped Anchor marker whose name hashes to that instruction's
 * discriminator. Display names still come only from the supplied IDL map.
 */
export function computeByInstruction(rawTransactions: readonly unknown[], program: string, names: ReadonlyMap<string, string>): { instructions: InstructionCompute[]; transactions: number; anchor: boolean } {
  const accepted = rawTransactions.flatMap((raw) => {
    const values = measureTransaction(raw, program)
    return values ? [values] : []
  })
  return groupMeasurements(accepted, names)
}

export function groupMeasurements(accepted: readonly (readonly InstructionMeasurement[])[], names: ReadonlyMap<string, string>): { instructions: InstructionCompute[]; transactions: number; anchor: boolean } {
  const groups = new Map<string, number[]>()
  let anchor = false
  for (const values of accepted) {
    for (const value of values) {
      anchor ||= value.anchor
      const units = groups.get(value.discriminator) ?? []
      units.push(value.units)
      groups.set(value.discriminator, units)
    }
  }
  const rows = [...groups].map(([discriminator, units]) => {
    units.sort((a, b) => a - b)
    return {
      name: names.get(discriminator) ?? null,
      discriminator,
      calls: units.length,
      median: units[Math.ceil(0.5 * units.length) - 1]!,
      p90: units[Math.ceil(0.9 * units.length) - 1]!,
      totalUnits: units.reduce((sum, value) => sum + value, 0),
    }
  })
  const total = rows.reduce((sum, row) => sum + row.totalUnits, 0)
  return {
    instructions: rows.map((row) => ({ ...row, share: total === 0 ? 0 : row.totalUnits / total })).sort((a, b) => b.totalUnits - a.totalUnits || a.discriminator.localeCompare(b.discriminator)),
    transactions: accepted.length,
    anchor,
  }
}
