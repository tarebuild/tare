import { json } from './json.js'
import type { Record as StoredRecord } from '@prisma/client'
import { Hono } from 'hono'
import * as s from '@tare/contract'
import { emptyRegistryTotals, fixtureToken } from '@tare/contract/fixtures'
import { createApi } from './engines/server/app.js'
import { ApiFailure } from './engines/server/errors.js'
import { readParam, readQuery } from './engines/server/input.js'
import { writeLimit } from './engines/server/limits.js'
import { env } from './config.js'
import { fail } from './errors.js'
import { fixtureRoutes } from './fixture-routes.js'
import { createDb } from './db.js'
import { createContentStore } from './content-store.js'
import { createBenchReader } from './bench.js'
import { readUpload } from './uploads.js'
import { createRegistryReadiness } from './registry-runtime.js'
import { createWeighService } from './weigh.js'
import { releaseRoutes } from './releases.js'
import { publicConfig } from './public-config.js'
import { contentResponse } from './content-response.js'

function summary(record: StoredRecord & { _count: { flags: number } }): s.RecordSummary {
  return {
    address: record.address, program: { address: record.program, name: record.programName },
    attester: record.attester, source: { repository: record.sourceRepository, commit: record.sourceCommit },
    toolchainVersion: record.toolchainVersion, stockBuildHash: record.stockBuildHash, tareBuildHash: record.tareBuildHash,
    corpus: { hash: record.corpusHash, counts: { tests: Number(record.testsCount), generated: Number(record.generatedCount), mainnet: Number(record.mainnetCount) } },
    gross: Number(record.gross), net: Number(record.net), tare: Number(record.gross - record.net),
    reportHash: record.reportHash, reportLocation: record.reportLocation, slot: Number(record.slot),
    createdAt: record.createdAt.toISOString(), flags: record._count.flags,
  }
}

export function buildApp() {
  const app = createApi({
    origins: env.CORS_ORIGINS, allowHeaders: ['X-Fixture'], contractVersion: s.CONTRACT_VERSION,
    maxBodyBytes: 1024 * 1024,
  })
  app.use('*', writeLimit(12, ['POST']))
  releaseRoutes(app)
  if (env.MODE === 'fixture') {
    app.route('/', fixtureRoutes())
    return { app, start: async () => {}, close: async () => {} }
  }

  const database = createDb(env.DATABASE_URL!)
  const { db } = database
  const content = createContentStore(db)
  const registry = createRegistryReadiness(database)
  const weigh = createWeighService(db)
  const routes = new Hono()
  routes.get('/health', (c) => json(c, { status: registry.status() }))
  routes.get('/v1/config', (c) => json(c, publicConfig()))
  const bench = createBenchReader(db)
  routes.get('/v1/bench', async (c) => json(c, await bench.results()))
  routes.get('/v1/bench/totals', async (c) => json(c, await bench.totals()))
  routes.get('/v1/compat', async (c) => json(c, await bench.compatibility()))
  routes.get('/v1/registry/totals', async (c) => {
    if (registry.status() !== 'ready') return json(c, emptyRegistryTotals)
    const [totals, programs, flags] = await Promise.all([
      db.record.aggregate({ _count: true, _sum: { gross: true, net: true, testsCount: true, generatedCount: true, mainnetCount: true, feeLamports: true } }),
      db.record.groupBy({ by: ['program'] }), db.flag.count(),
    ])
    const feesAccruedLamports = Number(totals._sum.feeLamports ?? 0n)
    return json(c, { ...emptyRegistryTotals, records: totals._count, programs: programs.length, flags,
      gross: Number(totals._sum.gross ?? 0n), net: Number(totals._sum.net ?? 0n),
      tare: Number((totals._sum.gross ?? 0n) - (totals._sum.net ?? 0n)),
      corpus: Number(totals._sum.testsCount ?? 0) + Number(totals._sum.generatedCount ?? 0) + Number(totals._sum.mainnetCount ?? 0), feesAccruedLamports })
  })
  routes.get('/v1/records', async (c) => {
    const q = readQuery(c, s.RecordsQuery)
    if (registry.status() !== 'ready') return json(c, { items: [], page: { nextCursor: null, hasMore: false }, asOf: new Date().toISOString() })
    const cursor = q.cursor ? await db.record.findUnique({ where: { address: q.cursor } }) : null
    if (q.cursor && (!cursor || (q.program && cursor.program !== q.program))) fail('VALIDATION_ERROR', 'The record cursor is unknown.', { stage: 'validation' })
    const rows = await db.record.findMany({
      where: { ...(q.program ? { program: q.program } : {}), ...(cursor ? { OR: [{ slot: { lt: cursor.slot } }, { slot: cursor.slot, address: { lt: cursor.address } }] } : {}) },
      orderBy: [{ slot: 'desc' }, { address: 'desc' }], take: q.limit + 1, include: { _count: { select: { flags: true } } },
    })
    const hasMore = rows.length > q.limit
    const items = rows.slice(0, q.limit).map(summary)
    return json(c, { items, page: { nextCursor: hasMore ? items.at(-1)!.address : null, hasMore }, asOf: new Date().toISOString() })
  })
  routes.get('/v1/records/:record', async (c) => {
    const address = readParam(c, 'record', s.Address)
    if (registry.status() !== 'ready') fail('RECORD_NOT_FOUND', 'This registry record was not found.')
    const row = await db.record.findUnique({ where: { address }, include: { _count: { select: { flags: true } }, flags: { orderBy: [{ slot: 'asc' }, { address: 'asc' }] } } })
    if (!row) fail('RECORD_NOT_FOUND', 'This registry record was not found.')
    const flags = await Promise.all(row.flags.map(async (flag) => {
      let input: s.FlagInput | null = null
      try { input = await content.get('flag_input', flag.inputHash, flag.inputLocation) as s.FlagInput }
      catch (error) { if (!(error instanceof ApiFailure) || error.body.error.code !== 'FLAG_INPUT_UNAVAILABLE') throw error }
      return { address: flag.address, record: address, flagger: flag.flagger, inputHash: flag.inputHash, inputLocation: flag.inputLocation,
        slot: Number(flag.slot), createdAt: flag.createdAt.toISOString(), input }
    }))
    const base = { record: summary(row), flags, checkCommand: `tare check ${address}` }
    try {
      const report = await content.get('report', row.reportHash, row.reportLocation) as s.ProofReport
      return json(c, { ...base, record: { ...base.record, program: { ...base.record.program, name: base.record.program.name ?? report.program.name } }, status: 'ready', report })
    }
    catch (error) {
      if (!(error instanceof ApiFailure) || error.body.error.code !== 'REPORT_UNAVAILABLE') throw error
      return json(c, { ...base, status: 'report_unavailable', report: null })
    }
  })
  routes.get('/v1/weigh', async (c) => {
    const q = readQuery(c, s.RecentQuery)
    return json(c, { items: await weigh.recent(q.limit), asOf: new Date().toISOString() })
  })
  routes.get('/v1/weigh/:program', async (c) => {
    const program = readParam(c, 'program', s.Address)
    const result = await weigh.get(program)
    const records = registry.status() === 'ready' ? await db.record.findMany({
      where: { program }, orderBy: [{ slot: 'desc' }, { address: 'desc' }], include: { _count: { select: { flags: true } } },
    }) : []
    if (result.status === 'sampling') c.header('Retry-After', String(result.retryAfterSeconds))
    return json(c, { ...result, records: records.map(summary) })
  })
  routes.post('/v1/reports', async (c) => {
    const { bytes, value } = await readUpload(c, s.UploadProofReport)
    return json(c, await content.put('report', value, bytes), 201)
  })
  routes.get('/v1/reports/:hash', async (c) => contentResponse(c, 'report', await content.getBytes('report', readParam(c, 'hash', s.Hash))))
  routes.post('/v1/flag-inputs', async (c) => {
    const { bytes, value } = await readUpload(c, s.UploadFlagInput)
    return json(c, await content.put('flag_input', value, bytes), 201)
  })
  routes.get('/v1/flag-inputs/:hash', async (c) => contentResponse(c, 'flag_input', await content.getBytes('flag_input', readParam(c, 'hash', s.Hash))))
  routes.get('/v1/token', async (c) => {
    const [fees, spent, burns] = await Promise.all([
      db.record.aggregate({ _sum: { feeLamports: true } }),
      db.burn.aggregate({ _sum: { spentLamports: true, burned: true } }),
      db.burn.findMany({ select: { signature: true, kind: true, spentLamports: true, burned: true, decimals: true, slot: true, at: true }, orderBy: { slot: 'desc' } }),
    ])
    const tokens = (raw: bigint, decimals: number) => Number(raw) / 10 ** decimals
    return json(c, {
      ...fixtureToken, mint: env.TOKEN_MINT ?? null, venue: env.TOKEN_VENUE ?? null,
      developerAllocationShare: env.TOKEN_DEVELOPER_SHARE ?? null, developerAllocationLock: env.TOKEN_DEVELOPER_LOCK ?? null,
      feesReceivedLamports: Number(fees._sum.feeLamports ?? 0n), spentLamports: Number(spent._sum.spentLamports ?? 0n),
      burned: burns.length ? tokens(spent._sum.burned ?? 0n, burns[0]!.decimals) : 0,
      burns: burns.slice(0, 20).map((burn) => ({ signature: burn.signature, kind: burn.kind, spentLamports: Number(burn.spentLamports),
        burned: tokens(burn.burned, burn.decimals), slot: Number(burn.slot), at: burn.at.toISOString() })),
    })
  })
  app.route('/', routes)
  app.route('/fixture', fixtureRoutes())
  return { app, start: () => registry.start(), close: async () => {
    await registry.stop()
    await weigh.close()
    await database.close()
  } }
}
