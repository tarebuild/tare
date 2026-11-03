#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';

export function summarize(directory, ids, stagesPassed = true) {
  const read = (file, fallback) => fs.existsSync(file) ? JSON.parse(fs.readFileSync(file, 'utf8')) : fallback;
  const manifest=read(path.join(directory,'manifest.json'),null);
  const selected=manifest.programs.filter(p=>!ids.length||ids.includes(p.id));
  for(const id of ids)if(!selected.some(p=>p.id===id))throw new Error(`Unknown corpus program: ${id}`);
  const resultFile=process.env.TARE_BENCH_RESULTS||path.join(directory,'results.json');
  const results=read(resultFile,{programs:[]});
  const stock=read(path.join(directory,'stock-tests.json'),{reports:[]}).reports;
  const tare=read(path.join(directory,'upstream-tests.json'),{reports:[]}).reports;
  const testRun=(reports,p,hash,field)=>reports.findLast(r=>r.sourceCommit===p.revision&&r.programs?.some(x=>x.id===p.id&&x[field]===hash));
  const display=run=>!run?'not run':run.error?'failed':`${run.tests?.passed??0}/${run.tests?.ran??0}${run.commandExit!==0?' (exit '+run.commandExit+')':''}`;
  const passes=run=>run&&!run.error&&run.commandExit===0&&run.tests?.ran>0&&run.tests.failed===0;
  let ok=stagesPassed;
  const rows=selected.map(p=>{
    const result=results.programs.find(r=>r.id===p.id);
    if(p.local){
      const tests=result?.tests;const passed=result?.status==='measured'&&result.proof?.status==='matched'&&tests?.ran>0&&tests.failed===0;
      ok&&=passed;
      return {program:p.id,recordedTransactions:result?.proof?.corpus?.counts?.tests??0,stockTests:tests?`${tests.passed}/${tests.ran} native`:'not run',tareTests:tests?`${tests.passed}/${tests.ran} native`:'not run',status:passed?'passed':'failed'};
    }
    if(p.id.startsWith('production-')){
      const passed=result?.status==='measured'&&result.proof?.status==='matched';
      ok&&=passed;
      return {program:p.id,recordedTransactions:0,stockTests:'not recorded',tareTests:'not run',status:passed?'proof matched; upstream suite pending':'failed'};
    }
    const stockHash=result?.weighing?.builds?.stock?.hash;
    const tareHash=result?.weighing?.builds?.tare?.hash;
    const stockRun=testRun(stock,p,stockHash,'stockBuildHash');
    const tareRun=testRun(tare,p,tareHash,'tareBuildHash');
    const recording=read(path.join(directory,'corpus-data',p.id,'tests.json'),null);
    const validRecording=recording&&stockRun&&!stockRun.error&&recording.recordingSession===stockRun.recordingSession&&stockRun.programs?.some(x=>x.id===p.id&&x.recordedTransactions>0)&&recording.sourceCommit===p.revision&&recording.stockBuildHash===stockHash;
    const count=validRecording?recording.sequences.reduce((n,s)=>n+s.transactions.length,0):0;
    const passed=result?.status==='measured'&&result.proof?.status==='matched'&&passes(stockRun)&&passes(tareRun)&&count>0&&!recording.captureErrors.length;
    ok&&=passed;
    return {program:p.id,recordedTransactions:count,stockTests:display(stockRun),tareTests:display(tareRun),status:passed?'passed':'failed'};
  });
  return {schemaVersion:'1',generatedAt:new Date().toISOString(),stagesPassed,passed:ok,programs:rows};
}

if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url)){
  const here=path.dirname(fileURLToPath(import.meta.url));
  const report=summarize(here,process.argv.slice(2),process.env.TARE_LINUX_STAGES_PASSED!=='0');
  const columns=['Program','Recorded tx','Stock tests','Tare tests','Status'];
  const rows=report.programs.map(p=>[p.program,String(p.recordedTransactions),p.stockTests,p.tareTests,p.status]);
  const widths=columns.map((x,i)=>Math.max(x.length,...rows.map(r=>r[i].length)));
  const line=row=>row.map((x,i)=>x.padEnd(widths[i])).join(' | ');
  console.log('\nLinux verification summary (tests: passed/ran)');
  console.log(line(columns));console.log(widths.map(n=>'-'.repeat(n)).join('-+-'));
  for(const row of rows)console.log(line(row));
  fs.writeFileSync(path.join(here,'linux-summary.json'),JSON.stringify(report,null,2)+'\n');
  process.exitCode=report.passed?0:1;
}
