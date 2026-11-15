import { readFile } from 'node:fs/promises'
import type { Idl } from '@coral-xyz/anchor'
import { z } from 'zod'
import { Address, Base64 } from '@tare/contract'
import { env } from './config.js'
import type { Database } from './db.js'
import { anchorProgram, type AnchorProgram } from './engines/indexer/anchor.js'
import { createRpc, type Rpc } from './engines/rpc/client.js'
import { createReadiness, type Readiness } from './engines/server/readiness.js'
import { readinessKey } from './engines/server/readiness.js'
import { registryPdas } from './registry-decoder.js'

const idlShape = z.object({ address: Address, instructions: z.array(z.unknown()),
  metadata: z.object({ name: z.string(), version: z.string(), spec: z.string() }) }).passthrough()
const account = z.object({ owner: Address, executable: z.boolean(), data: z.tuple([Base64, z.literal('base64')]) }).nullable()
const accountSet = z.object({ value: z.tuple([account, account]) })

/** Missing IDL/program configuration costs no network calls; all roles share the deployed probe. */
export function createRegistryReadiness(database: Database, options: { onReady?: () => Promise<void> } = {}) {
  let program: AnchorProgram | null = null
  let rpc: Rpc | null = null
  let readiness: Readiness | null = null
  let timer: ReturnType<typeof setInterval> | undefined
  let checking: Promise<void> | undefined
  let transition: Promise<void> | undefined
  let stopped = false

  async function load() {
    if (program || !env.REGISTRY_PROGRAM_ID || stopped) return
    let source: string
    try {
      source = await readFile(env.REGISTRY_IDL_PATH, 'utf8')
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === 'ENOENT') return
      throw error
    }
    const idl = idlShape.parse(JSON.parse(source))
    if (idl.address !== env.REGISTRY_PROGRAM_ID) throw new Error('Registry IDL address must match REGISTRY_PROGRAM_ID.')
    if (!env.SOLANA_RPC_URL) throw new Error('SOLANA_RPC_URL is required when the registry IDL and program id are available.')
    program = anchorProgram(idl as Idl)
    rpc = createRpc({ url: env.SOLANA_RPC_URL, service: 'tare-registry', rps: env.REGISTRY_RPC_RPS,
      scanIntervalMs: 30_000, commitment: env.REGISTRY_COMMITMENT })
    const pdas = registryPdas(env.REGISTRY_PROGRAM_ID)
    readiness = createReadiness({ pool: database.pool,
      key: readinessKey(env.SOLANA_RPC_URL, [env.REGISTRY_COMMITMENT, env.REGISTRY_PROGRAM_ID, pdas.config]),
      probe: async () => {
        const [deployed, config] = accountSet.parse(await rpc!.call('getMultipleAccounts', [
          [env.REGISTRY_PROGRAM_ID, pdas.config], { commitment: env.REGISTRY_COMMITMENT, encoding: 'base64' },
        ])).value
        if (!deployed?.executable || !config || config.owner !== env.REGISTRY_PROGRAM_ID) return false
        program!.account('Config', Buffer.from(config.data[0], 'base64'))
        return true
      },
    })
    clearInterval(timer)
    await readiness.start()
    transition = readiness.waitUntilReady().then(async () => {
      if (!stopped) await options.onReady?.()
    }).catch((error) => {
      if (!stopped) console.error(JSON.stringify({ event: 'registry_start_failed', name: error instanceof Error ? error.name : 'Error' }))
    })
  }

  return {
    status: () => readiness?.status() ?? 'waiting' as const,
    program: () => program,
    rpc: () => rpc,
    pdas: () => env.REGISTRY_PROGRAM_ID ? registryPdas(env.REGISTRY_PROGRAM_ID) : null,
    async start() {
      await load()
      if (!program && env.REGISTRY_PROGRAM_ID) timer = setInterval(() => {
        checking ??= load().catch((error) => console.error(JSON.stringify({ event: 'registry_setup_failed',
          name: error instanceof Error ? error.name : 'Error' }))).finally(() => { checking = undefined })
      }, 30_000)
    },
    async stop() {
      stopped = true
      clearInterval(timer)
      await checking
      await readiness?.stop()
      await transition
    },
  }
}

export type RegistryReadiness = ReturnType<typeof createRegistryReadiness>
