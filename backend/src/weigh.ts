import { ZodError } from 'zod'
import type { Prisma } from '@prisma/client'
import { WeighResponse } from '@tare/contract'
import { env } from './config.js'
import type { Database } from './db.js'
import { fail } from './errors.js'
import { createRpc, RpcError } from './engines/rpc/client.js'
import { sampleProgram, type SampleResult } from './sampler.js'

const QUIET_CACHE_FACTOR = 8

export function createWeighService(db: Database['db']) {
  const pending = new Map<string, { startedAt: string; done: Promise<void> }>()
  const failures = new Map<string, unknown>()
  let rpc: ReturnType<typeof createRpc> | undefined

  return {
    async get(program: string): Promise<WeighResponse> {
      const cached = await db.weighing.findUnique({ where: { program } })
      const stored = cached && WeighResponse.parse(cached.payload)
      // A program with nothing to measure changes slowly; it is sampled again less often.
      const ageLimit = stored && stored.status !== 'ready' ? QUIET_CACHE_FACTOR * env.WEIGH_CACHE_SECONDS : env.WEIGH_CACHE_SECONDS
      if (cached && stored && Date.now() - cached.sampledAt.getTime() < ageLimit * 1000) {
        // Earlier samples used a single signature page, and earlier empty samples did not look at
        // failed calls; both are sampled again.
        if (!('sample' in stored)) return stored
        if (stored.sample.signaturesExamined !== undefined && (stored.status === 'ready' || stored.sample.signaturesFailed !== undefined)) return stored
      }
      if (failures.has(program)) {
        const error = failures.get(program)
        failures.delete(program)
        if (error instanceof RpcError) fail('RPC_UNAVAILABLE', 'The RPC could not complete this sample.', { retryable: true })
        throw error
      }
      let work = pending.get(program)
      if (!work) {
        if (!env.HELIUS_RPC_URL) fail('CONFIGURATION_REQUIRED', 'HELIUS_RPC_URL is required to sample a program.')
        rpc ??= createRpc({ url: env.HELIUS_RPC_URL, service: 'tare-weigh', rps: 5, scanIntervalMs: 10_000 })
        const startedAt = new Date().toISOString()
        const done = sampleProgram(rpc, program, env.WEIGH_SAMPLE_SIZE).catch((error: unknown) => {
          const code = (error as NodeJS.ErrnoException)?.code
          if (error instanceof ZodError || error instanceof SyntaxError || code === 'Z_DATA_ERROR' || code === 'Z_BUF_ERROR' || code === 'ERR_BUFFER_TOO_LARGE')
            throw new RpcError('sample', -1, 'The RPC returned invalid sampling data.')
          throw error
        }).then(async ({ result }) => {
          await db.weighing.upsert({
            where: { program },
            create: { program, sampledAt: new Date(), payload: result as Prisma.InputJsonValue },
            update: { sampledAt: new Date(), payload: result as Prisma.InputJsonValue },
          })
        }).catch((error: unknown) => { failures.set(program, error) }).finally(() => { pending.delete(program) })
        work = { startedAt, done }
        pending.set(program, work)
      }
      return { status: 'sampling', program, startedAt: work.startedAt, retryAfterSeconds: 2, records: [] }
    },
    async recent(limit: number) {
      const rows = await db.weighing.findMany({ orderBy: { sampledAt: 'desc' }, take: limit })
      return rows.flatMap((row) => {
        const result = WeighResponse.parse(row.payload) as SampleResult
        return 'sample' in result ? [result.sample] : []
      })
    },
    async close() { await Promise.all([...pending.values()].map((work) => work.done)) },
  }
}
