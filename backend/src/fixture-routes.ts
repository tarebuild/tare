import { readFile } from 'node:fs/promises'
import { json } from './json.js'
import { createHash } from 'node:crypto'
import { Hono } from 'hono'
import * as s from '@tare/contract'
import * as f from '@tare/contract/fixtures'
import { fixtureScenario, type FixtureVariables } from './engines/server/fixture.js'
import { readParam, readQuery } from './engines/server/input.js'
import { fail } from './errors.js'
import { createBenchReader } from './bench.js'
import { readUpload } from './uploads.js'
import { publicConfig } from './public-config.js'
import { contentResponse } from './content-response.js'

export function fixtureRoutes() {
  const app = new Hono<{ Variables: FixtureVariables<s.Scenario> }>()
  const bench = createBenchReader()
  const reports = new Map<string, Buffer>()
  const inputs = new Map<string, Buffer>()
  app.use('*', fixtureScenario({ scenarios: s.SCENARIOS, errors: f.fixtureErrors, status: s.errors.status }))
  const empty = (scenario: s.Scenario | null) => scenario === 'empty' || scenario === 'waiting'
  app.get('/health', (c) => json(c, { status: c.get('scenario') === 'waiting' ? 'waiting' : c.get('scenario') === 'ready' ? 'ready' : 'fixture' }))
  app.get('/v1/config', (c) => json(c, publicConfig()))
  app.get('/v1/bench', async (c) => json(c, await bench.results()))
  app.get('/v1/bench/totals', async (c) => json(c, await bench.totals()))
  app.get('/v1/compat', async (c) => json(c, await bench.compatibility()))
  app.get('/v1/registry/totals', (c) => json(c, empty(c.get('scenario')) ? f.emptyRegistryTotals : f.fixtureRegistryTotals))
  app.get('/v1/records', (c) => {
    const q = readQuery(c, s.RecordsQuery)
    const rows = empty(c.get('scenario')) ? [] : f.fixtureRecords.filter((r) => !q.program || r.program.address === q.program)
    const start = q.cursor ? rows.findIndex((r) => r.address === q.cursor) + 1 : 0
    if (q.cursor && start === 0) fail('VALIDATION_ERROR', 'The record cursor is unknown.', { stage: 'validation' })
    const items = rows.slice(start, start + q.limit)
    const hasMore = start + q.limit < rows.length
    return json(c, { items, page: { nextCursor: hasMore ? items.at(-1)!.address : null, hasMore }, asOf: '2026-11-14T00:00:00.000Z' })
  })
  app.get('/v1/records/:record', (c) => {
    const address = readParam(c, 'record', s.Address)
    const record = f.fixtureRecords.find((r) => r.address === address)
    if (!record || empty(c.get('scenario'))) fail('RECORD_NOT_FOUND', 'This registry record was not found.')
    if (c.get('scenario') === 'report-unavailable') return json(c, { ...f.fixtureUnavailableRecord, record, flags: record.address === f.fixtureRecordAddress ? f.fixtureUnavailableRecord.flags : [], checkCommand: `tare check ${address}` })
    if (record.address === f.fixtureRecordAddress) {
      return json(c, { ...f.fixtureRecordDetail, report: c.get('scenario') === 'diverged' ? f.fixtureDivergedReport : f.fixtureReport })
    }
    const program = f.fixtureBench.programs.find((p) => p.status === 'measured' && p.weighing.program.address === record.program.address)
    if (!program || program.status !== 'measured') throw new Error('Fixture record has no report')
    return json(c, { status: 'ready', record, report: program.weighing, flags: [], checkCommand: `tare check ${address}` })
  })
  app.get('/v1/weigh', (c) => {
    const q = readQuery(c, s.RecentQuery)
    return json(c, { ...f.fixtureRecentWeighings, items: empty(c.get('scenario')) ? [] : f.fixtureRecentWeighings.items.slice(0, q.limit) })
  })
  app.get('/v1/weigh/:program', async (c) => {
    const program = readParam(c, 'program', s.Address)
    const scenario = c.get('scenario')
    if (!scenario || scenario === 'default') {
      let source: string | undefined
      try { source = await readFile(`contract/fixtures/weigh/${program}.sample.json`, 'utf8') }
      catch (error) { if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error }
      if (source !== undefined) {
        const sample = s.ProgramSample.parse(JSON.parse(source))
        return json(c, { status: sample.transactions ? 'ready' : 'no_transactions', sample, records: [] })
      }
    }
    const byAddress: Record<string, string> = {
      [f.fixturePrograms.anchor]: 'anchor', [f.fixturePrograms.nonAnchor]: 'non-anchor',
      [f.fixturePrograms.notProgram]: 'not-program', [f.fixturePrograms.noTransactions]: 'no-transactions', [f.fixturePrograms.callsFailed]: 'calls-failed',
      [f.fixturePrograms.sampling]: 'sampling', [f.fixturePrograms.withRecords]: 'with-records',
    }
    const chosen = scenario === 'waiting' ? 'sampling' : scenario && f.fixtureWeighings[scenario] ? scenario : byAddress[program] ?? 'anchor'
    const answer = structuredClone(f.fixtureWeighings[chosen]!)
    if ('sample' in answer) answer.sample.program.address = program
    else answer.program = program
    for (const record of answer.records) record.program.address = program
    if (answer.status === 'sampling') c.header('Retry-After', String(answer.retryAfterSeconds))
    return json(c, answer)
  })
  app.post('/v1/reports', async (c) => {
    const { bytes } = await readUpload(c, s.UploadProofReport)
    const hash = createHash('sha256').update(bytes).digest('hex')
    reports.set(hash, bytes)
    return json(c, { hash, location: `/v1/reports/${hash}` }, 201)
  })
  app.get('/v1/reports/:hash', (c) => {
    const hash = readParam(c, 'hash', s.Hash)
    if (c.get('scenario') === 'report-unavailable') fail('REPORT_UNAVAILABLE', 'The pinned report is unavailable.', { retryable: true })
    const uploaded = reports.get(hash)
    if (uploaded) return contentResponse(c, 'report', uploaded)
    const value = (hash === 'd'.repeat(64) ? f.fixtureReport : hash === 'f'.repeat(64) ? f.fixtureBench.programs[1] : undefined)
    if (!value) fail('REPORT_UNAVAILABLE', 'The pinned report is unavailable.', { retryable: true })
    const document = 'status' in value && value.status === 'measured' ? value.weighing : c.get('scenario') === 'diverged' ? f.fixtureDivergedReport : value
    return contentResponse(c, 'report', Buffer.from(JSON.stringify(document)))
  })
  app.post('/v1/flag-inputs', async (c) => {
    const { bytes } = await readUpload(c, s.UploadFlagInput)
    const hash = createHash('sha256').update(bytes).digest('hex')
    inputs.set(hash, bytes)
    return json(c, { hash, location: `/v1/flag-inputs/${hash}` }, 201)
  })
  app.get('/v1/flag-inputs/:hash', (c) => {
    const hash = readParam(c, 'hash', s.Hash)
    const uploaded = inputs.get(hash)
    if (uploaded) return contentResponse(c, 'flag_input', uploaded)
    const value = (hash === 'e'.repeat(64) ? f.fixtureFlagInput : undefined)
    if (!value) fail('FLAG_INPUT_UNAVAILABLE', 'The pinned flag input is unavailable.', { retryable: true })
    return contentResponse(c, 'flag_input', Buffer.from(JSON.stringify(value)))
  })
  app.get('/v1/token', (c) => json(c, empty(c.get('scenario')) ? f.fixtureToken : f.fixtureTokenBurning))
  return app
}
