// The crank: signs as the token's launch wallet, which receives every attest fee. On its interval
// it buys TARE through Jupiter with one clip of the wallet's SOL and burns what that swap delivered:
// the quote's minimum in the swap's own transaction, the rest right after. It waits, making no RPC
// calls, until the mint, the clip, the interval and the wallet's key are all set.

import { Connection, PublicKey, type AddressLookupTableAccount } from '@solana/web3.js'
import { NATIVE_MINT, createBurnCheckedInstruction, getAssociatedTokenAddressSync } from '@solana/spl-token'
import { CONTRACT_VERSION } from '@tare/contract'
import { burnFacts } from './burns.js'
import { createTradingFees } from './trading-fees.js'
import { env } from './config.js'
import { createDb } from './db.js'
import { createApi } from './engines/server/app.js'
import { serve } from './engines/server/serve.js'
import { TableLease } from './engines/indexer/table-lease.js'
import { createJournal } from './engines/keeper/journal.js'
import { prismaKeeperStore } from './engines/keeper/prisma-store.js'
import { createKeeperRuntime } from './engines/keeper/runtime.js'
import { createScheduler } from './engines/keeper/scheduler.js'
import { createJupiterClient } from './engines/jupiter/http.js'
import { quoteExactIn } from './engines/jupiter/quote.js'
import { swapInstructions } from './engines/jupiter/swap.js'
import { loadLookupTables } from './engines/tx/server/lookup-tables.js'
import { expectKey, keypairFromSecret } from './engines/tx/server/keys.js'

const log = (entry: Record<string, unknown>) => console.log(JSON.stringify({ service: 'tare-crank', ...entry }))
const database = createDb(env.DATABASE_URL!)
const signer = env.LAUNCH_WALLET_SECRET_KEY ? keypairFromSecret('LAUNCH_WALLET_SECRET_KEY', env.LAUNCH_WALLET_SECRET_KEY) : null
if (signer && env.LAUNCH_WALLET) expectKey('LAUNCH_WALLET_SECRET_KEY', signer, env.LAUNCH_WALLET)
const rpcUrl = env.SOLANA_RPC_URL ?? env.HELIUS_RPC_URL
const settings = signer && env.TOKEN_MINT && env.CRANK_CLIP_LAMPORTS && env.CRANK_INTERVAL_SECONDS && rpcUrl
  ? { signer, mint: new PublicKey(env.TOKEN_MINT), clip: env.CRANK_CLIP_LAMPORTS, intervalMs: env.CRANK_INTERVAL_SECONDS * 1000, rpcUrl }
  : null

let mintInfo: { tokenProgram: PublicKey; decimals: number } | null = null
let tables: AddressLookupTableAccount[] = []
const connection = settings ? new Connection(settings.rpcUrl, 'confirmed') : null
const lease = new TableLease(database.db, 'tare:crank')
const journal = settings && connection ? createJournal({
  connection, store: prismaKeeperStore(database.db), signer: settings.signer,
  lookupTables: async () => tables,
  microLamports: async () => env.CRANK_MICRO_LAMPORTS,
  isLeader: () => runtime.isLeader(),
}) : null
const jupiter = createJupiterClient({ apiKey: env.JUPITER_API_KEY })

/** The mint's token program and decimals, read once; the crank waits until the mint exists. */
async function gate(): Promise<'waiting' | 'ready'> {
  if (!settings || !connection) return 'waiting'
  if (mintInfo) return 'ready'
  const account = await connection.getAccountInfo(settings.mint)
  if (!account) return 'waiting'
  mintInfo = { tokenProgram: account.owner, decimals: account.data[44]! }
  log({ event: 'crank_ready', mint: settings.mint.toBase58(), wallet: settings.signer.publicKey.toBase58() })
  return 'ready'
}

/** Buys TARE with `lamports` through Jupiter and burns the quote's minimum in the same transaction. */
async function swapAndBurn(key: string, lamports: bigint) {
  const { signer, mint } = settings!
  const { tokenProgram, decimals } = mintInfo!
  const tokens = getAssociatedTokenAddressSync(mint, signer.publicKey, false, tokenProgram)
  const quote = await quoteExactIn(jupiter, { inputMint: NATIVE_MINT, outputMint: mint, amount: lamports, slippageBps: env.CRANK_SLIPPAGE_BPS, maxAccounts: 40 })
  const swap = await swapInstructions(jupiter, { quote, taker: signer.publicKey })
  tables = await loadLookupTables(connection!, swap.lookupTables)
  await journal!.execute(key, () => [...swap.setup, swap.swap, ...swap.cleanup,
    createBurnCheckedInstruction(tokens, mint, signer.publicKey, quote.minimumOut, decimals, [], tokenProgram)])
  log({ event: 'buyback', key, lamports: lamports.toString() })
}

async function buyback(now: number) {
  const { signer, mint, clip, intervalMs } = settings!
  const { tokenProgram, decimals } = mintInfo!
  const tokens = getAssociatedTokenAddressSync(mint, signer.publicKey, false, tokenProgram)
  const balance = BigInt(await connection!.getBalance(signer.publicKey, 'confirmed'))
  const key = `buyback:${mint.toBase58()}:${Math.floor(now / intervalMs)}`
  const existing = await database.db.keeperTransaction.findUnique({ where: { key } })
  if (balance >= env.CRANK_RESERVE_LAMPORTS + clip && (!existing || existing.state === 'expired')) await swapAndBurn(key, clip)
  // The rest of what each recent swap delivered: its own balance change, never the account's balance.
  const swaps = await database.db.keeperTransaction.findMany({ where: { state: 'confirmed', OR: [{ key: { startsWith: 'buyback:' } }, { key: { startsWith: 'feebuy:' } }] }, orderBy: { createdAt: 'desc' }, take: 5 })
  for (const row of swaps) {
    if (await database.db.keeperTransaction.findUnique({ where: { key: `burn:${row.signature}` } })) continue
    const tx = await connection!.getTransaction(row.signature, { commitment: 'confirmed', maxSupportedTransactionVersion: 0 })
    if (!tx?.meta) continue
    const owned = (list: typeof tx.meta.postTokenBalances) => BigInt(list?.find((entry) => entry.mint === mint.toBase58() && entry.owner === signer.publicKey.toBase58())?.uiTokenAmount.amount ?? '0')
    const rest = owned(tx.meta.postTokenBalances) - owned(tx.meta.preTokenBalances)
    if (rest > 0n) await journal!.execute(`burn:${row.signature}`, () => [createBurnCheckedInstruction(tokens, mint, signer.publicKey, rest, decimals, [], tokenProgram)])
  }
}

const trading = settings && env.TRADING_FEE_POOL ? { pool: new PublicKey(env.TRADING_FEE_POOL), recipient: new PublicKey(env.CRANK_TRADING_RECIPIENT) } : null
let tradingFees: ReturnType<typeof createTradingFees> | null = null

/**
 * The pool's creator fees, once per interval: claim them, send the claimer's share to the recipient
 * and burn the TARE side's burn share in one transaction; then buy and burn with what the claim left
 * in the wallet, which the claim's own balance change measures, so a restart resumes it.
 */
async function claimTradingFees(now: number) {
  if (!trading) return
  const { signer, mint, intervalMs } = settings!
  tradingFees ??= createTradingFees({ connection: connection!, pool: trading!.pool, wallet: signer.publicKey, mint, ...mintInfo!,
    recipient: trading!.recipient, burnBpsOfVolume: env.CRANK_TRADING_BURN_BPS, minLamports: env.CRANK_TRADING_MIN_LAMPORTS })
  const key = `fees:${mint.toBase58()}:${Math.floor(now / intervalMs)}`
  const existing = await database.db.keeperTransaction.findUnique({ where: { key } })
  if (!existing || existing.state === 'expired') {
    const claim = await tradingFees.plan()
    if ('waiting' in claim) log({ event: 'trading_fees_waiting', reason: claim.waiting })
    else {
      await journal!.execute(key, () => claim.instructions)
      log({ event: 'trading_fees_claimed', key, venue: claim.venue, quote: claim.claimed.quote.toString(), base: claim.claimed.base.toString(), burnQuote: claim.split.burnQuote.toString(), burnBase: claim.split.burnBase.toString() })
    }
  }
  const claims = await database.db.keeperTransaction.findMany({ where: { key: { startsWith: 'fees:' }, state: 'confirmed' }, orderBy: { createdAt: 'desc' }, take: 5 })
  for (const row of claims) {
    if (await database.db.keeperTransaction.findUnique({ where: { key: `feebuy:${row.signature}` } })) continue
    const tx = await connection!.getTransaction(row.signature, { commitment: 'confirmed', maxSupportedTransactionVersion: 0 })
    if (!tx?.meta) continue
    const index = tx.transaction.message.getAccountKeys({ accountKeysFromLookups: tx.meta.loadedAddresses }).staticAccountKeys.findIndex((k) => k.equals(signer.publicKey))
    const left = BigInt(tx.meta.postBalances[index]!) - BigInt(tx.meta.preBalances[index]!) + BigInt(tx.meta.fee)
    if (left > 0n) await swapAndBurn(`feebuy:${row.signature}`, left)
  }
}

/** Each confirmed crank transaction, read once from chain: the SOL the wallet spent and the TARE it burned. */
async function recordBurns() {
  const { signer, mint } = settings!
  const rows = await database.db.keeperTransaction.findMany({ where: { state: 'confirmed', OR: ['buyback:', 'burn:', 'fees:', 'feebuy:'].map((prefix) => ({ key: { startsWith: prefix } })) }, orderBy: { createdAt: 'asc' } })
  const bySignature = new Map(rows.map((row) => [row.signature, row.key]))
  const recorded = new Set((await database.db.burn.findMany({ select: { signature: true } })).map((row) => row.signature))
  for (const row of rows) {
    if (recorded.has(row.signature)) continue
    const tx = await connection!.getParsedTransaction(row.signature, { commitment: 'confirmed', maxSupportedTransactionVersion: 0 })
    if (!tx?.meta || tx.meta.err || tx.blockTime == null) continue
    const { spentLamports, burned, claimedLamports, keptLamports } = burnFacts(tx, signer.publicKey, mint, trading?.recipient)
    // A rest burn belongs to the swap it follows; trading-fee claims and their buys to the trading side.
    const origin = row.key.startsWith('burn:') ? bySignature.get(row.key.slice(5)) ?? '' : row.key
    await database.db.burn.create({ data: {
      signature: row.signature, key: row.key, kind: row.key.startsWith('fees:') ? 'claim' : row.key.startsWith('burn:') ? 'burn' : 'buy',
      source: origin.startsWith('fees:') || origin.startsWith('feebuy:') ? 'trading' : 'attest',
      claimSignature: row.key.startsWith('fees:') ? row.signature : origin.startsWith('feebuy:') ? origin.slice(7) : null,
      spentLamports: row.key.startsWith('fees:') ? 0n : spentLamports, burned, claimedLamports, keptLamports, decimals: mintInfo!.decimals, slot: BigInt(tx.slot), at: new Date(tx.blockTime * 1000),
    } })
    log({ event: 'burn_recorded', signature: row.signature, spentLamports: spentLamports.toString(), burned: burned.toString() })
  }
}

const scheduler = createScheduler([
  { name: 'resume', everyMs: 0, run: () => journal!.resume() },
  { name: 'trading-fees', everyMs: settings?.intervalMs ?? 0, run: claimTradingFees },
  { name: 'buyback', everyMs: settings?.intervalMs ?? 0, run: buyback },
  { name: 'burns', everyMs: 30_000, run: recordBurns },
], { onError: (task, error) => log({ event: 'crank_task_failed', task, message: error instanceof Error ? error.message : 'unknown' }) })

const runtime = createKeeperRuntime({
  lease: { acquire: () => lease.acquire(), renew: () => lease.acquire(), release: () => lease.release() },
  leaseMs: 15_000, tickMs: 5_000, gate, tick: () => scheduler.tick(), log,
})
if (!settings) log({ event: 'crank_waiting', missing: ['LAUNCH_WALLET_SECRET_KEY', 'TOKEN_MINT', 'CRANK_CLIP_LAMPORTS', 'CRANK_INTERVAL_SECONDS']
  .filter((name) => !process.env[name]) })
void runtime.run()

const app = createApi({ origins: env.CORS_ORIGINS, contractVersion: CONTRACT_VERSION })
app.get('/health', (c) => c.json(runtime.health))
const { server } = serve(app, { port: env.PORT, host: env.HOST, close: async () => {
  await runtime.stop()
  await database.close()
} })
server.keepAliveTimeout = 65_000
server.headersTimeout = 66_000
