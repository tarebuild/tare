import { z } from 'zod'
import { Address, Base64, Signature, Timestamp } from './primitives.js'
import { defineErrors } from './errors.js'

export const CONTRACT_VERSION = '0.9.0'
export const DOCUMENT_VERSION = '1'
export const Count = z.int().nonnegative()
/** Uploaded u64 metadata preserves safe numbers and exact canonical decimal strings. */
const uploadedU64Decimal = z.string().max(20).regex(/^(?:0|[1-9][0-9]{0,18}|1[0-7][0-9]{18}|18[0-3][0-9]{17}|184[0-3][0-9]{16}|1844[0-5][0-9]{15}|18446[0-6][0-9]{14}|184467[0-3][0-9]{13}|1844674[0-3][0-9]{12}|184467440[0-6][0-9]{10}|1844674407[0-2][0-9]{9}|18446744073[0-6][0-9]{8}|1844674407370[0-8][0-9]{6}|18446744073709[0-4][0-9]{5}|184467440737095[0-4][0-9]{4}|1844674407370955[0-0][0-9]{3}|18446744073709551[0-5][0-9]{2}|184467440737095516[0-0][0-9]{1}|1844674407370955161[0-4][0-9]{0}|18446744073709551615)$/)
export const UploadedU64 = z.union([Count, uploadedU64Decimal])
const uploadedPositiveU64 = z.union([Count.positive(), uploadedU64Decimal.regex(/^[1-9][0-9]*$/)])
export const Delta = z.int()
/** Signed uploaded compute savings preserve exact i64 decimal strings. */
export const UploadedI64 = z.union([Delta, z.string().max(20).regex(/^(?:0|(?:[1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-6][0-9]{0}|9223372036854775807)|-(?:[1-9][0-9]{0,17}|[1-8][0-9]{18}|9[0-1][0-9]{17}|92[0-1][0-9]{16}|922[0-2][0-9]{15}|9223[0-2][0-9]{14}|92233[0-6][0-9]{13}|922337[0-1][0-9]{12}|92233720[0-2][0-9]{10}|922337203[0-5][0-9]{9}|9223372036[0-7][0-9]{8}|92233720368[0-4][0-9]{7}|922337203685[0-3][0-9]{6}|9223372036854[0-6][0-9]{5}|92233720368547[0-6][0-9]{4}|922337203685477[0-4][0-9]{3}|9223372036854775[0-7][0-9]{2}|922337203685477580[0-7][0-9]{0}|9223372036854775808))$/)])
export const Hash = z.string().regex(/^[a-f0-9]{64}$/)
export const Commit = z.string().regex(/^[a-f0-9]{40,64}$/)
export const Discriminator = z.string().regex(/^(?:[a-f0-9]{2}){1,32}$/)
export const errors = defineErrors({
  RECORD_NOT_FOUND: 404,
  REPORT_UNAVAILABLE: 503,
  FLAG_INPUT_UNAVAILABLE: 503,
  RPC_UNAVAILABLE: 502,
  PINNING_UNAVAILABLE: 502,
  INVALID_REPORT: 422,
  INVALID_FLAG_INPUT: 422,
})
export const ErrorCode = errors.ErrorCode
export const ApiError = errors.ApiError
export type ErrorCode = z.output<typeof ErrorCode>
export type ApiError = z.output<typeof ApiError>

/** Public arithmetic accepts exact decimal input only when it fits a safe JavaScript number. */
const readCount = UploadedU64.transform(Number).pipe(Count)
const readDelta = UploadedI64.transform(Number).pipe(Delta)
const readPositiveCount = uploadedPositiveU64.transform(Number).pipe(Count.positive())
/** Account balances and epochs are exact decimal strings on every public read. */
const readAmount = UploadedU64.transform(String).pipe(uploadedU64Decimal)
export const ProgramIdentity = z.strictObject({ name: z.string().min(1).nullable(), address: Address.nullable() })
export const Source = z.strictObject({ repository: z.url().nullable(), commit: Commit.nullable() })
export const CorpusCounts = z.strictObject({ tests: readCount, generated: readCount, mainnet: readCount })
export const Corpus = z.strictObject({ hash: Hash, counts: CorpusCounts })
export const UploadedCorpusCounts = z.strictObject({ tests: UploadedU64, generated: UploadedU64, mainnet: UploadedU64 })
export const UploadedCorpus = z.strictObject({ hash: Hash, counts: UploadedCorpusCounts })
/** Sum one median gross/net/tare per reached instruction; inputs is the successful measured call count. */
export const Weights = z.strictObject({ gross: readCount, net: readCount, tare: readDelta, inputs: readCount })
export const UploadedWeights = z.strictObject({ gross: UploadedU64, net: UploadedU64, tare: UploadedI64, inputs: UploadedU64 })
const instructionIdentity = z.strictObject({ name: z.string().min(1).nullable(), discriminator: Discriminator })
/** Per-call median CU. An instruction with no successful measured calls has no observed weight. */
export const UploadInstructionWeight = z.union([
  instructionIdentity.extend({ inputs: z.union([z.literal(0), z.literal('0')]), gross: z.null(), net: z.null(), tare: z.null() }),
  instructionIdentity.extend({ inputs: uploadedPositiveU64, gross: UploadedU64, net: UploadedU64, tare: UploadedI64 }),
])
export const UploadBuild = z.strictObject({ hash: Hash, sizeBytes: UploadedU64, rentLamports: UploadedU64 })
export const UploadAccountSnapshot = z.strictObject({
  address: Address, owner: Address, lamports: UploadedU64, executable: z.boolean(), dataBase64: Base64,
  rentEpoch: UploadedU64.optional(),
})
export const CorpusKind = z.enum(['tests', 'generated', 'mainnet'])
export const UploadReplayInput = z.strictObject({
  kind: CorpusKind,
  label: z.string().min(1),
  transactionBase64: Base64,
  accounts: z.array(UploadAccountSnapshot),
  slot: UploadedU64.nullable(),
})
export const ReturnData = z.strictObject({ program: Address, dataBase64: Base64 })
export const UploadAccountMetadata = z.strictObject({ owner: Address, executable: z.boolean(), rentEpoch: UploadedU64 })
export const UploadDifference = z.discriminatedUnion('kind', [
  z.strictObject({ kind: z.literal('account_bytes'), account: Address, gross: Base64.nullable(), net: Base64.nullable() }),
  z.strictObject({ kind: z.literal('lamports'), account: Address, gross: UploadedU64.nullable(), net: UploadedU64.nullable() }),
  z.strictObject({ kind: z.literal('account_metadata'), account: Address, gross: UploadAccountMetadata.nullable(), net: UploadAccountMetadata.nullable() }),
  z.strictObject({ kind: z.literal('logs'), gross: z.array(z.string()), net: z.array(z.string()) }),
  z.strictObject({ kind: z.literal('return_data'), gross: ReturnData.nullable(), net: ReturnData.nullable() }),
  z.strictObject({ kind: z.literal('error'), gross: z.json(), net: z.json() }),
])
export const UploadDivergence = z.strictObject({ input: UploadReplayInput, instructionIndex: UploadedU64, differences: z.array(UploadDifference).min(1) })
export const UploadComparisons = z.strictObject({ inputs: UploadedU64, accountBytes: UploadedU64, lamports: UploadedU64, logs: UploadedU64, returnData: UploadedU64, errors: UploadedU64 })
export const UploadProofSummary = z.strictObject({ status: z.enum(['matched', 'diverged']), corpus: UploadedCorpus, comparisons: UploadComparisons })
export const UploadProof = z.discriminatedUnion('status', [
  UploadProofSummary.extend({ status: z.literal('matched'), firstDivergence: z.null() }),
  UploadProofSummary.extend({ status: z.literal('diverged'), firstDivergence: UploadDivergence }),
])
/** Optimizer statistics from tare-opt; later pass metadata is preserved verbatim. */
export const UploadOptimization = z.looseObject({
  inputBytes: UploadedU64,
  outputBytes: UploadedU64,
  inputHash: Hash,
  outputHash: Hash,
  textInstructions: UploadedU64,
  branchTargetsThreaded: UploadedU64,
  callTargetsThreaded: UploadedU64,
  stackLoadsForwarded: UploadedU64,
  arithmeticStrengthReductions: UploadedU64,
  constantBranchesSimplified: UploadedU64,
  identityRunsSkipped: UploadedU64,
  identityInstructionsBypassed: UploadedU64,
  unreachableInstructionsNeutralized: UploadedU64,
  unreachableFunctionsNeutralized: UploadedU64,
  indirectCalls: UploadedU64,
  relocationSitesPreserved: UploadedU64,
  rootPcs: z.array(UploadedU64),
  modifiedPcs: z.array(UploadedU64),
  textSlotsRemoved: UploadedU64.optional(),
  textBytesRemoved: UploadedU64.optional(),
  relocationsRemoved: UploadedU64.optional(),
  nonAllocatedBytesRemoved: UploadedU64.optional(),
  identicalFunctionsMerged: UploadedU64.optional(),
  pcMap: z.array(z.tuple([UploadedU64, UploadedU64, UploadedU64])).optional(),
})
/** Whether replay reproduced the captured validator outcomes, distinct from build equivalence. */
export const UploadValidatorReproduction = z.strictObject({
  inputs: UploadedU64,
  matched: UploadedU64,
  failed: UploadedU64,
  clockExactInputs: UploadedU64,
  firstDifference: z.json(),
})
export const UploadRecordedProvenance = z.looseObject({
  generatedCorpusHash: Hash,
  recordedCorpusHash: Hash,
  capturedStockHash: Hash,
  replayStockHash: Hash,
  sameStockBinary: z.boolean(),
  validatorReproduction: UploadValidatorReproduction,
})
/** Recorded corpus provenance emitted by the Rust writer, one entry per replayed corpus. */
export const UploadRecordedCorpus = z.array(UploadRecordedProvenance)
/** SHA-256 covers the decompressed UTF-8 JSON bytes containing the exact replay bundle. */
export const Replay = z.strictObject({
  encoding: z.literal('gzip+base64'),
  sha256: Hash,
  data: Base64,
})
export const BuildEnvironment = z.discriminatedUnion('kind', [
  z.strictObject({ kind: z.literal('host'), targetTriple: z.string().min(1) }),
  z.strictObject({ kind: z.literal('image'), image: z.string().min(1) }),
])
export const AccountAbi = z.strictObject({ mode: z.enum(['raw', 'sdk']), selection: z.string().min(1) })
export const UploadWeighing = z.strictObject({
  schemaVersion: z.literal(DOCUMENT_VERSION),
  program: ProgramIdentity,
  toolchainVersion: z.string().min(1),
  anchorVersion: z.string().min(1),
  source: Source,
  builds: z.strictObject({ stock: UploadBuild, tare: UploadBuild }),
  instructions: z.array(UploadInstructionWeight),
  totals: UploadedWeights,
  proof: UploadProof.nullable(),
  optimization: UploadOptimization.nullable().optional(),
  recordedCorpus: UploadRecordedCorpus.nullable().optional(),
  replay: Replay.optional(),
  buildEnvironment: BuildEnvironment.optional(),
  accountAbi: AccountAbi.optional(),
})
export const UploadProofReport = UploadWeighing.extend({ proof: UploadProof })
export const WeighingTotals = Weights
export const InstructionWeight = z.union([
  instructionIdentity.extend({ inputs: z.union([z.literal(0), z.literal('0')]).transform(() => 0 as const).pipe(z.literal(0)), gross: z.null(), net: z.null(), tare: z.null() }),
  instructionIdentity.extend({ inputs: readPositiveCount, gross: readCount, net: readCount, tare: readDelta }),
])
export const Build = UploadBuild.extend({ sizeBytes: readCount, rentLamports: readCount })
export const AccountSnapshot = UploadAccountSnapshot.extend({ lamports: readAmount, rentEpoch: readAmount.optional() })
export const ReplayInput = UploadReplayInput.extend({ accounts: z.array(AccountSnapshot), slot: readCount.nullable() })
export const AccountMetadata = UploadAccountMetadata.extend({ rentEpoch: readAmount })
export const Difference = z.discriminatedUnion('kind', [
  z.strictObject({ kind: z.literal('account_bytes'), account: Address, gross: Base64.nullable(), net: Base64.nullable() }),
  z.strictObject({ kind: z.literal('lamports'), account: Address, gross: readAmount.nullable(), net: readAmount.nullable() }),
  z.strictObject({ kind: z.literal('account_metadata'), account: Address, gross: AccountMetadata.nullable(), net: AccountMetadata.nullable() }),
  z.strictObject({ kind: z.literal('logs'), gross: z.array(z.string()), net: z.array(z.string()) }),
  z.strictObject({ kind: z.literal('return_data'), gross: ReturnData.nullable(), net: ReturnData.nullable() }),
  z.strictObject({ kind: z.literal('error'), gross: z.json(), net: z.json() }),
])
export const Divergence = UploadDivergence.extend({ input: ReplayInput, instructionIndex: readCount, differences: z.array(Difference).min(1) })
export const Comparisons = z.strictObject({ inputs: readCount, accountBytes: readCount, lamports: readCount, logs: readCount, returnData: readCount, errors: readCount })
const readCorpusCounts = z.strictObject({ tests: readCount, generated: readCount, mainnet: readCount })
const readCorpus = z.strictObject({ hash: Hash, counts: readCorpusCounts })
export const ProofSummary = UploadProofSummary.extend({ corpus: readCorpus, comparisons: Comparisons })
export const Proof = z.discriminatedUnion('status', [
  ProofSummary.extend({ status: z.literal('matched'), firstDivergence: z.null() }),
  ProofSummary.extend({ status: z.literal('diverged'), firstDivergence: Divergence }),
])
export const Optimization = UploadOptimization.extend({
  inputBytes: readCount, outputBytes: readCount, textInstructions: readCount,
  branchTargetsThreaded: readCount, callTargetsThreaded: readCount, stackLoadsForwarded: readCount,
  arithmeticStrengthReductions: readCount, constantBranchesSimplified: readCount,
  identityRunsSkipped: readCount, identityInstructionsBypassed: readCount,
  unreachableInstructionsNeutralized: readCount, unreachableFunctionsNeutralized: readCount,
  indirectCalls: readCount, relocationSitesPreserved: readCount,
  rootPcs: z.array(readCount), modifiedPcs: z.array(readCount),
  textSlotsRemoved: readCount.optional(), textBytesRemoved: readCount.optional(),
  relocationsRemoved: readCount.optional(), nonAllocatedBytesRemoved: readCount.optional(),
  identicalFunctionsMerged: readCount.optional(), pcMap: z.array(z.tuple([readCount, readCount, readCount])).optional(),
})
export const ValidatorReproduction = UploadValidatorReproduction.extend({ inputs: readCount, matched: readCount, failed: readCount, clockExactInputs: readCount })
export const RecordedProvenance = UploadRecordedProvenance.extend({ validatorReproduction: ValidatorReproduction })
export const RecordedCorpus = z.array(RecordedProvenance)
export const Weighing = UploadWeighing.extend({
  builds: z.strictObject({ stock: Build, tare: Build }), instructions: z.array(InstructionWeight),
  totals: WeighingTotals, proof: Proof.nullable(),
  optimization: Optimization.nullable().optional(), recordedCorpus: RecordedCorpus.nullable().optional(),
})
export const ProofReport = Weighing.extend({ proof: Proof })

export const TestSuite = z.strictObject({ ran: readCount, passed: readCount, failed: readCount })
const corpusProgram = z.strictObject({
  id: z.string().min(1), name: z.string().min(1), source: Source,
  anchorVersion: z.string().min(1), kind: z.enum(['anchor_test', 'anchor_example', 'production']),
})
export const BenchProgram = z.discriminatedUnion('status', [
  corpusProgram.extend({ status: z.literal('measured'), weighing: Weighing, tests: TestSuite, proof: ProofSummary, record: Address.nullable(), recordCluster: z.string().min(1).max(64).regex(/^[A-Za-z0-9._-]+$/).optional(), recordUrl: z.url().optional() }),
  corpusProgram.extend({ status: z.literal('not_built'), reason: z.string().min(1), construct: z.string().min(1).nullable() }),
])
export const SizeTotals = z.strictObject({ gross: readCount, net: readCount, tare: readDelta })
export const BenchTotals = z.strictObject({
  programs: readCount, measured: readCount, notBuilt: readCount, instructions: readCount,
  weights: Weights, binarySize: SizeTotals, rentLamports: SizeTotals, corpus: CorpusCounts,
})
export const BenchResults = z.strictObject({
  schemaVersion: z.literal(DOCUMENT_VERSION), fixture: z.boolean(), toolchainVersion: z.string().nullable(),
  generatedAt: Timestamp.nullable(), programs: z.array(BenchProgram), totals: BenchTotals,
})
export const CONSTRUCTS = [
  '#[program]', '#[derive(Accounts)]', 'account_constraints', 'Account', 'Signer', 'Program', 'SystemAccount',
  'UncheckedAccount', 'AccountLoader', 'InterfaceAccount', 'Context', 'CpiContext', '#[account]', '#[event]',
  'emit!', '#[error_code]', 'require!', 'require_family', 'init', 'init_if_needed', 'realloc', 'close', 'seeds', 'bumps', 'remaining_accounts',
] as const
export const Compat = z.strictObject({
  schemaVersion: z.literal(DOCUMENT_VERSION), fixture: z.boolean(), toolchainVersion: z.string().nullable(), generatedAt: Timestamp.nullable(),
  anchorVersions: z.array(z.string().min(1)),
  constructs: z.array(z.strictObject({
    construct: z.string().min(1),
    versions: z.array(z.strictObject({ anchorVersion: z.string().min(1), supported: z.boolean(), programs: z.array(z.string().min(1)), note: z.string().nullable() })),
  })),
})
export const PublicConfig = z.strictObject({ siteUrl: z.url().nullable(), recordUrlTemplate: z.string().nullable(), installLine: z.string().nullable() })
export const Health = z.strictObject({ status: z.enum(['fixture', 'waiting', 'ready']) })
export const RegistryTotals = z.strictObject({
  records: readCount, programs: readCount, flags: readCount, gross: readCount, net: readCount, tare: readDelta, corpus: readCount,
  feesAccruedLamports: readCount.describe('Attest fees paid to the launch wallet, summed over indexed records.'),
})
export const RecordSummary = z.strictObject({
  address: Address, program: ProgramIdentity, attester: Address, source: Source, toolchainVersion: z.string().min(1),
  stockBuildHash: Hash, tareBuildHash: Hash, corpus: Corpus, gross: readCount, net: readCount, tare: readDelta,
  reportHash: Hash, reportLocation: z.string().min(1), slot: readCount, createdAt: Timestamp, flags: readCount,
})
export const UploadFlagInput = z.strictObject({ schemaVersion: z.literal(DOCUMENT_VERSION), record: Address, divergence: UploadDivergence })
export const FlagInput = UploadFlagInput.extend({ divergence: Divergence })
export const Flag = z.strictObject({
  address: Address, record: Address, flagger: Address, inputHash: Hash, inputLocation: z.string().min(1), slot: readCount,
  createdAt: Timestamp, input: FlagInput.nullable(),
})
export const RecordDetail = z.discriminatedUnion('status', [
  z.strictObject({ status: z.literal('ready'), record: RecordSummary, report: ProofReport, flags: z.array(Flag), checkCommand: z.string() }),
  z.strictObject({ status: z.literal('report_unavailable'), record: RecordSummary, report: z.null(), flags: z.array(Flag), checkCommand: z.string() }),
])
export const PageInfo = z.strictObject({ nextCursor: z.string().nullable(), hasMore: z.boolean() })
export const RecordsQuery = z.strictObject({ program: Address.optional(), cursor: z.string().min(1).max(256).optional(), limit: z.coerce.number().int().min(1).max(100).default(24) })
export const RecordsPage = z.strictObject({ items: z.array(RecordSummary), page: PageInfo, asOf: Timestamp })
export const RecentQuery = z.strictObject({ limit: z.coerce.number().int().min(1).max(100).default(24) })
export const SampledInstruction = z.strictObject({
  name: z.string().min(1).nullable(), discriminator: Discriminator, calls: Count, median: Count, p90: Count,
  totalUnits: Count, share: z.number().min(0).max(1),
})
export const ProgramSample = z.strictObject({
  program: ProgramIdentity, anchor: z.boolean(), hasIdl: z.boolean(), binarySize: Count, rentLamports: Count,
  sampledAt: Timestamp, transactions: Count,
  signaturesExamined: Count.optional().describe('Signature rows examined during this sampling run. Omitted in older samples where the count is unknown.'),
  signaturesFailed: Count.optional().describe('Examined signatures whose transaction failed. Omitted in older samples.'),
  instructions: z.array(SampledInstruction),
})
export const WeighResponse = z.discriminatedUnion('status', [
  z.strictObject({ status: z.literal('ready'), sample: ProgramSample, records: z.array(RecordSummary) }),
  z.strictObject({ status: z.literal('no_transactions'), sample: ProgramSample, records: z.array(RecordSummary) }),
  z.strictObject({ status: z.literal('calls_failed').describe('The program was called in the sampled window, but every sampled call failed, so none can be measured.'), sample: ProgramSample, records: z.array(RecordSummary) }),
  z.strictObject({ status: z.literal('not_program'), program: Address, records: z.array(RecordSummary) }),
  z.strictObject({ status: z.literal('sampling'), program: Address, startedAt: Timestamp, retryAfterSeconds: Count, records: z.array(RecordSummary) }),
])
export const RecentWeighings = z.strictObject({ items: z.array(ProgramSample), asOf: Timestamp })
export const UploadResult = z.strictObject({ hash: Hash, location: z.string().min(1) })
export const Burn = z.strictObject({
  signature: Signature.describe('The crank transaction, checkable on any explorer.'),
  kind: z.enum(['buy', 'burn']).describe('`buy`: the swap and the burn of its quoted minimum in one transaction. `burn`: the rest that swap delivered.'),
  spentLamports: Count.describe('SOL the launch wallet spent in this transaction, its fee left out.'),
  burned: z.number().nonnegative().describe('TARE burned in this transaction, in whole tokens.'),
  slot: Count, at: Timestamp,
})
export const TokenFacts = z.strictObject({
  ticker: z.literal('TARE'), supply: z.literal(1000000000), mint: Address.nullable(), venue: z.url().nullable(),
  developerAllocationShare: z.number().min(0).max(1).nullable(),
  developerAllocationLock: Address.nullable().describe('The Streamflow lock holding the developer allocation.'),
  feesReceivedLamports: Count.describe('Attest fees paid to the launch wallet, summed over indexed records: the same sum as `RegistryTotals.feesAccruedLamports`.'),
  spentLamports: Count.describe('SOL the crank spent buying TARE, summed over its transactions.'),
  burned: z.number().nonnegative().describe('TARE the crank burned, in whole tokens.'),
  burns: z.array(Burn).describe('The latest crank transactions, newest first, at most 20.'),
})
export const SCENARIOS = ['default', 'empty', 'waiting', 'ready', 'anchor', 'non-anchor', 'not-program', 'no-transactions', 'calls-failed', 'sampling', 'with-records', 'report-unavailable', 'diverged'] as const
export type Scenario = typeof SCENARIOS[number]
export type UploadWeighing = z.output<typeof UploadWeighing>
export type UploadProofReport = z.output<typeof UploadProofReport>
export type BenchResults = z.output<typeof BenchResults>
export type Compat = z.output<typeof Compat>
export type BenchTotals = z.output<typeof BenchTotals>
export type RecordSummary = z.output<typeof RecordSummary>
export type RecordDetail = z.output<typeof RecordDetail>
export type UploadFlagInput = z.output<typeof UploadFlagInput>
export type WeighResponse = z.output<typeof WeighResponse>
export type ProgramSample = z.output<typeof ProgramSample>
export type RegistryTotals = z.output<typeof RegistryTotals>
export type UploadResult = z.output<typeof UploadResult>
export type Burn = z.output<typeof Burn>
export type TokenFacts = z.output<typeof TokenFacts>
export type PublicConfig = z.output<typeof PublicConfig>

export type UploadOptimization = z.output<typeof UploadOptimization>
export type UploadValidatorReproduction = z.output<typeof UploadValidatorReproduction>
export type UploadRecordedProvenance = z.output<typeof UploadRecordedProvenance>
export type UploadRecordedCorpus = z.output<typeof UploadRecordedCorpus>

export type Replay = z.output<typeof Replay>

export type UploadAccountMetadata = z.output<typeof UploadAccountMetadata>

export type UploadedU64 = z.output<typeof UploadedU64>

export type UploadedI64 = z.output<typeof UploadedI64>
export type UploadedCorpusCounts = z.output<typeof UploadedCorpusCounts>
export type UploadedCorpus = z.output<typeof UploadedCorpus>
export type UploadedWeights = z.output<typeof UploadedWeights>
export type BenchProgram = z.output<typeof BenchProgram>

export type InstructionWeight = z.output<typeof InstructionWeight>
export type Build = z.output<typeof Build>
export type AccountSnapshot = z.output<typeof AccountSnapshot>
export type ReplayInput = z.output<typeof ReplayInput>
export type AccountMetadata = z.output<typeof AccountMetadata>
export type Difference = z.output<typeof Difference>
export type Divergence = z.output<typeof Divergence>
export type Comparisons = z.output<typeof Comparisons>
export type ProofSummary = z.output<typeof ProofSummary>
export type Proof = z.output<typeof Proof>
export type Optimization = z.output<typeof Optimization>
export type ValidatorReproduction = z.output<typeof ValidatorReproduction>
export type RecordedProvenance = z.output<typeof RecordedProvenance>
export type RecordedCorpus = z.output<typeof RecordedCorpus>
export type Weighing = z.output<typeof Weighing>
export type ProofReport = z.output<typeof ProofReport>
export type FlagInput = z.output<typeof FlagInput>
export type WeighingTotals = z.output<typeof WeighingTotals>
export type BuildEnvironment = z.output<typeof BuildEnvironment>
export type AccountAbi = z.output<typeof AccountAbi>
