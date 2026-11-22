import assert from 'node:assert/strict'
import { test } from 'node:test'
import { attestSpend, tradingBurnShare } from './crank-budget.js'

const clip = 50_000_000n, reserve = 10_000_000n

test('a wallet holding its own SOL buys nothing without attest fees to account for it', () => {
  assert.equal(attestSpend({ balance: 14_000_000_000n, feesReceived: 0n, attestSpent: 0n, clip, reserve }), 0n)
})

test('attest fees received since the last buys pay one clip, and only once', () => {
  assert.equal(attestSpend({ balance: 14_000_000_000n, feesReceived: 100_000_000n, attestSpent: 0n, clip, reserve }), clip)
  assert.equal(attestSpend({ balance: 14_000_000_000n, feesReceived: 100_000_000n, attestSpent: 51_488_440n, clip, reserve }), 0n)
  assert.equal(attestSpend({ balance: 14_000_000_000n, feesReceived: 150_000_000n, attestSpent: 51_488_440n, clip, reserve }), clip)
})

test('the reserve still holds when fees are owed but the wallet runs low', () => {
  assert.equal(attestSpend({ balance: 55_000_000n, feesReceived: 100_000_000n, attestSpent: 0n, clip, reserve }), 0n)
})

test('a trading claim buys only its burn share of what the claim itself brought in', () => {
  const f = { numerator: 5n, denominator: 28n }
  assert.equal(tradingBurnShare({ claimDelta: 2_800_000_000n, fraction: f }), 500_000_000n)
  assert.equal(tradingBurnShare({ claimDelta: -5_000n, fraction: f }), 0n)
})
