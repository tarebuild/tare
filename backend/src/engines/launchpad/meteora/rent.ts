// What a DBC launch spends on rent, from account sizes and Metaplex's create-fee formula, priced
// at the cluster's own rent each time: a test validator charges a different byte rate than
// mainnet, so no rent figure is ever cached.

import type { Connection } from '@solana/web3.js'

/** Account sizes in bytes, discriminators included (DBC 0.2.2, SPL Token, Metaplex). */
export const DBC_ACCOUNT_BYTES = { mint: 82, pool: 424, config: 1048, tokenAccount: 165, metadata: 679 } as const

/** Metaplex charges rent(1308) + 5440 lamports to create a metadata account. */
export const METAPLEX_CREATE_FEE = { bytes: 1308, lamports: 5440n } as const

export type LaunchRentOptions = {
  /** the launch creates its own DBC config too */
  config?: boolean
  /** token accounts beyond the pool's two vaults (the first buyer's ATAs) */
  tokenAccounts?: number
}

/** Lamports a launch leaves in rent and Metaplex's fee: mint, pool, vaults, metadata, plus the options. */
export async function launchRent(connection: Pick<Connection, 'getMinimumBalanceForRentExemption'>, options: LaunchRentOptions = {}): Promise<bigint> {
  const b = DBC_ACCOUNT_BYTES
  const sizes = [b.mint, b.pool, b.metadata, METAPLEX_CREATE_FEE.bytes, ...Array(2 + (options.tokenAccounts ?? 0)).fill(b.tokenAccount)]
  if (options.config) sizes.push(b.config)
  const rents = await Promise.all(sizes.map((size) => connection.getMinimumBalanceForRentExemption(size, 'confirmed')))
  return rents.reduce<bigint>((sum, r) => sum + BigInt(r), METAPLEX_CREATE_FEE.lamports)
}
