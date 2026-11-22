import type { z } from 'zod'
import * as s from './schemas.js'
import { fixtureAddress, fixtureTransaction } from './fixture-helpers.js'
import type { ApiErrorBody } from './errors.js'
export { fixtureAddress, fixtureSignature } from './fixture-helpers.js'

export const FIXTURE_NOTICE = 'Fictional fixture data. These programs, addresses and weights are not benchmark evidence.'
const at = '2026-11-14T00:00:00.000Z'
const hash = (character: string) => character.repeat(64)
export const fixturePrograms = {
  anchor: fixtureAddress('fixture-anchor-vault'),
  nonAnchor: fixtureAddress('fixture-native-counter'),
  notProgram: fixtureAddress('fixture-wallet'),
  noTransactions: fixtureAddress('fixture-idle-program'),
  callsFailed: fixtureAddress('fixture-failing-program'),
  sampling: fixtureAddress('fixture-sampling-program'),
  withRecords: fixtureAddress('fixture-recorded-program'),
} as const
export const fixtureRecordAddress = fixtureAddress('fixture-record-one')
export const fixtureReplay: s.Replay = {
  encoding: 'gzip+base64',
  sha256: '47fb60393b573d3159d7a5c06a0ba0773dc95c1cbb39ebc8e01c769a79a21ad4',
  data: 'H4sIAAAAAAAC/z2OwQrCQAxE/yXnYum1NxE8e/IiInFN24XdrCZZKZT+u1tbPSXzmBlmAnUDRTyTqE8MLTRQQUT2Hamd0IaCnpJ6wah150fLQrc35mD1AaVPO0sxlEhPTIJGD2gnUFpuU/2S++BRSaG9XCugkVw2vAc6rnUbV3plYveTntUkOyurvmT+t20GKwPLyzmEZbFnJlvl/AGkLknV1gAAAA==',
}
const corpus = { hash: fixtureReplay.sha256, counts: { tests: 12, generated: 80, mainnet: 8 } }
const comparisons = { inputs: 100, accountBytes: 300, lamports: 300, logs: 100, returnData: 100, errors: 100 }
export const fixtureOptimization: s.Optimization = {
  inputBytes: 204800, outputBytes: 102400, inputHash: hash('a'), outputHash: hash('b'), textInstructions: 10000,
  branchTargetsThreaded: 8, callTargetsThreaded: 2, stackLoadsForwarded: 4,
  arithmeticStrengthReductions: 3, constantBranchesSimplified: 1, identityRunsSkipped: 5,
  identityInstructionsBypassed: 10, unreachableInstructionsNeutralized: 30, unreachableFunctionsNeutralized: 2,
  indirectCalls: 0, relocationSitesPreserved: 120, rootPcs: [0, 20], modifiedPcs: [30, 40],
}
export const fixtureRecordedCorpus: s.RecordedCorpus = [{
  generatedCorpusHash: hash('c'), recordedCorpusHash: hash('f'), capturedStockHash: hash('a'), replayStockHash: hash('a'),
  sameStockBinary: true,
  validatorReproduction: { inputs: 12, matched: 12, failed: 0, clockExactInputs: 12, firstDifference: null },
}]
export const fixtureReport: s.ProofReport = {
  schemaVersion: '1', program: { name: 'fixture_vault', address: fixturePrograms.withRecords },
  toolchainVersion: '0.2.0-fixture', anchorVersion: '0.31.1',
  source: { repository: 'https://example.invalid/tare-fixtures/vault', commit: 'a'.repeat(40) },
  builds: { stock: { hash: hash('a'), sizeBytes: 204800, rentLamports: 1200000000 }, tare: { hash: hash('b'), sizeBytes: 102400, rentLamports: 600000000 } },
  instructions: [
    { name: 'initialize', discriminator: 'afaf6d1f0d989bed', gross: 12000, net: 5500, tare: 6500, inputs: 10 },
    { name: 'deposit', discriminator: 'f223c68952e1f2b6', gross: 9000, net: 3400, tare: 5600, inputs: 70 },
    { name: 'withdraw', discriminator: 'b712469c946da122', gross: 11500, net: 4500, tare: 7000, inputs: 20 },
    { name: 'close_vault', discriminator: '79a8589d7f31a601', gross: null, net: null, tare: null, inputs: 0 },
  ],
  totals: { gross: 32500, net: 13400, tare: 19100, inputs: 100 },
  proof: { status: 'matched', corpus, comparisons, firstDivergence: null },
  optimization: fixtureOptimization, recordedCorpus: fixtureRecordedCorpus, replay: fixtureReplay,
}
export const fixtureFlagInput: s.FlagInput = {
  schemaVersion: '1', record: fixtureRecordAddress,
  divergence: {
    instructionIndex: 0,
    input: {
      kind: 'generated', label: 'fixture_overflow_boundary', transactionBase64: fixtureTransaction('overflow-boundary'), slot: null,
      accounts: [{ address: fixtureAddress('fixture-vault-account'), owner: fixturePrograms.withRecords, lamports: '5000000', executable: false, dataBase64: 'AA==', rentEpoch: '0' }],
    },
    differences: [
      { kind: 'account_bytes', account: fixtureAddress('fixture-vault-account'), gross: 'AA==', net: 'AQ==' },
      { kind: 'lamports', account: fixtureAddress('fixture-vault-account'), gross: '5000000', net: '4999999' },
      { kind: 'account_metadata', account: fixtureAddress('fixture-vault-account'), gross: { owner: fixturePrograms.withRecords, executable: false, rentEpoch: '0' }, net: { owner: fixturePrograms.nonAnchor, executable: false, rentEpoch: '0' } },
      { kind: 'logs', gross: ['Program log: fixture deposit'], net: ['Program log: fixture withdraw'] },
      { kind: 'return_data', gross: { program: fixturePrograms.withRecords, dataBase64: 'AA==' }, net: { program: fixturePrograms.withRecords, dataBase64: 'AQ==' } },
      { kind: 'error', gross: null, net: { InstructionError: [0, { Custom: 6000 }] } },
    ],
  },
}
export const fixtureUploadFlagInput: s.UploadFlagInput = {
  ...fixtureFlagInput,
  divergence: {
    ...fixtureFlagInput.divergence,
    input: {
      ...fixtureFlagInput.divergence.input,
      accounts: fixtureFlagInput.divergence.input.accounts.map((account) => ({ ...account, lamports: Number(account.lamports), rentEpoch: 0 })),
    },
  },
}
export const fixtureDivergedReport: s.ProofReport = {
  ...fixtureReport,
  proof: { status: 'diverged', corpus, comparisons, firstDivergence: fixtureFlagInput.divergence },
}
export const fixtureRecords: s.RecordSummary[] = [
  {
    address: fixtureRecordAddress, program: fixtureReport.program,
    attester: fixtureAddress('fixture-attester'), source: fixtureReport.source,
    toolchainVersion: fixtureReport.toolchainVersion, stockBuildHash: hash('a'), tareBuildHash: hash('b'),
    corpus, gross: 32500, net: 13400, tare: 19100,
    reportHash: hash('d'), reportLocation: `/v1/reports/${hash('d')}`, slot: 123456, createdAt: at, flags: 1,
  },
  {
    address: fixtureAddress('fixture-record-two'), program: { name: 'fixture_counter', address: fixturePrograms.anchor },
    attester: fixtureAddress('fixture-attester-two'), source: { repository: 'https://example.invalid/tare-fixtures/counter', commit: 'b'.repeat(40) },
    toolchainVersion: '0.2.0-fixture', stockBuildHash: hash('e'), tareBuildHash: hash('f'),
    corpus: { hash: hash('e'), counts: { tests: 3, generated: 20, mainnet: 0 } }, gross: 2000, net: 700, tare: 1300,
    reportHash: hash('f'), reportLocation: `/v1/reports/${hash('f')}`, slot: 123455, createdAt: '2026-11-13T00:00:00.000Z', flags: 0,
  },
]
export const fixtureRecordDetail: s.RecordDetail = {
  status: 'ready', record: fixtureRecords[0]!, report: fixtureReport,
  flags: [{ address: fixtureAddress('fixture-flag'), record: fixtureRecordAddress, flagger: fixtureAddress('fixture-flagger'), inputHash: hash('e'), inputLocation: `/v1/flag-inputs/${hash('e')}`, slot: 123460, createdAt: at, input: fixtureFlagInput }],
  checkCommand: `tare check ${fixtureRecordAddress}`,
}
export const fixtureUnavailableRecord: s.RecordDetail = { ...fixtureRecordDetail, status: 'report_unavailable', report: null }
const anchorSample: s.ProgramSample = {
  program: { name: 'fixture_vault', address: fixturePrograms.anchor }, anchor: true, hasIdl: true,
  binarySize: 204800, rentLamports: 1200000000, sampledAt: at, transactions: 40, signaturesExamined: 46,
  instructions: [
    { name: 'initialize', discriminator: 'afaf6d1f0d989bed', calls: 4, median: 12000, p90: 13500, totalUnits: 48000, share: 0.12 },
    { name: 'deposit', discriminator: 'f223c68952e1f2b6', calls: 28, median: 9000, p90: 11200, totalUnits: 252000, share: 0.63 },
    { name: 'withdraw', discriminator: 'b712469c946da122', calls: 8, median: 12500, p90: 14500, totalUnits: 100000, share: 0.25 },
  ],
}
export const fixtureWeighings: Record<string, s.WeighResponse> = {
  anchor: { status: 'ready', sample: anchorSample, records: [] },
  'non-anchor': {
    status: 'ready', sample: { ...anchorSample, program: { name: null, address: fixturePrograms.nonAnchor }, anchor: false, hasIdl: false, binarySize: 64000,
      instructions: [{ name: null, discriminator: '0901000000000000', calls: 40, median: 1200, p90: 1800, totalUnits: 52000, share: 1 }] }, records: [],
  },
  'not-program': { status: 'not_program', program: fixturePrograms.notProgram, records: [] },
  'no-transactions': { status: 'no_transactions', sample: { ...anchorSample, program: { name: 'fixture_idle', address: fixturePrograms.noTransactions }, transactions: 0, signaturesExamined: 0, instructions: [] }, records: [] },
  'calls-failed': { status: 'calls_failed', sample: { ...anchorSample, program: { name: 'fixture_paused', address: fixturePrograms.callsFailed }, transactions: 0, signaturesExamined: 1000, signaturesFailed: 831, instructions: [] }, records: [] },
  sampling: { status: 'sampling', program: fixturePrograms.sampling, startedAt: at, retryAfterSeconds: 2, records: [] },
  'with-records': { status: 'ready', sample: { ...anchorSample, program: fixtureReport.program }, records: [fixtureRecords[0]!] },
}
export const fixtureRecentWeighings = { items: [anchorSample, (fixtureWeighings['non-anchor'] as Extract<s.WeighResponse, { status: 'ready' }>).sample], asOf: at }
export const emptyRegistryTotals: s.RegistryTotals = { records: 0, programs: 0, flags: 0, gross: 0, net: 0, tare: 0, corpus: 0, feesAccruedLamports: 0 }
export const fixtureRegistryTotals: s.RegistryTotals = { records: 2, programs: 2, flags: 1, gross: 34500, net: 14100, tare: 20400, corpus: 123, feesAccruedLamports: 0 }
export const fixtureToken: z.output<typeof s.TokenFacts> = { ticker: 'TARE', supply: 1000000000, mint: null, venue: null, developerAllocationShare: null, developerAllocationLock: null, feesReceivedLamports: 0, spentLamports: 0, burned: 0, burns: [],
  bySource: { attest: { feesReceivedLamports: 0, spentLamports: 0, burned: 0 }, trading: { feesReceivedLamports: 0, keptLamports: 0, spentLamports: 0, burned: 0 } } }
export const fixtureTokenBurning: z.output<typeof s.TokenFacts> = {
  ...fixtureToken, mint: 'BW2pMhQQSwGui8bW1iW1AJUUhYtEAYMDhBAkVHDmSrcX', feesReceivedLamports: 100000000, spentLamports: 50000000, burned: 412913.586421,
  bySource: { attest: { feesReceivedLamports: 100000000, spentLamports: 50000000, burned: 412913.586421 }, trading: { feesReceivedLamports: 0, keptLamports: 0, spentLamports: 0, burned: 0 } },
  burns: [
    { signature: '4zKGNEW2dKgKixRsyJQDL5rz8bGUo4JFv5yCXRN4tCWPeb9djBfav6HrCutzya1ihAGF6wxesNqBByzs8wDQFgvp', kind: 'burn', source: 'attest', spentLamports: 0, burned: 2031.552108, claimedLamports: 0, keptLamports: 0, claimSignature: null, slot: 381000140, at: '2026-11-22T21:00:14.000Z' },
    { signature: '4qnML9cJBeJT6qSbmv486WycqzZwmoAmbrzssZxMQyw5Z6g4m9ZYkNACdNL3GHzpca1f7zW7P5UiZ4ZdiKKDKFSR', kind: 'buy', source: 'attest', spentLamports: 50000000, burned: 410882.034313, claimedLamports: 0, keptLamports: 0, claimSignature: null, slot: 381000128, at: '2026-11-22T21:00:09.000Z' },
  ],
}
export const emptyBench: s.BenchResults = {
  schemaVersion: '1', fixture: false, toolchainVersion: null, generatedAt: null, programs: [],
  totals: { programs: 0, measured: 0, notBuilt: 0, instructions: 0, weights: { gross: 0, net: 0, tare: 0, inputs: 0 }, binarySize: { gross: 0, net: 0, tare: 0 }, rentLamports: { gross: 0, net: 0, tare: 0 }, corpus: { tests: 0, generated: 0, mainnet: 0 } },
}
export const fixtureBench: s.BenchResults = {
  schemaVersion: '1', fixture: true, toolchainVersion: '0.2.0-fixture', generatedAt: at,
  programs: [
    { id: 'fixture-vault', name: 'Fixture Vault', source: fixtureReport.source, anchorVersion: '0.31.1', kind: 'anchor_example', status: 'measured', weighing: fixtureReport, tests: { ran: 12, passed: 12, failed: 0 }, proof: { status: 'matched', corpus, comparisons }, record: fixtureRecordAddress, recordCluster: 'localnet', recordUrl: `http://127.0.0.1:7800/builds/${fixtureRecordAddress}` },
    { id: 'fixture-counter', name: 'Fixture Counter', source: fixtureRecords[1]!.source, anchorVersion: '0.32.1', kind: 'production', status: 'measured',
      weighing: { ...fixtureReport, optimization: null, recordedCorpus: null, replay: undefined, program: fixtureRecords[1]!.program, source: fixtureRecords[1]!.source, anchorVersion: '0.32.1',
        builds: { stock: { hash: hash('e'), sizeBytes: 100000, rentLamports: 600000000 }, tare: { hash: hash('f'), sizeBytes: 70000, rentLamports: 420000000 } },
        instructions: [{ name: 'increment', discriminator: '0b12680968ae3b21', gross: 2000, net: 700, tare: 1300, inputs: 23 }],
        totals: { gross: 2000, net: 700, tare: 1300, inputs: 23 }, proof: { status: 'matched', corpus: fixtureRecords[1]!.corpus, comparisons: { inputs: 23, accountBytes: 23, lamports: 23, logs: 23, returnData: 23, errors: 23 }, firstDivergence: null } },
      tests: { ran: 3, passed: 3, failed: 0 }, proof: { status: 'matched', corpus: fixtureRecords[1]!.corpus, comparisons: { inputs: 23, accountBytes: 23, lamports: 23, logs: 23, returnData: 23, errors: 23 } }, record: fixtureRecords[1]!.address, recordCluster: 'devnet' },
    { id: 'fixture-loader', name: 'Fixture Loader (unsupported)', source: { repository: 'https://example.invalid/tare-fixtures/loader', commit: 'c'.repeat(40) }, anchorVersion: '1.0.0', kind: 'anchor_test', status: 'not_built', reason: 'Fixture toolchain does not implement AccountLoader for this version.', construct: 'AccountLoader' },
  ],
  totals: { programs: 3, measured: 2, notBuilt: 1, instructions: 4, weights: { gross: 34500, net: 14100, tare: 20400, inputs: 123 }, binarySize: { gross: 304800, net: 172400, tare: 132400 }, rentLamports: { gross: 1800000000, net: 1020000000, tare: 780000000 }, corpus: { tests: 15, generated: 100, mainnet: 8 } },
}
export const emptyCompat: s.Compat = { schemaVersion: '1', fixture: false, toolchainVersion: null, generatedAt: null, anchorVersions: [], constructs: [] }
export const fixtureCompat: s.Compat = {
  schemaVersion: '1', fixture: true, toolchainVersion: '0.2.0-fixture', generatedAt: at, anchorVersions: ['0.31.1', '0.32.1', '1.0.0'],
  constructs: s.CONSTRUCTS.map((construct) => ({ construct, versions: ['0.31.1', '0.32.1', '1.0.0'].map((anchorVersion) => ({ anchorVersion, supported: !(construct === 'AccountLoader' && anchorVersion === '1.0.0'), programs: [construct === 'AccountLoader' ? 'fixture-loader' : 'fixture-vault'], note: 'Fictional fixture compatibility only.' })) })),
}
export const fixtureErrors = Object.fromEntries(s.ErrorCode.options.map((code) => [code, {
  error: { code, message: `Fixture: ${code.toLowerCase().replaceAll('_', ' ')}.`, retryable: ['PROGRAM_UNAVAILABLE', 'REPORT_UNAVAILABLE', 'FLAG_INPUT_UNAVAILABLE', 'RPC_UNAVAILABLE', 'PINNING_UNAVAILABLE', 'RATE_LIMITED', 'UPSTREAM_UNAVAILABLE'].includes(code), stage: code === 'INVALID_REPORT' || code === 'INVALID_FLAG_INPUT' ? 'upload' : code === 'VALIDATION_ERROR' ? 'validation' : 'read',
    ...(code === 'RATE_LIMITED' ? { retryAfterSeconds: 60 } : {}),
    ...(code === 'VALIDATION_ERROR' ? { fieldErrors: { program: ['Must be a base58 address of 32 bytes.'] } } : {}),
  },
}])) as Record<s.ErrorCode, ApiErrorBody<s.ErrorCode>>

export const fixtureConfig: s.PublicConfig = { siteUrl: 'http://127.0.0.1:7800', recordUrlTemplate: 'http://127.0.0.1:7800/builds/{record}', installLine: 'curl -fsSL http://127.0.0.1:7810/install.sh | sh' }
