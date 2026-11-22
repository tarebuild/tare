// Hands TARE's DBC pool creator role from the deployer to the launch wallet, so the crank, signing
// as the launch wallet, claims the creator's trading fees (and receives the DAMM v2 position NFT at
// migration). The fees booked so far move with the role: the crank's first trading run claims them
// and burns their share like any other claim. Simulates by default; `--send` signs and sends.
//
//   npx tsx scripts/transfer-creator.ts --keypair <deployer.json> --url <rpc> [--send]

import fs from 'node:fs'
import { ComputeBudgetProgram, Connection, Keypair, PublicKey, TransactionMessage, VersionedTransaction } from '@solana/web3.js'
import { createDbc } from '../src/engines/launchpad/meteora/dbc.js'
import { createDamm } from '../src/engines/launchpad/meteora/damm.js'
import { creatorFeeAmounts } from '../src/engines/launchpad/meteora/fees.js'

const POOL = new PublicKey('CfqxKkatD95ZERPeSezC5MwVUGCa62fTZQ5x8r3m7kkX')
const LAUNCH_WALLET = new PublicKey('EvycjhbgcQDVN92xpPsdy6oLKezCcDiykg96g8ZXZDe9')

const arg = (name: string) => { const i = process.argv.indexOf(`--${name}`); return i > 0 ? process.argv[i + 1] : undefined }
const keypairPath = arg('keypair'), url = arg('url'), send = process.argv.includes('--send')
if (!keypairPath || !url) throw new Error('Use --keypair <deployer.json> --url <rpc> [--send].')
const deployer = Keypair.fromSecretKey(Uint8Array.from(JSON.parse(fs.readFileSync(keypairPath, 'utf8'))))
const connection = new Connection(url, 'confirmed')
const dbc = createDbc(connection)
const damm = createDamm(connection)

const snap = await dbc.snapshot(POOL)
const creator = snap.pool.poolState.creator
if (creator.equals(LAUNCH_WALLET)) { console.log('The launch wallet is already the pool creator.'); process.exit(0) }
if (!creator.equals(deployer.publicKey)) throw new Error(`The pool creator is ${creator.toBase58()}, not this keypair.`)
if (snap.pool.poolState.isMigrated) throw new Error('The pool has migrated; the creator role no longer carries the fees.')

const owed = await creatorFeeAmounts(dbc, damm, { dbcPool: POOL, creator })
console.log(`booked creator fees that move with the role: ${owed.curve.quote} lamports, ${owed.curve.base} TARE base units`)
const transfer = await dbc.client.creator.transferPoolCreator({ pool: POOL, creator, newCreator: LAUNCH_WALLET })
const instructions = [ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 50_000 }), ...transfer.instructions.filter((ix) => !ix.programId.equals(ComputeBudgetProgram.programId))]

const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash('confirmed')
const tx = new VersionedTransaction(new TransactionMessage({ payerKey: deployer.publicKey, recentBlockhash: blockhash, instructions }).compileToV0Message())
tx.sign([deployer])
const simulation = await connection.simulateTransaction(tx, { sigVerify: false })
if (simulation.value.err) { console.log(simulation.value.logs?.join('\n')); throw new Error(`Simulation failed: ${JSON.stringify(simulation.value.err)}`) }
console.log(`simulated: ${simulation.value.unitsConsumed} CU, ${tx.serialize().length} bytes`)
if (!send) { console.log('Not sent; run again with --send.'); process.exit(0) }
const signature = await connection.sendRawTransaction(tx.serialize(), { maxRetries: 5 })
console.log(`sent ${signature}`)
await connection.confirmTransaction({ signature, blockhash, lastValidBlockHeight }, 'confirmed')
const after = await dbc.snapshot(POOL)
console.log(`pool creator now ${after.pool.poolState.creator.toBase58()}`)
