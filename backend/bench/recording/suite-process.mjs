import {spawn} from 'node:child_process';
import fs from 'node:fs';
import net from 'node:net';

let active;
let interrupted;
let interruptTimer;
const delay=ms=>new Promise(resolve=>setTimeout(resolve,ms));
function birth(pid){try{const value=fs.readFileSync(`/proc/${pid}/stat`,'utf8');const fields=value.slice(value.lastIndexOf(')')+2).split(' ');return fields[0]==='Z'?null:fields[19]}catch{return null}}
function killGroup(child,signal){if(!child.pid)return;const now=birth(child.pid);if(child.ownedBirth&&now&&now!==child.ownedBirth)return;try{process.kill(-child.pid,signal)}catch(error){if(error.code!=='ESRCH')throw error}}
function registered(context){if(!context.registry)return [];try{return fs.readFileSync(context.registry,'utf8').trim().split('\n').filter(Boolean).map(line=>{const[pid,start]=line.split(' ');return {pid:Number(pid),start}}).filter(p=>Number.isInteger(p.pid)&&p.pid>1&&p.start)}catch(error){if(error.code==='ENOENT')return [];throw error}}
function killOwned(context,signal){
 killGroup(context.child,signal);
 for(const p of registered(context))if(birth(p.pid)===p.start){try{process.kill(p.pid,signal)}catch(error){if(error.code!=='ESRCH')throw error}}
}
export async function waitForPortsFree(ports=[8899,8900],timeoutMs=10000){
 const free=port=>new Promise(resolve=>{const server=net.createServer();server.once('error',()=>resolve(false));server.listen({host:'127.0.0.1',port,exclusive:true},()=>server.close(()=>resolve(true)))});
 const end=Date.now()+timeoutMs;
 do{if((await Promise.all(ports.map(free))).every(Boolean))return;await delay(100)}while(Date.now()<end);
 throw new Error(`Validator RPC ports still in use: ${ports.join(', ')}. No unrelated process is stopped.`);
}
export function validatorWrapper(){return `#!/bin/sh
if [ -n "\${TARE_OWNED_VALIDATOR_PIDS:-}" ]; then
  start=$(sed 's/.*) //' "/proc/$$/stat" | awk '{print $20}')
  printf '%s %s\\n' "$$" "$start" >> "$TARE_OWNED_VALIDATOR_PIDS"
fi
exec "$TARE_REAL_VALIDATOR" --deactivate-feature B8JJXCy5amZyWG9r7EnUYLwzXSXTxG7GZ1qZ1qggo83g "$@"
`}

for(const signal of ['SIGINT','SIGTERM'])process.once(signal,()=>{interrupted=signal;process.exitCode=signal==='SIGINT'?130:143;if(active){killOwned(active,signal);interruptTimer=setTimeout(()=>{if(active)killOwned(active,'SIGKILL')},5000)}});
export async function stopSuite(){
 if(!active)return;
 const context=active;killOwned(context,'SIGTERM');
 const end=Date.now()+5000;
 while(Date.now()<end&&registered(context).some(p=>birth(p.pid)===p.start))await delay(100);
 killOwned(context,'SIGKILL');
 await waitForPortsFree(context.ports);
}
export async function runSuite(command,args,options){
 if(interrupted)throw new Error(`Test run interrupted by ${interrupted}`);
 const ports=options.validatorPorts||[8899,8900];await waitForPortsFree(ports);
 const registry=options.env?.TARE_OWNED_VALIDATOR_PIDS;
 if(registry)fs.writeFileSync(registry,'');
 const {validatorPorts,...spawnOptions}=options;
 const child=spawn(command,args,{...spawnOptions,detached:true,stdio:['ignore','pipe','pipe']});
 child.ownedBirth=birth(child.pid);active={child,registry,ports};
 const chunks={stdout:[],stderr:[]};for(const name of Object.keys(chunks))child[name].on('data',chunk=>chunks[name].push(chunk));
 let error;child.once('error',value=>{error=value});
 let cleanupTimer;child.once('exit',()=>{if(active?.child===child)killOwned(active,'SIGTERM');cleanupTimer=setTimeout(()=>{if(active?.child===child)killOwned(active,'SIGKILL')},5000)});
 const outcome=await new Promise(resolve=>child.once('close',(status,signal)=>resolve({status,signal})));
 clearTimeout(interruptTimer);clearTimeout(cleanupTimer);
 try{await stopSuite()}finally{active=undefined}
 if(interrupted)throw new Error(`Test run interrupted by ${interrupted}`);
 return {...outcome,error,stdout:Buffer.concat(chunks.stdout).toString('utf8'),stderr:Buffer.concat(chunks.stderr).toString('utf8')};
}
