import type { ParsedTransactionWithMeta, PublicKey } from '@solana/web3.js'

/** What one crank transaction did: the SOL the wallet spent, its fee left out, and the TARE it burned in base units. */
export function burnFacts(tx: ParsedTransactionWithMeta, wallet: PublicKey, mint: PublicKey, recipient?: PublicKey) {
  const meta = tx.meta!
  const index = tx.transaction.message.accountKeys.findIndex((key) => key.pubkey.equals(wallet))
  const spent = BigInt(meta.preBalances[index]!) - BigInt(meta.postBalances[index]!) - BigInt(meta.fee)
  const instructions = [...tx.transaction.message.instructions, ...(meta.innerInstructions ?? []).flatMap((inner) => inner.instructions)]
  let burned = 0n
  for (const ix of instructions) {
    if (!('parsed' in ix) || !['burn', 'burnChecked'].includes(ix.parsed?.type) || ix.parsed.info.mint !== mint.toBase58()) continue
    burned += BigInt(ix.parsed.info.amount ?? ix.parsed.info.tokenAmount.amount)
  }
  // A trading-fee claim: what reached the wallet plus what it passed on to the recipient.
  let keptLamports = 0n
  for (const ix of instructions) {
    if (!recipient || !('parsed' in ix) || ix.program !== 'system' || ix.parsed?.type !== 'transfer') continue
    if (ix.parsed.info.source === wallet.toBase58() && ix.parsed.info.destination === recipient.toBase58()) keptLamports += BigInt(ix.parsed.info.lamports)
  }
  const claimedLamports = keptLamports > 0n ? keptLamports - spent : 0n
  return { spentLamports: spent > 0n ? spent : 0n, burned, claimedLamports, keptLamports }
}
