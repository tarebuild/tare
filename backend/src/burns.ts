import type { ParsedTransactionWithMeta, PublicKey } from '@solana/web3.js'

/** What one crank transaction did: the SOL the wallet spent, its fee left out, and the TARE it burned in base units. */
export function burnFacts(tx: ParsedTransactionWithMeta, wallet: PublicKey, mint: PublicKey) {
  const meta = tx.meta!
  const index = tx.transaction.message.accountKeys.findIndex((key) => key.pubkey.equals(wallet))
  const spent = BigInt(meta.preBalances[index]!) - BigInt(meta.postBalances[index]!) - BigInt(meta.fee)
  const instructions = [...tx.transaction.message.instructions, ...(meta.innerInstructions ?? []).flatMap((inner) => inner.instructions)]
  let burned = 0n
  for (const ix of instructions) {
    if (!('parsed' in ix) || !['burn', 'burnChecked'].includes(ix.parsed?.type) || ix.parsed.info.mint !== mint.toBase58()) continue
    burned += BigInt(ix.parsed.info.amount ?? ix.parsed.info.tokenAmount.amount)
  }
  return { spentLamports: spent > 0n ? spent : 0n, burned }
}
