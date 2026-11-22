// The share of claimed trading fees that equals a slice of the volume. The pool charges
// `tradingFeeBps` of every trade and the venue keeps `protocolFeePercent` of that fee, so the
// claimable side receives tradingFeeBps x (100 - protocolFeePercent)% of the volume; burning
// `burnBpsOfVolume` of the volume takes burnBpsOfVolume / that of each claim.

export type Fraction = { numerator: bigint; denominator: bigint }

export function burnFraction(args: { burnBpsOfVolume: number; tradingFeeBps: number; protocolFeePercent: number }): Fraction {
  return { numerator: BigInt(args.burnBpsOfVolume) * 100n, denominator: BigInt(args.tradingFeeBps) * BigInt(100 - args.protocolFeePercent) }
}

/** The burn side of a claim, rounded down; the claimer keeps the rest, rounding included. */
export function splitClaim(claim: { quote: bigint; base: bigint }, f: Fraction) {
  const burnQuote = claim.quote * f.numerator / f.denominator
  const burnBase = claim.base * f.numerator / f.denominator
  return { burnQuote, keepQuote: claim.quote - burnQuote, burnBase, keepBase: claim.base - burnBase }
}
