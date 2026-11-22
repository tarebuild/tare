import { z } from 'zod'
import { address, host, httpUrl, origins, parseEnv, port } from './engines/server/config.js'

const localOrigins = 'http://127.0.0.1:7800,http://localhost:7800,http://127.0.0.1:7801,http://localhost:7801'
const configured = parseEnv(z.object({
  MODE: z.enum(['fixture', 'chain']).default('fixture'),
  PORT: port(7810), HOST: host(), DATABASE_URL: z.string().optional(),
  CORS_ORIGINS: origins.prefault(localOrigins),
  CLUSTER: z.enum(['localnet', 'devnet', 'mainnet']).default('localnet'),
  REGISTRY_PROGRAM_ID: address.optional(),
  REGISTRY_IDL_PATH: z.string().default('registry/target/idl/tare_registry.json'),
  RPC_URL: httpUrl.optional(),
  SOLANA_RPC_URL: httpUrl.optional(),
  REGISTRY_COMMITMENT: z.enum(['confirmed', 'finalized']).optional(),
  INDEXER_POLL_MS: z.coerce.number().int().min(1000).optional(),
  REGISTRY_RPC_RPS: z.coerce.number().positive().max(50).optional(),
  HELIUS_RPC_URL: httpUrl.optional(),
  SITE_PUBLIC_URL: httpUrl.optional(),
  API_PUBLIC_URL: httpUrl.transform((value) => new URL(value).origin).optional(),
  RELEASES_BASE_URL: httpUrl.transform((value) => value.replace(/\/+$/, '')).optional(),
  WEIGH_SAMPLE_SIZE: z.coerce.number().int().min(1).max(100).default(50),
  WEIGH_CACHE_SECONDS: z.coerce.number().int().min(1).default(900),
  PINATA_JWT: z.string().optional(),
  PINATA_GATEWAY: httpUrl.optional(),
  TOKEN_MINT: address.optional(),
  TOKEN_VENUE: httpUrl.optional(),
  TOKEN_DEVELOPER_SHARE: z.coerce.number().min(0).max(1).optional(),
  TOKEN_DEVELOPER_LOCK: address.optional(),
  // Trading fees: the DBC pool whose creator side the crank's wallet claims, the slice of volume burned
  // (basis points), and the smallest claim worth a transaction. The rest of each claim stays in the wallet.
  TRADING_FEE_POOL: address.optional(),
  CRANK_TRADING_BURN_BPS: z.coerce.number().int().min(0).max(10_000).default(50),
  CRANK_TRADING_MIN_LAMPORTS: z.coerce.bigint().positive().default(50_000_000n),
  // The crank. The clip and the interval are the user's figures; the crank waits while either is unset.
  CRANK_WALLET: address.optional(),
  CRANK_WALLET_SECRET_KEY: z.string().optional(),
  CRANK_CLIP_LAMPORTS: z.coerce.bigint().positive().optional(),
  CRANK_INTERVAL_SECONDS: z.coerce.number().int().min(60).optional(),
  CRANK_RESERVE_LAMPORTS: z.coerce.bigint().nonnegative().default(10_000_000n),
  CRANK_SLIPPAGE_BPS: z.coerce.number().int().min(1).max(1000).default(100),
  CRANK_MICRO_LAMPORTS: z.coerce.bigint().nonnegative().default(50_000n),
  JUPITER_API_KEY: z.string().optional(),
}).superRefine((value, ctx) => {
  if (value.MODE === 'chain' && !value.DATABASE_URL)
    ctx.addIssue({ code: 'custom', path: ['DATABASE_URL'], message: 'DATABASE_URL is required in chain mode.' })
  if (value.PINATA_JWT && !value.PINATA_GATEWAY)
    ctx.addIssue({ code: 'custom', path: ['PINATA_GATEWAY'], message: 'PINATA_GATEWAY is required when PINATA_JWT is set.' })
}))
const local = configured.CLUSTER === 'localnet'
export const env = {
  ...configured,
  SOLANA_RPC_URL: configured.RPC_URL ?? configured.SOLANA_RPC_URL,
  REGISTRY_COMMITMENT: configured.REGISTRY_COMMITMENT ?? (local ? 'confirmed' : 'finalized'),
  INDEXER_POLL_MS: configured.INDEXER_POLL_MS ?? (local ? 2000 : 15000),
  REGISTRY_RPC_RPS: configured.REGISTRY_RPC_RPS ?? (local ? 50 : 5),
}
