// TARE's trading fees. The launch wallet holds the pool's creator role, so it claims the creator's
// side of every trade: on the DBC curve until it migrates, then on its locked DAMM v2 position.
// Each claim is split once: the burn share (`trading-share.ts`) stays to be burned, the rest goes to
// the recipient in the same transaction, SOL by system transfer and TARE to its token account.

import { SystemProgram, type Connection, type PublicKey, type TransactionInstruction } from '@solana/web3.js'
import { createAssociatedTokenAccountIdempotentInstruction, createBurnCheckedInstruction, createTransferCheckedInstruction, getAssociatedTokenAddressSync } from '@solana/spl-token'
import { PROTOCOL_FEE_PERCENT } from '@meteora-ag/dynamic-bonding-curve-sdk'
import { createDbc } from './engines/launchpad/meteora/dbc.js'
import { createDamm } from './engines/launchpad/meteora/damm.js'
import { creatorFeeAmounts, creatorFeeClaims } from './engines/launchpad/meteora/fees.js'
import { findMigration } from './engines/launchpad/meteora/migration.js'
import { burnFraction, splitClaim } from './trading-share.js'

export type TradingFeeClaim = {
  venue: 'dbc' | 'damm-v2'
  claimed: { quote: bigint; base: bigint }
  split: ReturnType<typeof splitClaim>
  instructions: TransactionInstruction[]
}

export function createTradingFees(args: {
  connection: Connection; pool: PublicKey; wallet: PublicKey; mint: PublicKey; tokenProgram: PublicKey; decimals: number
  recipient: PublicKey; burnBpsOfVolume: number; minLamports: bigint
}) {
  const dbc = createDbc(args.connection)
  const damm = createDamm(args.connection)
  let dammPool: PublicKey | undefined

  /** The next claim, or why there is none. */
  async function plan(): Promise<TradingFeeClaim | { waiting: string }> {
    const snap = await dbc.snapshot(args.pool)
    const state = snap.pool.poolState
    if (!state.baseMint.equals(args.mint)) throw new Error('The trading fee pool is not the mint\'s.')
    if (state.isMigrated && !dammPool) dammPool = (await findMigration(dbc, damm, args.pool)).pool
    // Before migration the creator role carries the fees; after it, the position NFT the creator held then.
    if (!state.isMigrated && !state.creator.equals(args.wallet)) return { waiting: `pool creator is ${state.creator.toBase58()}` }
    const owed = await creatorFeeAmounts(dbc, damm, { dbcPool: args.pool, creator: args.wallet, dammPool })
    const claimed = { quote: owed.curve.quote + owed.positions.quote, base: owed.curve.base + owed.positions.base }
    if (claimed.quote < args.minLamports) return { waiting: `${claimed.quote} lamports claimable, below ${args.minLamports}` }

    // The curve's fee from its config numerator (of 1e9); the graduated pool's from the config's migrated fee.
    const tradingFeeBps = dammPool ? snap.config.migratedPoolFeeBps : Number(snap.config.poolFees.baseFee.cliffFeeNumerator.toString()) * 10_000 / 1_000_000_000
    const protocolFeePercent = dammPool ? (await damm.pool(dammPool)).poolFees.protocolFeePercent : PROTOCOL_FEE_PERCENT
    const split = splitClaim(claimed, burnFraction({ burnBpsOfVolume: args.burnBpsOfVolume, tradingFeeBps, protocolFeePercent }))

    const instructions = await creatorFeeClaims(dbc, damm, { creator: args.wallet, dbcPool: args.pool, dammPool, receiver: args.wallet })
    if (split.keepQuote > 0n) instructions.push(SystemProgram.transfer({ fromPubkey: args.wallet, toPubkey: args.recipient, lamports: split.keepQuote }))
    const tokens = getAssociatedTokenAddressSync(args.mint, args.wallet, false, args.tokenProgram)
    if (split.keepBase > 0n) {
      const to = getAssociatedTokenAddressSync(args.mint, args.recipient, true, args.tokenProgram)
      instructions.push(createAssociatedTokenAccountIdempotentInstruction(args.wallet, to, args.recipient, args.mint, args.tokenProgram),
        createTransferCheckedInstruction(tokens, args.mint, to, args.wallet, split.keepBase, args.decimals, [], args.tokenProgram))
    }
    if (split.burnBase > 0n) instructions.push(createBurnCheckedInstruction(tokens, args.mint, args.wallet, split.burnBase, args.decimals, [], args.tokenProgram))
    return { venue: dammPool ? 'damm-v2' : 'dbc', claimed, split, instructions }
  }

  return { plan }
}
