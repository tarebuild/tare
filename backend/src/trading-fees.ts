// TARE's trading fees. The crank's wallet holds the pool's creator role, so it claims the creator's
// side of every trade: on the DBC curve until it migrates, then on its locked DAMM v2 position. The
// claimer's share simply stays in the wallet; the burn share (`trading-share.ts`) is burned: the
// TARE side in the claim's own transaction, the SOL side through the buy-and-burn after it.

import type { Connection, PublicKey, TransactionInstruction } from '@solana/web3.js'
import { createBurnCheckedInstruction, getAssociatedTokenAddressSync } from '@solana/spl-token'
import { PROTOCOL_FEE_PERCENT } from '@meteora-ag/dynamic-bonding-curve-sdk'
import { createDbc } from './engines/launchpad/meteora/dbc.js'
import { createDamm } from './engines/launchpad/meteora/damm.js'
import { creatorFeeAmounts, creatorFeeClaims } from './engines/launchpad/meteora/fees.js'
import { findMigration } from './engines/launchpad/meteora/migration.js'
import { burnFraction, splitClaim, type Fraction } from './trading-share.js'

export type TradingFeeClaim = {
  venue: 'dbc' | 'damm-v2'
  claimed: { quote: bigint; base: bigint }
  split: ReturnType<typeof splitClaim>
  instructions: TransactionInstruction[]
}

export function createTradingFees(args: {
  connection: Connection; pool: PublicKey; wallet: PublicKey; mint: PublicKey; tokenProgram: PublicKey; decimals: number
  burnBpsOfVolume: number; minLamports: bigint
}) {
  const dbc = createDbc(args.connection)
  const damm = createDamm(args.connection)
  let dammPool: PublicKey | undefined

  async function venue() {
    const snap = await dbc.snapshot(args.pool)
    if (!snap.pool.poolState.baseMint.equals(args.mint)) throw new Error('The trading fee pool is not the mint\'s.')
    if (snap.pool.poolState.isMigrated && !dammPool) dammPool = (await findMigration(dbc, damm, args.pool)).pool
    return snap
  }

  /** The share of each claim that is burned, from the venue's fee rate and Meteora's share read on chain. */
  async function fraction(): Promise<Fraction> {
    const snap = await venue()
    // The curve's fee from its config numerator (of 1e9); the graduated pool's from the config's migrated fee.
    const tradingFeeBps = dammPool ? snap.config.migratedPoolFeeBps : Number(snap.config.poolFees.baseFee.cliffFeeNumerator.toString()) * 10_000 / 1_000_000_000
    const protocolFeePercent = dammPool ? (await damm.pool(dammPool)).poolFees.protocolFeePercent : PROTOCOL_FEE_PERCENT
    return burnFraction({ burnBpsOfVolume: args.burnBpsOfVolume, tradingFeeBps, protocolFeePercent })
  }

  /** The next claim, or why there is none. */
  async function plan(): Promise<TradingFeeClaim | { waiting: string }> {
    const snap = await venue()
    const state = snap.pool.poolState
    // Before migration the creator role carries the fees; after it, the position NFT the creator held
    // then. Both move together (scripts/move-roles.ts creator), and the curve's claim needs the role.
    if (!state.creator.equals(args.wallet)) return { waiting: `pool creator is ${state.creator.toBase58()}` }
    const owed = await creatorFeeAmounts(dbc, damm, { dbcPool: args.pool, creator: args.wallet, dammPool })
    const claimed = { quote: owed.curve.quote + owed.positions.quote, base: owed.curve.base + owed.positions.base }
    if (claimed.quote < args.minLamports) return { waiting: `${claimed.quote} lamports claimable, below ${args.minLamports}` }
    const split = splitClaim(claimed, await fraction())
    const instructions = await creatorFeeClaims(dbc, damm, { creator: args.wallet, dbcPool: args.pool, dammPool, receiver: args.wallet })
    const tokens = getAssociatedTokenAddressSync(args.mint, args.wallet, false, args.tokenProgram)
    if (split.burnBase > 0n) instructions.push(createBurnCheckedInstruction(tokens, args.mint, args.wallet, split.burnBase, args.decimals, [], args.tokenProgram))
    return { venue: dammPool ? 'damm-v2' : 'dbc', claimed, split, instructions }
  }

  return { plan, fraction }
}
