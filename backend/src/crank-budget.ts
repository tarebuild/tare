// What the crank may spend from a wallet that also holds its owner's SOL: only what fees account for.
// Attest fees received, less what earlier attest buys spent, pay one clip at a time; a trading claim
// pays its burn share of what that claim brought in. The reserve stays a floor under both.

import type { Fraction } from './trading-share.js'

export function attestSpend(args: { balance: bigint; feesReceived: bigint; attestSpent: bigint; clip: bigint; reserve: bigint }): bigint {
  const owed = args.feesReceived - args.attestSpent
  return owed >= args.clip && args.balance >= args.reserve + args.clip ? args.clip : 0n
}

export function tradingBurnShare(args: { claimDelta: bigint; fraction: Fraction }): bigint {
  return args.claimDelta > 0n ? args.claimDelta * args.fraction.numerator / args.fraction.denominator : 0n
}
