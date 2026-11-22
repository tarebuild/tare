// The one module every surface takes Tare's names, numbers and labels from.
// Sources: BRIEF.md and PLAN.md for what the product is; backend/bench/results.json
// (synced to lib/bench.json by `npm run sync`) for every number about Tare's effect.

import { bench } from './bench'

export const NAME = 'Tare'
export const TICKER = 'TARE'

/** The product's one line, the hero's. */
export const TAGLINE = 'Weigh less.'

/** The pitch, one sentence. */
export const PITCH =
  'anchor build becomes tare build, and the program you already wrote comes out at hand-written cost. Not a line changed.'

export const DESCRIPTION =
  'Tare is a drop-in build tool for Anchor programs. The same source comes out at a fraction of the compute units and binary size, with a proof that both builds behave identically and a public record of it on chain.'

/** Site origin. Set NEXT_PUBLIC_SITE_URL on the deployed service. */
export const SITE_URL = process.env.NEXT_PUBLIC_SITE_URL ?? 'http://127.0.0.1:7801'

/** The three weights, the product's own numbers. */
export const WEIGHTS = [
  { key: 'gross', label: 'Gross', says: 'What an instruction burns when built with stock Anchor.' },
  { key: 'tare', label: 'Tare', says: 'The difference: the weight of the framework.' },
  { key: 'net', label: 'Net', says: 'What it burns built with Tare.' },
] as const

/** The four commands, in the order a team runs them. */
export const COMMANDS = [
  { cmd: 'tare build', does: 'The same workspace, the same source.', writes: 'target/tare/<program>.so' },
  { cmd: 'tare weigh', does: 'Gross, tare and net for every instruction.', writes: 'target/tare/weigh.json' },
  { cmd: 'tare prove', does: 'The stock build and the Tare build, run side by side.', writes: 'target/tare/proof.json' },
  { cmd: 'tare attest', does: 'The result, on chain, for anyone to check.', writes: 'a record in the registry' },
] as const

export const MORE_COMMANDS = [
  { cmd: 'tare check <record>', does: 'Rebuild and rerun a record from the record alone.' },
  { cmd: 'tare flag <record> --input <file>', does: 'File an input on which the two builds differ.' },
] as const

/** What is identical between the stock build and the Tare build. */
export const IDENTICAL = [
  'Account layouts',
  'Discriminators',
  'The IDL',
  'Error codes and messages',
  'Events',
  'Logs',
  'Return data',
] as const

/** The three corpora `tare prove` runs on both builds. */
export const CORPORA = [
  { key: 'tests', label: 'Own tests', says: "The program’s own test transactions." },
  { key: 'generated', label: 'Generated', says: 'Seeded instruction sequences generated from the IDL.' },
  { key: 'mainnet', label: 'Mainnet', says: 'Transactions sampled from mainnet for the program address, replayed over current account state.' },
] as const

/** What is compared after every transaction. */
export const COMPARED = ['Account bytes', 'Lamports', 'Logs', 'Return data', 'Errors'] as const

/** Anchor versions the build supports first. */
export const ANCHOR_VERSIONS = ['0.31', '0.32', '1.x'] as const

/** Compute limits a weighing is read against (Solana docs, Compute budget). */
export const LIMITS = {
  transactionUnits: 1_400_000,
  accountUnitsPerBlock: 12_000_000,
} as const

/**
 * The token, told in full on /docs/token and nowhere else.
 * `mint` and `venue` stay blank until they exist; the page shows them once set.
 */
export const TOKEN = {
  ticker: 'TARE',
  supply: 1_000_000_000,
  launch: 'Fair launch',
  allocation:
    "The whole supply goes to the public launch. The developer’s allocation is the developer’s own first buy in it, on the same terms as every buyer.",
  fee: 'Attesting carries a fee in SOL, paid by the attester to the token’s launch wallet. A crank buys TARE on the market with it and burns it.',
  feeAsset: 'SOL',
  mint: process.env.NEXT_PUBLIC_TOKEN_MINT ?? '',
  venue: process.env.NEXT_PUBLIC_TOKEN_VENUE ?? '',
} as const

// The registry on mainnet (backend/registry/deployments/mainnet.json).
/** The tare_registry program. */
export const REGISTRY_PROGRAM_ID = '7viqDjx5k8ffnfnZiBv9dMhxzVhcGSYLMb5bKdDP1XtP'
/** Its Config account, the PDA of ["config"]. */
export const REGISTRY_CONFIG = 'Fvcic6U24RfK7CG9a4rKNaRubMgsQZN3NpywXEoYUQkD'
/** Config.fee_recipient: the TARE launch wallet every attest fee goes to. */
export const FEE_RECIPIENT = 'EvycjhbgcQDVN92xpPsdy6oLKezCcDiykg96g8ZXZDe9'

/** The X handle, with its @. Configuration point: set once the account exists. */
export const X_HANDLE = process.env.NEXT_PUBLIC_X_HANDLE ?? ''

/** Deployed mainnet programs offered as examples on /weigh. */
export const EXAMPLE_PROGRAMS = [
  { name: 'Whirlpools', address: 'whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc' },
  { name: 'Squads v4', address: 'SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf' },
  { name: 'Marinade', address: 'MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD' },
] as const

/** Outside places. */
export const LINKS = {
  solar: 'https://www.paradigm.xyz/writing/solar-100-faster-solidity',
  x: process.env.NEXT_PUBLIC_X_URL ?? '',
  source: 'https://github.com/tarebuild/tare',
} as const

/**
 * The install line: one `curl ... | sh` that fetches the prebuilt binary.
 * Configuration point: it names where the binary is published, set once that location exists.
 * Empty until then; the site then gives the build-from-source lines in /docs/install.
 */
export const INSTALL = process.env.NEXT_PUBLIC_INSTALL_LINE ?? ''

/** Every number about Tare's effect. Empty until backend's benchmark holds one. */
export const BENCH = bench
export { featured as BENCH_FEATURED, measured as BENCH_MEASURED } from './bench'
