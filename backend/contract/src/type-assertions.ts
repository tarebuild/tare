import type { z } from 'zod'
import type { AccountSnapshot, BenchResults, Build, FlagInput, ProofReport, RecordSummary, RecordDetail, Weighing, WeighingTotals, UploadProofReport } from './schemas.js'
import type { createClient } from './client.js'

type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends (<T>() => T extends B ? 1 : 2) ? true : false
type Assert<T extends true> = T
type MeasuredBench = Extract<BenchResults['programs'][number], { status: 'measured' }>
type ReachedInstruction = Extract<Weighing['instructions'][number], { gross: number }>
type ReadyRecord = Extract<RecordDetail, { status: 'ready' }>
type Client = ReturnType<typeof createClient>
// Compile-time contract regressions: public arithmetic never becomes number | string.
export type PublicNumberAssertions = [
  Assert<Equal<ReachedInstruction['gross'], number>>,
  Assert<Equal<ReachedInstruction['net'], number>>,
  Assert<Equal<ReachedInstruction['tare'], number>>,
  Assert<Equal<ReachedInstruction['inputs'], number>>,
  Assert<Equal<WeighingTotals['gross'] | WeighingTotals['net'] | WeighingTotals['tare'] | WeighingTotals['inputs'], number>>,
  Assert<Equal<ReadyRecord['report']['totals']['gross'], number>>,
  Assert<Equal<ReadyRecord['report']['builds']['stock']['sizeBytes'], number>>,
  Assert<Equal<ReadyRecord['report']['builds']['tare']['rentLamports'], number>>,
  Assert<Equal<ReadyRecord['report']['proof']['corpus']['counts']['tests'], number>>,
  Assert<Equal<RecordSummary['slot'], number>>,
  Assert<Equal<MeasuredBench['weighing']['totals']['gross'], number>>,
  Assert<Equal<MeasuredBench['weighing']['totals']['tare'], number>>,
  Assert<Equal<Weighing['totals']['net'], number>>,
  Assert<Equal<WeighingTotals['gross'], number>>,
  Assert<Equal<Build['sizeBytes'], number>>,
  Assert<Equal<Build['rentLamports'], number>>,
  Assert<Equal<RecordSummary['gross'] | RecordSummary['net'] | RecordSummary['tare'], number>>,
  Assert<Equal<ProofReport['proof']['comparisons']['inputs'], number>>,
  Assert<Equal<ProofReport['proof']['corpus']['counts']['tests'], number>>,
  Assert<Equal<FlagInput['divergence']['input']['slot'], number | null>>,
  Assert<Equal<AccountSnapshot['lamports'], string>>,
  Assert<Equal<AccountSnapshot['rentEpoch'], string | undefined>>,
  Assert<Equal<Awaited<ReturnType<Client['report']>>['totals']['gross'], number>>,
  Assert<Equal<z.output<typeof UploadProofReport>['totals']['gross'], number | string>>,
]
