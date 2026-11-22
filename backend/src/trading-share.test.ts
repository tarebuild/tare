import assert from 'node:assert/strict'
import { test } from 'node:test'
import { burnFraction, splitClaim } from './trading-share.js'

test('0.5% of volume out of a 3.5% fee whose claimable side keeps 80% is 5/28 of what is claimed', () => {
  const f = burnFraction({ burnBpsOfVolume: 50, tradingFeeBps: 350, protocolFeePercent: 20 })
  assert.equal(f.numerator * 28n, f.denominator * 5n)
})

test('the split burns the floor of the fraction and keeps every rounding unit for the claimer', () => {
  const f = burnFraction({ burnBpsOfVolume: 50, tradingFeeBps: 350, protocolFeePercent: 20 })
  assert.deepEqual(splitClaim({ quote: 2_800_000_000n, base: 28_000_001n }, f), { burnQuote: 500_000_000n, keepQuote: 2_300_000_000n, burnBase: 5_000_000n, keepBase: 23_000_001n })
  assert.deepEqual(splitClaim({ quote: 27n, base: 0n }, f), { burnQuote: 4n, keepQuote: 23n, burnBase: 0n, keepBase: 0n })
})

test('the fee rate read from a DBC config numerator', () => {
  const f = burnFraction({ burnBpsOfVolume: 50, tradingFeeBps: 35_000_000 * 10_000 / 1_000_000_000, protocolFeePercent: 20 })
  assert.equal(f.numerator * 28n, f.denominator * 5n)
})
