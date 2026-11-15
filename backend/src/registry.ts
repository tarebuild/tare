import type { Prisma } from '@prisma/client'
import type { StagedTransaction } from './engines/indexer/store.js'
import type { RegistryFacts } from './registry-decoder.js'

/** The engine calls this inside the same transaction that advances its cursor. */
export async function applyRegistry(tx: Prisma.TransactionClient, row: StagedTransaction<RegistryFacts>) {
  if (row.failed || !row.facts) return
  // Pruned block times use indexing time; ordering is always the on-chain slot and address.
  const createdAt = row.blockTime ?? new Date()
  for (const fact of row.facts.records) {
    const data = { ...fact, testsCount: BigInt(fact.testsCount), generatedCount: BigInt(fact.generatedCount), mainnetCount: BigInt(fact.mainnetCount),
      gross: BigInt(fact.gross), net: BigInt(fact.net), slot: BigInt(fact.slot), feeLamports: BigInt(fact.feeLamports),
      signature: row.signature, programName: null, createdAt }
    await tx.record.upsert({ where: { address: fact.address }, create: data, update: {} })
  }
  for (const fact of row.facts.flags) {
    const data = { ...fact, slot: BigInt(fact.slot), signature: row.signature, createdAt }
    await tx.flag.upsert({ where: { address: fact.address }, create: data, update: {} })
  }
}
