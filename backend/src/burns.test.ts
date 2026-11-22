import assert from 'node:assert/strict'
import { test } from 'node:test'
import { Keypair, type ParsedTransactionWithMeta } from '@solana/web3.js'
import { burnFacts } from './burns.js'

const wallet = Keypair.generate().publicKey
const mint = Keypair.generate().publicKey
const other = Keypair.generate().publicKey
const pool = Keypair.generate().publicKey
const burn = (type: string, amount: string, burned = mint) => ({ program: 'spl-token', programId: other, parsed: { type, info: type === 'burnChecked' ? { mint: burned.toBase58(), tokenAmount: { amount } } : { mint: burned.toBase58(), amount } } })
const tx = (pre: number, post: number, fee: number, outer: unknown[], inner: unknown[] = []) => ({
  slot: 1, blockTime: 1, transaction: { signatures: [], message: { accountKeys: [{ pubkey: wallet, signer: true, writable: true }, { pubkey: pool, signer: false, writable: true }], instructions: outer, recentBlockhash: '' } },
  meta: { err: null, fee, preBalances: [pre, 0], postBalances: [post, 0], innerInstructions: inner.length ? [{ index: 0, instructions: inner }] : [] },
}) as unknown as ParsedTransactionWithMeta

test('a buy: the clip spent, the fee left out, the quoted minimum burned', () => {
  const facts = burnFacts(tx(1_000_000_000, 949_985_000, 15_000, [{ programId: pool, accounts: [], data: '' }, burn('burnChecked', '410882034313')]), wallet, mint)
  assert.deepEqual(facts, { spentLamports: 50_000_000n, burned: 410_882_034_313n })
})

test('a rest burn: nothing spent, the burn counted, other mints and inner burns of other mints ignored', () => {
  const facts = burnFacts(tx(949_985_000, 949_980_000, 5_000, [burn('burn', '2031552108')], [burn('burn', '7', other)]), wallet, mint)
  assert.deepEqual(facts, { spentLamports: 0n, burned: 2_031_552_108n })
})

test('burns inside inner instructions count', () => {
  const facts = burnFacts(tx(10, 10, 0, [], [burn('burnChecked', '5'), burn('burn', '6')]), wallet, mint)
  assert.equal(facts.burned, 11n)
})
