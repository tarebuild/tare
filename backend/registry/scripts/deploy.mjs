#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { Connection, Keypair, PublicKey, SystemProgram, Transaction, TransactionInstruction, sendAndConfirmTransaction } from '@solana/web3.js';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const options = new Map();
for (let i = 2; i < process.argv.length; i += 2) {
  if (!process.argv[i].startsWith('--') || !process.argv[i + 1]) throw new Error('Use --keypair FILE --fee LAMPORTS --fee-recipient ADDRESS [--url URL] [--program-keypair FILE] [--buffer-keypair FILE] [--compute-unit-price MICROLAMPORTS] [--binary FILE] [--authority FILE].');
  options.set(process.argv[i].slice(2), process.argv[i + 1]);
}
for (const name of options.keys()) if (!['keypair', 'fee', 'fee-recipient', 'url', 'program-keypair', 'buffer-keypair', 'compute-unit-price', 'binary', 'authority'].includes(name)) throw new Error(`Unknown option --${name}.`);
if (!options.has('keypair') || !options.has('fee') || !options.has('fee-recipient')) throw new Error('--keypair, --fee and --fee-recipient are required; no fee or recipient is assumed.');
// The launch wallet: every attest fee is paid to it, and the crank signs as it.
const feeRecipient = new PublicKey(options.get('fee-recipient'));
if (!/^(0|[1-9][0-9]*)$/.test(options.get('fee'))) throw new Error('--fee must be integer lamports.');
const fee = BigInt(options.get('fee'));
if (fee > 0xffffffffffffffffn) throw new Error('--fee exceeds u64.');
const url = options.get('url') || 'http://127.0.0.1:7840';
const local = /^https?:\/\/(127\.0\.0\.1|localhost)[:/]/.test(url);
const computeUnitPrice = options.get('compute-unit-price') ?? (local ? '0' : '50000');
if (!/^(0|[1-9][0-9]*)$/.test(computeUnitPrice)) throw new Error('--compute-unit-price must be integer micro-lamports.');
const walletPath = path.resolve(options.get('keypair'));
// A fixed buffer keypair lets an interrupted deploy resume and its rent come back.
const bufferPath = path.resolve(options.get('buffer-keypair') || path.join(path.dirname(walletPath), 'tare-buffer.json'));
if (!fs.existsSync(bufferPath)) fs.writeFileSync(bufferPath, JSON.stringify(Array.from(Keypair.generate().secretKey)), { flag: 'wx', mode: 0o600 });
const programKeyPath = path.resolve(options.get('program-keypair') || path.join(root, 'localnet-keypair.json'));
const binaryPath = path.resolve(options.get('binary') || path.join(root, 'target/tare/tare_registry.so'));
const wallet = Keypair.fromSecretKey(Uint8Array.from(JSON.parse(fs.readFileSync(walletPath, 'utf8'))));
// The upgrade authority and config admin: its own key when the payer is not trusted with them.
const authorityPath = path.resolve(options.get('authority') || walletPath);
const authority = Keypair.fromSecretKey(Uint8Array.from(JSON.parse(fs.readFileSync(authorityPath, 'utf8'))));
const programKey = Keypair.fromSecretKey(Uint8Array.from(JSON.parse(fs.readFileSync(programKeyPath, 'utf8'))));
const idl = JSON.parse(fs.readFileSync(path.join(root, 'target/idl/tare_registry.json'), 'utf8'));
const program = new PublicKey(idl.address);
if (!program.equals(programKey.publicKey)) throw new Error('Program keypair does not match the stock IDL address. Change the single declare_id! in src/lib.rs and rebuild before deployment.');
const elf = fs.readFileSync(binaryPath);
const loader = new PublicKey('BPFLoaderUpgradeab1e11111111111111111111111');
const connection = new Connection(url, 'confirmed');
const [config] = PublicKey.findProgramAddressSync([Buffer.from('config')], program);
const [eventAuthority] = PublicKey.findProgramAddressSync([Buffer.from('__event_authority')], program);
const programInfo = await connection.getAccountInfo(program);
let deployedSame = false;
if (programInfo) {
  if (!programInfo.executable || !programInfo.owner.equals(loader) || programInfo.data.readUInt32LE() !== 2) throw new Error('Existing registry address is not an upgradeable program.');
  const data = await connection.getAccountInfo(new PublicKey(programInfo.data.subarray(4, 36)));
  deployedSame = !!data && data.data.subarray(45, 45 + elf.length).equals(elf) && data.data.subarray(45 + elf.length).every(value => value === 0);
}
if (!deployedSame) {
  const result = solana(['program', 'deploy', '--use-rpc', '--url', url, '--keypair', walletPath, '--upgrade-authority', authorityPath,
    '--program-id', programKeyPath, '--buffer', bufferPath, '--with-compute-unit-price', computeUnitPrice, binaryPath]);
  if (result.status !== 0) throw new Error('solana program deploy failed.');
} else console.log('Registry executable already matches; deployment skipped.');
// A load-balanced RPC can answer from a node behind the deploy: ask again for a while.
let actualProgram = await connection.getAccountInfo(program);
for (let attempt = 0; !actualProgram && attempt < 10; attempt++) {
  await new Promise(resolve => setTimeout(resolve, 3000));
  actualProgram = await connection.getAccountInfo(program);
}
if (!actualProgram || !actualProgram.owner.equals(loader) || actualProgram.data.readUInt32LE() !== 2) throw new Error('Deployed upgradeable program metadata is unavailable.');
const programData = new PublicKey(actualProgram.data.subarray(4, 36));
const info = await connection.getAccountInfo(config);
const hash = name => createHash('sha256').update(name).digest().subarray(0, 8);
const u64 = value => { const b = Buffer.alloc(8); b.writeBigUInt64LE(value); return b; };
const ro = (pubkey, isSigner = false) => ({ pubkey, isSigner, isWritable: false });
const rw = (pubkey, isSigner = false) => ({ pubkey, isSigner, isWritable: true });
let instruction;
if (!info) {
  instruction = new TransactionInstruction({ programId: program, keys: [rw(authority.publicKey, true), ro(program), ro(programData), rw(config), ro(SystemProgram.programId), ro(eventAuthority), ro(program)], data: Buffer.concat([hash('global:init_config'), authority.publicKey.toBuffer(), feeRecipient.toBuffer(), u64(fee)]) });
} else {
  if (!info.owner.equals(program) || info.data.length !== 80 || !info.data.subarray(0, 8).equals(hash('account:Config'))) throw new Error('Config has an unexpected owner or layout.');
  const admin = new PublicKey(info.data.subarray(8, 40));
  const currentRecipient = new PublicKey(info.data.subarray(40, 72));
  const currentFee = info.data.readBigUInt64LE(72);
  if (currentFee !== fee || !currentRecipient.equals(feeRecipient)) {
    if (!admin.equals(authority.publicKey)) throw new Error('The supplied authority is not the current config admin.');
    instruction = new TransactionInstruction({ programId: program, keys: [ro(authority.publicKey, true), rw(config), ro(eventAuthority), ro(program)], data: Buffer.concat([hash('global:set_config'), admin.toBuffer(), feeRecipient.toBuffer(), u64(fee)]) });
  } else console.log('Registry config already matches; initialization skipped.');
}
if (instruction) {
  const transaction = new Transaction().add(instruction);
  transaction.feePayer = wallet.publicKey;
  const signers = authority.publicKey.equals(wallet.publicKey) ? [wallet] : [wallet, authority];
  const signature = await sendAndConfirmTransaction(connection, transaction, signers, { commitment: 'confirmed', preflightCommitment: 'confirmed' });
  console.log(`Config transaction: ${signature}`);
}
const finalConfig = await connection.getAccountInfo(config);
if (!finalConfig || finalConfig.data.readBigUInt64LE(72) !== fee || !new PublicKey(finalConfig.data.subarray(40, 72)).equals(feeRecipient)) throw new Error('Post-deployment registry verification failed.');
// A fee sent to an account below its rent-exempt minimum fails the attest: the recipient holds SOL first.
const recipientLamports = await connection.getBalance(feeRecipient);
if (recipientLamports < await connection.getMinimumBalanceForRentExemption(0)) console.log(`Fund the fee recipient ${feeRecipient.toBase58()} before the first attest: it holds ${recipientLamports} lamports.`);
const genesisHash = await connection.getGenesisHash();
const cluster = { '5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d': 'mainnet' }[genesisHash] ?? (local ? 'localnet' : genesisHash);
const programDataBytes = (await connection.getAccountInfo(programData)).data;
const upgradeAuthority = programDataBytes[12] ? new PublicKey(programDataBytes.subarray(13, 45)).toBase58() : null;
// One record per cluster, holding only what derives from the chain: never the RPC URL.
const deployment = {
  cluster, genesisHash, programId: program.toBase58(), programData: programData.toBase58(), config: config.toBase58(), feeRecipient: feeRecipient.toBase58(),
  upgradeAuthority, admin: new PublicKey(finalConfig.data.subarray(8, 40)).toBase58(),
  attestFeeLamports: fee.toString(), elfSha256: createHash('sha256').update(elf).digest('hex'),
};
fs.mkdirSync(path.join(root, 'deployments'), { recursive: true });
fs.writeFileSync(path.join(root, 'deployments', `${cluster}.json`), JSON.stringify(deployment, null, 2) + '\n');
console.log(JSON.stringify(deployment, null, 2));

// The Solana CLI runs in WSL on Windows, as the solana skill deploys; elsewhere it runs as is.
function solana(args) {
  if (process.platform !== 'win32') return spawnSync('solana', args, { stdio: ['ignore', 'inherit', 'inherit'] });
  const home = spawnSync('wsl.exe', ['--exec', 'printenv', 'HOME'], { encoding: 'utf8' }).stdout.trim();
  const bin = `${home}/.local/share/solana/install/releases/2.3.13/solana-release/bin`;
  const wslPath = (value) => /^[A-Za-z]:[\\/]/.test(value) ? `/mnt/${value[0].toLowerCase()}${value.slice(2).replaceAll('\\', '/')}` : value;
  return spawnSync('wsl.exe', ['--exec', 'env', `PATH=${bin}:/usr/bin:/bin`, 'solana', ...args.map(wslPath)], { stdio: ['ignore', 'inherit', 'inherit'] });
}
