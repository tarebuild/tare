#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { Keypair } from '@solana/web3.js';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
if (process.argv.length !== 3) throw new Error('Usage: node registry/scripts/set-id.mjs PROGRAM_KEYPAIR.json');
const keyFile = path.resolve(process.argv[2]);
const raw = fs.readFileSync(keyFile, 'utf8');
const id = Keypair.fromSecretKey(Uint8Array.from(JSON.parse(raw))).publicKey.toBase58();
const source = path.join(root, 'programs/tare-registry/src/lib.rs');
const old = fs.readFileSync(source, 'utf8');
if (!/declare_id!\("[1-9A-HJ-NP-Za-km-z]+"\);/.test(old)) throw new Error('Expected one ordinary declare_id! literal.');
fs.writeFileSync(source, old.replace(/declare_id!\("[1-9A-HJ-NP-Za-km-z]+"\);/, `declare_id!("${id}");`));
const config = path.join(root, 'Anchor.toml');
let toml = fs.readFileSync(config, 'utf8');
if (/\[programs\.localnet\]/.test(toml)) toml = toml.replace(/(\[programs\.localnet\][\s\S]*?tare_registry\s*=\s*)"[^"]+"/, `$1"${id}"`);
else toml += `\n[programs.localnet]\ntare_registry = "${id}"\n`;
fs.writeFileSync(config, toml);
const abiFile = path.join(root, 'abi.json');
const abi = JSON.parse(fs.readFileSync(abiFile, 'utf8')); abi.programId = id;
fs.writeFileSync(abiFile, JSON.stringify(abi, null, 2) + '\n');
// The program keypair stays where it was made, outside the project; deploy.mjs takes its path.
console.log(`Registry program ID: ${id}. Rebuild before deployment; no existing ELF or IDL was modified.`);
