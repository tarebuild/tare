import { mkdirSync, writeFileSync } from 'node:fs'
import { z } from 'zod'
import { Address } from './primitives.js'
import { openApiDocument, type Operation } from './openapi.js'
import * as s from './schemas.js'
import * as f from './fixtures.js'

const root = new URL('../', import.meta.url)
const write = (path: string, value: unknown) => writeFileSync(new URL(path, root), JSON.stringify(value, null, 2) + '\n', 'utf8')
mkdirSync(new URL('schema/', root), { recursive: true })
mkdirSync(new URL('fixtures/', root), { recursive: true })
const schemas = {
  UploadWeighing: s.UploadWeighing, UploadProofReport: s.UploadProofReport, UploadFlagInput: s.UploadFlagInput,
  BuildEnvironment: s.BuildEnvironment, AccountAbi: s.AccountAbi, Build: s.Build, WeighingTotals: s.WeighingTotals,
  UploadedI64: s.UploadedI64, UploadedWeights: s.UploadedWeights, UploadedCorpus: s.UploadedCorpus, UploadedCorpusCounts: s.UploadedCorpusCounts,
  UploadedU64: s.UploadedU64, Replay: s.Replay, Optimization: s.Optimization, ValidatorReproduction: s.ValidatorReproduction, RecordedProvenance: s.RecordedProvenance, RecordedCorpus: s.RecordedCorpus,
  Address, PublicConfig: s.PublicConfig, Hash: s.Hash, ApiError: s.ApiError, Health: s.Health, Weighing: s.Weighing,
  ProofReport: s.ProofReport, BenchResults: s.BenchResults, BenchTotals: s.BenchTotals, Compat: s.Compat,
  RegistryTotals: s.RegistryTotals, RecordsPage: s.RecordsPage, RecordDetail: s.RecordDetail,
  WeighResponse: s.WeighResponse, RecentWeighings: s.RecentWeighings,
  FlagInput: s.FlagInput, UploadResult: s.UploadResult, TokenFacts: s.TokenFacts,
}
for (const [name, schema] of Object.entries(schemas)) {
  write(`schema/${name}.json`, { ...z.toJSONSchema(schema, { target: 'draft-2020-12' }), title: name })
}
const operations: Operation[] = [
  { method: 'get', path: '/v1/config', operationId: 'config', summary: 'Public install command and site URL for CLI record links', response: 'PublicConfig' },
  { method: 'get', path: '/health', operationId: 'health', summary: 'Service readiness', response: 'Health' },
  { method: 'get', path: '/v1/bench', operationId: 'bench', summary: 'Measured benchmark or an empty document', response: 'BenchResults' },
  { method: 'get', path: '/v1/bench/totals', operationId: 'benchTotals', summary: 'Benchmark corpus totals', response: 'BenchTotals' },
  { method: 'get', path: '/v1/compat', operationId: 'compat', summary: 'Anchor compatibility matrix', response: 'Compat' },
  { method: 'get', path: '/v1/registry/totals', operationId: 'registryTotals', summary: 'Registry totals', response: 'RegistryTotals' },
  { method: 'get', path: '/v1/records', operationId: 'records', summary: 'Newest records first', response: 'RecordsPage', query: s.RecordsQuery },
  { method: 'get', path: '/v1/records/{record}', operationId: 'record', summary: 'A record with its pinned proof and flags', response: 'RecordDetail', params: { record: 'Address' } },
  { method: 'get', path: '/v1/weigh', operationId: 'recentWeighings', summary: 'Recently weighed programs', response: 'RecentWeighings', query: s.RecentQuery },
  { method: 'get', path: '/v1/weigh/{program}', operationId: 'weigh', summary: 'Start sampling or return a cached weighing', response: 'WeighResponse', params: { program: 'Address' }, description: 'The sampling status is an ordinary 200 response; retry after retryAfterSeconds. This request may start one bounded mainnet sampling run.' },
  { method: 'post', path: '/v1/reports', operationId: 'uploadReport', summary: 'Pin a validated proof report', response: 'UploadResult', request: 'UploadProofReport', created: true },
  { method: 'get', path: '/v1/reports/{hash}', operationId: 'report', summary: 'Read a proof report by SHA-256', response: 'ProofReport', params: { hash: 'Hash' } },
  { method: 'post', path: '/v1/flag-inputs', operationId: 'uploadFlagInput', summary: 'Pin a validated diverging input', response: 'UploadResult', request: 'UploadFlagInput', created: true },
  { method: 'get', path: '/v1/flag-inputs/{hash}', operationId: 'flagInput', summary: 'Read a diverging input by SHA-256', response: 'FlagInput', params: { hash: 'Hash' } },
  { method: 'get', path: '/v1/token', operationId: 'token', summary: 'Token facts; unset values are null', response: 'TokenFacts' },
]
const openapi = openApiDocument({
  title: 'Tare API', version: s.CONTRACT_VERSION,
  description: 'Public reads and two size-capped, rate-limited JSON uploads. All fixture values are fictional.',
  servers: [{ url: 'http://127.0.0.1:7810', description: 'Local API' }], schemas, operations,
  error: { schema: 'ApiError', statuses: [...new Set(Object.values(s.errors.status))] },
  fixture: { header: 'X-Fixture', values: [...s.SCENARIOS, ...s.ErrorCode.options] },
})
for (const path of Object.values(openapi.paths)) {
  for (const operation of Object.values(path)) {
    (operation as { parameters: unknown[] }).parameters.push({ in: 'query', name: 'fixture', required: false, description: 'Fixture mode only; equivalent to X-Fixture.', schema: { type: 'string', enum: [...s.SCENARIOS, ...s.ErrorCode.options] } })
  }
}
for (const path of ['/v1/reports/{hash}', '/v1/flag-inputs/{hash}']) {
  const operation = openapi.paths[path]!.get as { description?: string; parameters: unknown[]; responses: Record<string, { content?: Record<string, unknown> }> }
  operation.description = 'Send Accept: application/json for the normalized public read shape. Without that explicit media type, the original uploaded bytes are returned for content-hash verification.'
  operation.parameters.push({ in: 'header', name: 'Accept', required: false, schema: { type: 'string', enum: ['application/json', 'application/octet-stream', '*/*'] }, description: 'application/json selects the normalized public read shape; other values retain original bytes.' })
  operation.responses['200']!.content!['application/octet-stream'] = { schema: { $ref: path.includes('/reports/') ? '#/components/schemas/UploadProofReport' : '#/components/schemas/UploadFlagInput' } }
}
const rawError = { description: 'API error envelope.', content: { 'application/json': { schema: { $ref: '#/components/schemas/ApiError' } } } }
openapi.paths['/install.sh'] = {
  get: {
    operationId: 'installer', summary: 'Download the configured CLI installer', parameters: [],
    responses: {
      '200': { description: 'POSIX shell installer.', content: { 'text/x-shellscript': { schema: { type: 'string' } } } },
      '409': rawError,
      '500': rawError,
    },
  },
}
openapi.paths['/releases/{file}'] = {
  get: {
    operationId: 'releaseFile', summary: 'Download a local CLI release file',
    parameters: [{ in: 'path', name: 'file', required: true, description: 'A release filename without directory separators.', schema: { type: 'string', pattern: '^[A-Za-z0-9][A-Za-z0-9._-]*$' } }],
    responses: {
      '200': {
        description: 'Release archive, checksum, version marker or other release file.',
        content: {
          'application/gzip': { schema: { type: 'string', format: 'binary' } },
          'text/plain': { schema: { type: 'string' } },
          'application/octet-stream': { schema: { type: 'string', format: 'binary' } },
        },
      },
      '404': rawError,
      '500': rawError,
    },
  },
}
write('openapi.json', openapi)
write('fixtures/results.fixture.json', s.BenchResults.parse(f.fixtureBench))
write('fixtures/compat.fixture.json', s.Compat.parse(f.fixtureCompat))
write('fixtures/proof.fixture.json', s.ProofReport.parse(f.fixtureReport))
write('fixtures/proof-diverged.fixture.json', s.ProofReport.parse(f.fixtureDivergedReport))
write('fixtures/flag-input.fixture.json', s.FlagInput.parse(f.fixtureFlagInput))
write('fixtures/upload-proof.fixture.json', s.UploadProofReport.parse(f.fixtureReport))
write('fixtures/upload-flag-input.fixture.json', s.UploadFlagInput.parse(f.fixtureUploadFlagInput))
write('fixtures/states.fixture.json', {
  notice: f.FIXTURE_NOTICE, config: s.PublicConfig.parse(f.fixtureConfig),
  programs: f.fixturePrograms,
  health: ['fixture', 'waiting', 'ready'].map((status) => s.Health.parse({ status })),
  emptyBench: s.BenchResults.parse(f.emptyBench), emptyCompat: s.Compat.parse(f.emptyCompat),
  registry: s.RegistryTotals.parse(f.fixtureRegistryTotals), emptyRegistry: s.RegistryTotals.parse(f.emptyRegistryTotals),
  records: s.RecordsPage.parse({ items: f.fixtureRecords, page: { nextCursor: null, hasMore: false }, asOf: '2026-11-14T00:00:00.000Z' }),
  emptyRecords: s.RecordsPage.parse({ items: [], page: { nextCursor: null, hasMore: false }, asOf: '2026-11-14T00:00:00.000Z' }),
  record: s.RecordDetail.parse(f.fixtureRecordDetail), reportUnavailable: s.RecordDetail.parse(f.fixtureUnavailableRecord),
  weighings: Object.fromEntries(Object.entries(f.fixtureWeighings).map(([key, value]) => [key, s.WeighResponse.parse(value)])),
  recentWeighings: s.RecentWeighings.parse(f.fixtureRecentWeighings),
  token: s.TokenFacts.parse(f.fixtureToken),
  upload: s.UploadResult.parse({ hash: 'd'.repeat(64), location: '/v1/reports/' + 'd'.repeat(64) }),
  errors: Object.fromEntries(Object.entries(f.fixtureErrors).map(([key, value]) => [key, s.ApiError.parse(value)])),
})
