#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {canonicalRepository} from './terminal.mjs';

const here=path.dirname(fileURLToPath(import.meta.url));
const usage='node bench/attach-record.mjs --program ID --record ADDRESS --cluster CLUSTER [--record-url PAGE_URL] [--results FILE] [--url RPC_URL] [--registry-program ADDRESS]';
if(process.argv.includes('--help')){console.log(usage);process.exit(0)}
const options=new Map();
for(let i=2;i<process.argv.length;i+=2){
  const name=process.argv[i];const value=process.argv[i+1];
  if(!name?.startsWith('--')||!value||value.startsWith('--'))throw new Error(usage);
  if(!['--program','--record','--cluster','--record-url','--results','--url','--registry-program'].includes(name)||options.has(name))throw new Error(`Unknown or repeated option: ${name}`);
  options.set(name,value);
}
for(const name of ['--program','--record','--cluster'])if(!options.has(name))throw new Error(`Required ${name}. ${usage}`);
const alphabet='123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
function address(value){
  if(!/^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(value))throw new Error('Invalid Solana record address');
  let number=0n;for(const c of value)number=number*58n+BigInt(alphabet.indexOf(c));
  const digits=[];while(number){digits.push(Number(number&255n));number>>=8n}
  const zeros=value.match(/^1*/)[0].length;
  if(zeros+digits.length!==32)throw new Error('Solana address must decode to 32 bytes');
  return Buffer.from([...Array(zeros).fill(0),...digits.reverse()]);
}
const record=options.get('--record');address(record);
const cluster=options.get('--cluster');
if(!/^[a-zA-Z0-9][a-zA-Z0-9._-]{0,63}$/.test(cluster))throw new Error('Use a non-secret cluster label such as localnet, devnet or mainnet-beta.');
const file=path.resolve(options.get('--results')||process.env.TARE_BENCH_RESULTS||path.join(here,'results.json'));
const raw=fs.readFileSync(file);const results=JSON.parse(raw);
const row=results.programs.find(p=>p.id===options.get('--program'));
if(!row||row.status!=='measured'||row.proof?.status!=='matched')throw new Error('The selected program must have a matched measured proof.');
if(options.has('--url')){
  const response=await fetch(options.get('--url'),{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({jsonrpc:'2.0',id:1,method:'getAccountInfo',params:[record,{encoding:'base64',commitment:'confirmed'}]}),signal:AbortSignal.timeout(30000)});
  if(!response.ok)throw new Error(`RPC HTTP ${response.status}`);
  const rpc=await response.json();if(rpc.error)throw new Error(`RPC error: ${rpc.error.message}`);
  const account=rpc.result?.value;if(!account)throw new Error('Registry record account does not exist.');
  const registry=options.get('--registry-program')||JSON.parse(fs.readFileSync(path.join(here,'../registry/abi.json'),'utf8')).programId;
  address(registry);if(account.owner!==registry)throw new Error('Record owner does not match the registry program.');
  const data=Buffer.from(account.data[0],'base64');let at=0;
  function take(n){if(at+n>data.length)throw new Error('Truncated registry Record');const out=data.subarray(at,at+n);at+=n;return out}
  function blob(max){const n=take(4).readUInt32LE();if(n>max)throw new Error('Invalid registry Record field length');return take(n)}
  const utf8=new TextDecoder('utf-8',{fatal:true});const string=max=>utf8.decode(blob(max));
  const expected=createHash('sha256').update('account:Record').digest().subarray(0,8);
  if(!take(8).equals(expected))throw new Error('Account is not a registry Record.');
  const program=take(32),stock=take(32).toString('hex'),tare=take(32).toString('hex');
  const repository=string(256),commit=blob(32).toString('hex'),version=string(64),corpus=take(32).toString('hex');
  const tests=take(8).readBigUInt64LE(),generated=take(8).readBigUInt64LE(),mainnet=take(8).readBigUInt64LE(),gross=take(8).readBigUInt64LE(),net=take(8).readBigUInt64LE();
  const proof=row.weighing.proof;
  if(!program.equals(address(row.weighing.program.address))||stock!==row.weighing.builds.stock.hash||tare!==row.weighing.builds.tare.hash||corpus!==proof.corpus.hash||canonicalRepository(repository)!==canonicalRepository(row.source.repository)||commit!==row.source.commit||version!==row.weighing.toolchainVersion)throw new Error('Record build/source/corpus provenance differs from this benchmark row.');
  const counts=proof.corpus.counts;
  if(tests!==BigInt(counts.tests)||generated!==BigInt(counts.generated)||mainnet!==BigInt(counts.mainnet)||gross!==BigInt(row.weighing.totals.gross)||net!==BigInt(row.weighing.totals.net))throw new Error('Record totals differ from this benchmark row.');
}
for(const program of results.programs){
  for(const source of [program.source,program.weighing?.source]){
    if(typeof source?.repository==='string')source.repository=canonicalRepository(source.repository);
  }
}
row.record=record;row.recordCluster=cluster;
if(options.has('--record-url')){
  const url=new URL(options.get('--record-url'));
  if(!['http:','https:'].includes(url.protocol))throw new Error('Record page URL must use HTTP(S).');
  row.recordUrl=url.href;
}
// Avoid replacing a results file that changed while the RPC lookup was pending.
if(!fs.readFileSync(file).equals(raw))throw new Error('Benchmark results changed during verification; retry attachment.');
const temporary=`${file}.${process.pid}.tmp`;
fs.writeFileSync(temporary,JSON.stringify(results,null,2)+'\n');fs.renameSync(temporary,file);
console.log(`Attached ${record} on ${cluster} to ${row.id}${options.has('--url')?' (RPC provenance verified)':' (lead-supplied address)'}.`);
