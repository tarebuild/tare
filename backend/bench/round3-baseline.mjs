#!/usr/bin/env node
import fs from 'node:fs';import path from 'node:path';import{spawn}from'node:child_process';import{createHash}from'node:crypto';import{fileURLToPath}from'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..'),bench=path.join(root,'bench');
const args=process.argv.slice(2),opts={results:path.join(bench,'results.json'),index:path.join(bench,'history/round-five/round3-corpus-index.json'),output:path.join(bench,'history/round-five/gate-reports'),helper:path.join(root,'toolchain/target/release',process.platform==='win32'?'tare-round3-gate-helper.exe':'tare-round3-gate-helper')};let ids=[];
for(let i=0;i<args.length;i++){if(args[i].startsWith('--')){const key=args[i].slice(2);if(!(key in opts))throw Error(`Unknown ${args[i]}`);opts[key]=path.resolve(args[++i]);}else ids.push(args[i]);}
const read=p=>JSON.parse(fs.readFileSync(p,'utf8')),hash=p=>createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const current=read(opts.results),baselineFile=path.join(bench,'history/round-three/final-preserved/results.json'),baseline=read(baselineFile),baselineHash=hash(baselineFile),index=read(opts.index),rows=new Map(current.programs.map(r=>[r.id,r]));
fs.mkdirSync(opts.output,{recursive:true});let failures=0;
for(const entry of index.programs){
 if(ids.length&&!ids.includes(entry.id))continue;
 if(!entry.bundle){console.log(`${entry.id}: exact old corpus unavailable`);failures++;continue;}
 const old=baseline.programs.find(r=>r.id===entry.id),row=rows.get(entry.id);
 if(!row||row.status!=='measured'){console.log(`${entry.id}: candidate unavailable`);failures++;continue;}
 const bundle=path.resolve(root,entry.bundle);if(hash(bundle)!==old.proof.corpus.hash)throw Error(`Round-three bundle hash mismatch: ${entry.id}`);
 const artifact=path.join(bench,'artifacts',entry.id),net=path.join(artifact,'tare.so'),stock=path.join(artifact,'stock.so');
 const previous=path.join(opts.output,entry.id+'.json');if(fs.existsSync(previous)){const saved=read(previous);if(saved.gate?.candidateSha256===row.weighing.builds.tare.hash&&saved.gate?.proofCorpusHash===old.proof.corpus.hash&&saved.gate?.baselineResultsSha256===baselineHash){console.log(`${entry.id}: current exact replay retained`);continue;}}
 if(hash(net)!==row.weighing.builds.tare.hash||hash(stock)!==row.weighing.builds.stock.hash)throw Error(`Candidate artifact hash mismatch: ${entry.id}`);
 const report=path.join(opts.output,entry.id+'.json'),log=fs.createWriteStream(path.join(opts.output,entry.id+'.txt'));
 const command=[bundle,stock,net,row.weighing.program.address,path.join(artifact,'idl.json'),baselineHash,report];log.write(`$ gate-helper ${command.map(x=>JSON.stringify(x)).join(' ')}\n`);
 const child=spawn(opts.helper,command,{stdio:['ignore','pipe','pipe']});child.stdout.pipe(log,{end:false});child.stderr.pipe(log,{end:false});const code=await new Promise((resolve,reject)=>{child.once('error',reject);child.once('close',resolve)});await new Promise(r=>log.end(r));
 console.log(`${entry.id}: replay exit ${code}`);if(code!==0)failures++;
}
process.exitCode=failures?1:0;
