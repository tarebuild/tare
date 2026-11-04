import {terminalText} from '../terminal.mjs';
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {fileURLToPath} from 'node:url';
import {spawn,spawnSync} from 'node:child_process';
import {selectPrograms,workspaces,cohort,checkCli,testArgs} from './cohorts.mjs';
import {recorderDependencies} from './dependencies.mjs';
import {testCounts,writeReports} from './results.mjs';
import {runSuite,stopSuite,validatorWrapper} from './suite-process.mjs';

const here=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const manifest=JSON.parse(fs.readFileSync(path.join(here,'manifest.json')));
const selected=selectPrograms(manifest,process.argv.slice(2));
if(!selected.length){console.log('No upstream validator suites selected; local registry tests run in bench/run.mjs.');process.exit(0)}
const reportFile=path.join(here,'stock-tests.json');
const selectedWorkspaces=new Set(workspaces(selected).map(p=>`${p.checkout}/${p.workspace}`));
const previous=fs.existsSync(reportFile)?JSON.parse(fs.readFileSync(reportFile,'utf8')).reports:[];
const reports=previous.filter(r=>!selectedWorkspaces.has(`${r.checkout}/${r.workspace}`));
const temp=fs.mkdtempSync(path.join(here,'record-run-'));
const wallet=path.join(temp,'wallet.json');
let proxy;
async function stopProxy(){
  if(!proxy)return;
  const child=proxy;proxy=undefined;
  if(!child.pid||child.exitCode!==null||child.signalCode!==null)return;
  const done=new Promise(resolve=>child.once('exit',resolve));
  const timer=setTimeout(()=>child.kill('SIGKILL'),5000);
  child.kill('SIGTERM');await done;clearTimeout(timer);
}
try{
  const keygen=spawnSync('solana-keygen',['new','--no-bip39-passphrase','--silent','--outfile',wallet],{encoding:'utf8'});
  if(keygen.status!==0)throw new Error(keygen.stderr||'Failed to create isolated test wallet');
  const validator=spawnSync('sh',['-c','command -v solana-test-validator'],{encoding:'utf8'}).stdout.trim();
  if(!validator)throw new Error('solana-test-validator missing');
  fs.writeFileSync(path.join(temp,'solana-test-validator'),validatorWrapper());
  fs.chmodSync(path.join(temp,'solana-test-validator'),0o755);
  for(const workspaceProgram of workspaces(selected)){
    const root=path.join(here,workspaceProgram.checkout,workspaceProgram.workspace);
    const report={workspace:workspaceProgram.workspace,checkout:workspaceProgram.checkout,sourceCommit:workspaceProgram.revision,anchorVersion:workspaceProgram.anchorVersion};
    const backups=[];
    let logFile,logPath;
    try{
      const info=cohort(here,workspaceProgram);checkCli(info);
      const {toml}=recorderDependencies(root,info.clientPackage);
      const config=toml.parse(fs.readFileSync(path.join(root,'Anchor.toml'),'utf8'));
      const targets=manifest.programs.filter(p=>p.workspace===workspaceProgram.workspace&&p.checkout===workspaceProgram.checkout).map(p=>{
        const entry=config.programs.localnet[p.name];const address=typeof entry==='string'?entry:entry.address;
        const idl=path.join(root,'target/idl',`${p.name}.json`);const elf=path.join(root,'target/deploy',`${p.name}.so`);
        if(!fs.existsSync(elf)||!fs.existsSync(idl))throw new Error(`Stock build missing: ${p.id}`);
        backups.push([idl,fs.readFileSync(idl)]);
        fs.mkdirSync(path.join(root,'target/types'),{recursive:true});
        const type=spawnSync(info.anchor,['idl','type',idl,'--out',path.join(root,'target/types',`${p.name}.ts`)],{encoding:'utf8'});
        if(type.status!==0)throw new Error(type.stderr||`IDL type generation failed for ${p.id}`);
        return {id:p.id,address,stockBuildHash:crypto.createHash('sha256').update(fs.readFileSync(elf)).digest('hex')};
      });
      report.programs=targets;
      const proxyConfig={session:crypto.randomUUID(),workspace:root,upstream:'http://127.0.0.1:8899',port:7812,targets,output:path.join(here,'corpus-data'),sourceCommit:workspaceProgram.revision,anchorPackage:info.clientPackage};
      report.recordingSession=proxyConfig.session;
      const configFile=path.join(temp,'config.json');fs.writeFileSync(configFile,JSON.stringify(proxyConfig));
      fs.mkdirSync(path.join(here,'transcripts'),{recursive:true});
      await import('./suite-process.mjs').then(m=>m.waitForPortsFree([7812],1000));
      logPath=path.join(here,'transcripts',`${workspaceProgram.id}-recorder.txt`);logFile=fs.openSync(logPath,'a');
      const proxyEnv={...process.env};delete proxyEnv.NODE_OPTIONS;
      proxy=spawn(process.execPath,[path.join(here,'recording/rpc-proxy.mjs'),configFile],{env:proxyEnv,stdio:['ignore',logFile,logFile]});
      let proxyError;proxy.once('error',error=>{proxyError=error});
      let ready=false;
      for(let i=0;i<100;i++){
        if(proxyError)throw proxyError;
        if(proxy.exitCode!==null||proxy.signalCode!==null)throw new Error(`Recorder exited before readiness; see ${path.basename(logPath)}`);
        try{
          const response=await fetch('http://127.0.0.1:7812/_tare/health',{signal:AbortSignal.timeout(1000)});
          if(response.ok&&(await response.json()).session===proxyConfig.session){ready=true;break}
        }catch{}
        await new Promise(resolve=>setTimeout(resolve,50));
      }
      if(!ready)throw new Error(`Recorder did not start; see ${path.basename(logPath)}`);
      const preflightFile=path.join(temp,'preflight.json');fs.writeFileSync(preflightFile,JSON.stringify({url:'http://127.0.0.1:8899',targets:targets.map(t=>({...t,elf:path.join(root,'target/deploy',`${manifest.programs.find(p=>p.id===t.id).name}.so`)}))}));
      const options=`${process.env.NODE_OPTIONS||''} --require=${JSON.stringify(path.join(here,'recording/preload.cjs'))} --require=${JSON.stringify(path.join(here,'recording/redirect.cjs'))}`;
      const args=testArgs(info,wallet);
      const result=await runSuite(info.anchor,args,{cwd:root,env:{...process.env,PATH:`${temp}:${path.dirname(info.anchor)}:${process.env.PATH}`,TARE_REAL_VALIDATOR:validator,TARE_OWNED_VALIDATOR_PIDS:path.join(temp,'validators.pid'),TARE_RECORD_PROXY:'http://127.0.0.1:7812',NODE_OPTIONS:options,TARE_VALIDATOR_PREFLIGHT:path.join(here,'recording/preflight.mjs'),TARE_VALIDATOR_PREFLIGHT_CONFIG:preflightFile}});
      const output=`${result.stdout||''}${result.stderr||''}`;
      const text=terminalText(`$ anchor ${args.join(' ')}\n${output}\n[exit ${result.status}]\n`,root);
      fs.appendFileSync(path.join(here,'transcripts',`${workspaceProgram.id}-stock-recording.txt`),text);process.stdout.write(text);
      report.commandExit=result.status;report.tests=testCounts(output);
      if(result.error)throw result.error;
      const flushed=await fetch('http://127.0.0.1:7812/_tare/flush',{method:'POST',signal:AbortSignal.timeout(5000)});
      if(!flushed.ok)throw new Error('Recorder flush failed');
      await stopProxy();
      report.programs=targets.map(target=>{
        const recording=JSON.parse(fs.readFileSync(path.join(here,'corpus-data',target.id,'tests.json'),'utf8'));
        if(recording.recordingSession!==proxyConfig.session)throw new Error(`Stale recording for ${target.id}`);
        return {...target,recordedTransactions:recording.sequences.reduce((sum,sequence)=>sum+sequence.transactions.length,0),captureErrors:recording.captureErrors};
      });
      if(result.status!==0||report.tests.failed>0||report.tests.ran===0||report.programs.some(p=>p.captureErrors.length||p.recordedTransactions===0))process.exitCode=1;
    }catch(error){
      process.exitCode=1;report.error=error.message;
      fs.mkdirSync(path.join(here,'transcripts'),{recursive:true});
      const text=terminalText(`Recording failed for ${workspaceProgram.id}: ${error.stack||error.message}\n`,root);
      fs.appendFileSync(path.join(here,'transcripts',`${workspaceProgram.id}-stock-recording.txt`),text);process.stderr.write(text);
    }finally{
      await stopProxy();
      if(logFile!==undefined){fs.closeSync(logFile);fs.writeFileSync(logPath,terminalText(fs.readFileSync(logPath,'utf8'),root))}
      for(const [file,bytes]of backups)fs.writeFileSync(file,bytes);
      reports.push(report);writeReports(reportFile,reports);
    }
  }
}finally{
  await stopSuite();
  await stopProxy();
  for(const name of ['wallet.json','config.json','solana-test-validator','validators.pid','preflight.json']){const file=path.join(temp,name);if(fs.existsSync(file))fs.unlinkSync(file)}
  fs.rmdirSync(temp);
}
