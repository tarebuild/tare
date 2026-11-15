import type { Prisma } from '@prisma/client'
import { env } from './config.js'
import { createDb } from './db.js'
import { createApi } from './engines/server/app.js'
import { serve } from './engines/server/serve.js'
import { CONTRACT_VERSION } from '@tare/contract'
import { Indexer } from './engines/indexer/indexer.js'
import { PrismaIndexStore } from './engines/indexer/prisma-store.js'
import { TableLease } from './engines/indexer/table-lease.js'
import { runWorker } from './engines/indexer/worker.js'
import { createRegistryReadiness } from './registry-runtime.js'
import { registryFacts, type RegistryFacts } from './registry-decoder.js'
import { applyRegistry } from './registry.js'

const app = createApi({ origins: env.CORS_ORIGINS, contractVersion: CONTRACT_VERSION })
const database = env.MODE === 'chain' ? createDb(env.DATABASE_URL!) : null
let worker: ReturnType<typeof runWorker> | undefined
const readiness = database ? createRegistryReadiness(database, { onReady: async () => {
  const program = readiness!.program()!
  const rpc = readiness!.rpc()!
  const indexer = new Indexer<RegistryFacts, Prisma.TransactionClient>({
    rpc, programs: [program], addresses: [program.programId],
    store: new PrismaIndexStore<RegistryFacts, Prisma.TransactionClient>(database.db, { lock: 'tare:registry:apply' }),
    facts: registryFacts, apply: applyRegistry, pollMs: env.INDEXER_POLL_MS, commitment: env.REGISTRY_COMMITMENT, concurrency: 5,
    log: (entry) => console.log(JSON.stringify(entry)),
  })
  console.log(JSON.stringify({ event: 'registry_worker', state: 'standby' }))
  worker = runWorker({ lease: new TableLease(database.db, 'tare:registry:indexer'), job: {
    async start() {
      await indexer.start()
      console.log(JSON.stringify({ event: 'registry_worker', state: 'active' }))
    },
    async stop() {
      await indexer.stop()
      console.log(JSON.stringify({ event: 'registry_worker', state: 'standby' }))
    },
  } })
} }) : null
app.get('/health', (c) => c.json({ status: readiness?.status() ?? 'fixture' }))
await readiness?.start()
const { server } = serve(app, { port: env.PORT, host: env.HOST, close: async () => {
  await readiness?.stop()
  await worker?.stop()
  await database?.close()
} })
server.keepAliveTimeout = 65_000
server.headersTimeout = 66_000
