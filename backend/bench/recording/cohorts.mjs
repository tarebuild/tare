import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';

export const supportedVersions=['0.31.1','0.32.3','1.2.2'];
export function selectPrograms(manifest,ids){
 const selected=manifest.programs.filter(p=>!ids.length||ids.includes(p.id));
 for(const id of ids)if(!selected.some(p=>p.id===id))throw new Error(`Unknown corpus entry: ${id}`);
 const upstream=selected.filter(p=>!p.local&&!p.id.startsWith("production-"));
 for(const program of upstream)if(!supportedVersions.includes(program.anchorVersion))throw new Error(`Unsupported upstream-suite Anchor version: ${program.anchorVersion} (${program.id})`);
 return upstream;
}
export function workspaces(programs){return [...new Map(programs.map(p=>[`${p.checkout}/${p.workspace}`,p])).values()]}
export function cohort(here,program){
 const checkout=path.join(here,program.checkout);
 const revision=spawnSync('git',['-c',`safe.directory=${checkout}`,'-C',checkout,'rev-parse','HEAD'],{encoding:'utf8'});
 if(revision.status!==0||revision.stdout.trim()!==program.revision)throw new Error(`Checkout revision mismatch for ${program.id}`);
 const client=JSON.parse(fs.readFileSync(path.join(checkout,'ts/packages/anchor/package.json')));
 if(client.version!==program.anchorVersion)throw new Error(`Client version mismatch for ${program.id}`);
 return {checkout,root:path.join(checkout,program.workspace),clientPackage:client.name,anchor:path.join(here,`tools/anchor-${program.anchorVersion}/bin/anchor`),version:program.anchorVersion};
}
export function checkCli(info){
 const output=spawnSync(info.anchor,['--version'],{encoding:'utf8'});
 if(output.status!==0||output.stdout.trim()!==`anchor-cli ${info.version}`)throw new Error(`Expected anchor-cli ${info.version} at ${info.anchor}; run bench/setup-upstream-tests.sh ${info.version}`);
}
export function testArgs(info,wallet){return ['test','--skip-build','--provider.wallet',wallet,...(info.version.startsWith('1.')?['--validator','legacy']:[])]}
