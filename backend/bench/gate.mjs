#!/usr/bin/env node
// The current-stock acceptance gate. Historical comparisons are informational.
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';
const here=path.dirname(fileURLToPath(import.meta.url));
const args=process.argv.slice(2);
if(args.includes('--against-round-three')){
 const forwarded=args.filter(x=>x!=='--against-round-three');
 if(!forwarded.includes('--output'))forwarded.push('--output',path.join(here,'history/round-five/round-three-comparison.json'));
 const result=spawnSync(process.execPath,[path.join(here,'history/round-five/gate-against-round-three.mjs'),...forwarded],{encoding:'utf8'});
 process.stdout.write((result.stdout||'').replace('Round-three gate:','Informational round-three comparison:'));
 if(result.stderr)process.stderr.write(result.stderr);
 console.log('Informational only: historical regressions do not determine current-stock acceptance.');
 process.exit(0);
}
const summaryOnly=args.includes('--summary');if(summaryOnly)args.splice(args.indexOf('--summary'),1);
const opts={results:path.join(here,'results.json'),output:path.join(here,'gaps/round-six/gate.json'),manifest:path.join(here,'manifest.json')};
// Legacy paths may be passed by an already-running worker; only explicit
// --against-round-three activates historical comparison.
for(let i=0;i<args.length;i++){
 const key=args[i].replace(/^--/,'');if(!args[i+1])throw Error(`Missing option ${args[i]}`);
 if(['baseline','index','reports'].includes(key)){i++;continue}
 if(!(key in opts))throw Error(`Unknown option ${args[i]}`);opts[key]=path.resolve(args[++i]);
}
const read=p=>JSON.parse(fs.readFileSync(p,'utf8'));
const hash=p=>createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const current=read(opts.results),manifest=read(opts.manifest),expected=new Set(manifest.programs.map(p=>p.id));
const seen=new Set(),coverageIssues=[],rows=[],refusals=[];
for(const row of current.programs){
 if(seen.has(row.id))coverageIssues.push(`duplicate result: ${row.id}`);seen.add(row.id);
 if(!expected.has(row.id))coverageIssues.push(`unknown result: ${row.id}`);
 if(row.status==='not_built'){refusals.push({id:row.id,reason:row.reason,construct:row.construct??null});continue}
 const reasons=[],regressions=[];const weighing=row.weighing;const production=row.kind==='production';
 if(row.status!=='measured'||!weighing){coverageIssues.push(`invalid measured result: ${row.id}`);continue}
 if(row.proof?.status!=='matched'||weighing.proof?.status!=='matched')reasons.push('observable proof did not match');
 const stockBytes=weighing.builds.stock.sizeBytes,candidateBytes=weighing.builds.tare.sizeBytes;
 if(candidateBytes>stockBytes)reasons.push(`ELF exceeds stock by ${candidateBytes-stockBytes} bytes`);
 let reached=0;
 for(const ix of weighing.instructions){
  if(ix.inputs<=0)continue;reached++;
  if(!Number.isSafeInteger(ix.gross)||!Number.isSafeInteger(ix.net)){reasons.push(`invalid measured weight: ${ix.name}`);continue}
  if(production?ix.net>=ix.gross:ix.net>ix.gross)regressions.push({instruction:ix.name,stock:ix.gross,candidate:ix.net,increase:ix.net-ix.gross,inputs:ix.inputs,requirement:production?'net < gross':'net <= gross'});
 }
 if(production&&reached===0)reasons.push('production program has no successful instruction samples');
 rows.push({id:row.id,kind:row.kind,passed:!reasons.length&&!regressions.length,reasons,regressions,stockBytes,candidateBytes,reachedInstructions:reached,proofStatus:row.proof?.status??null});
}
for(const id of expected)if(!seen.has(id))coverageIssues.push(`missing result: ${id}`);
const passed=coverageIssues.length===0&&rows.every(r=>r.passed);
const output={schemaVersion:1,comparison:'current_stock',candidate:opts.results,candidateResultsSha256:hash(opts.results),manifest:opts.manifest,manifestSha256:hash(opts.manifest),rule:'Every measured program must match observables, have Tare ELF size <= current stock, and every successfully reached instruction must have net <= gross; production instructions require net < gross. Every manifest program must have an explicit measured or not_built result.',passed,coverageIssues,counts:{manifestPrograms:expected.size,recordedPrograms:seen.size,programs:rows.length,passed:rows.filter(r=>r.passed).length,failed:rows.filter(r=>!r.passed).length,notBuilt:refusals.length,sizeRegressions:rows.filter(r=>r.candidateBytes>r.stockBytes).length,computeRegressions:rows.reduce((n,r)=>n+r.regressions.length,0),proofFailures:rows.filter(r=>r.proofStatus!=='matched').length},programs:rows,notBuilt:refusals};
fs.mkdirSync(path.dirname(opts.output),{recursive:true});fs.writeFileSync(opts.output,JSON.stringify(output,null,2)+'\n');
console.log(`Current-stock gate: ${passed?'PASS':'FAIL'}: ${output.counts.passed}/${rows.length} measured programs pass; ${output.counts.sizeRegressions} size failures; ${output.counts.computeRegressions} instruction CU failures.`);
console.log(`Coverage: ${seen.size}/${expected.size} manifest results; ${refusals.length} explicit not-built; ${coverageIssues.length} coverage issues; ${output.counts.proofFailures} proof failures.`);
if(!summaryOnly){for(const row of rows.filter(r=>!r.passed))console.log(`${row.id}: ${[...row.reasons,...row.regressions.map(x=>`${x.instruction} ${x.stock}->${x.candidate} CU (${x.requirement})`)].join('; ')}`);for(const issue of coverageIssues)console.log(issue)}
console.log(`Report: ${path.relative(process.cwd(),opts.output)}`);
process.exitCode=passed?0:1;
