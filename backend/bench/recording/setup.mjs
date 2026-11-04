import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';
import {supportedVersions,cohort,checkCli} from './cohorts.mjs';
import {recorderDependencies} from './dependencies.mjs';
const here=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const manifest=JSON.parse(fs.readFileSync(path.join(here,'manifest.json')));
const versions=process.argv.slice(2).length?process.argv.slice(2):supportedVersions;
const recorderRoot=path.join(here,'tools/recorder');fs.mkdirSync(recorderRoot,{recursive:true});
fs.writeFileSync(path.join(recorderRoot,'package.json'),JSON.stringify({name:'tare-recorder-dependencies',private:true,dependencies:{'@solana/web3.js':'1.99.0',bs58:'4.0.1',toml:'3.0.0'}},null,2)+'\n');
function run(command,args,cwd){console.log(`$ ${command} ${args.join(' ')}`);const r=spawnSync(command,args,{cwd,stdio:'inherit'});if(r.status!==0)throw new Error(`${command} failed with exit ${r.status}`)}
run('npm',['install','--prefix',recorderRoot,'--ignore-scripts','--no-audit','--no-fund'],here);
for(const version of versions){
 if(!supportedVersions.includes(version))throw new Error(`Unsupported upstream-suite Anchor version: ${version}`);
 const program=manifest.programs.find(p=>p.anchorVersion===version);if(!program)throw new Error(`No corpus checkout for Anchor ${version}`);
 const info=cohort(here,program);
 const toolCheckout=path.join(here,`tools/source/anchor-${version}`);
 fs.mkdirSync(path.dirname(toolCheckout),{recursive:true});
 if(!fs.existsSync(toolCheckout))run('git',['-c','core.autocrlf=false','clone','--no-hardlinks',info.checkout,toolCheckout],here);
 const toolHead=spawnSync('git',['-c',`safe.directory=${toolCheckout}`,'-C',toolCheckout,'rev-parse','HEAD'],{encoding:'utf8'});
 if(toolHead.status!==0||toolHead.stdout.trim()!==program.revision)throw new Error(`Tool checkout revision mismatch: ${toolCheckout}`);
 run('git',['-c',`safe.directory=${toolCheckout}`,'-c','core.autocrlf=false','-C',toolCheckout,'submodule','update','--init','--recursive','--depth','1'],here);
 const links=path.join(here,'tools/yarn-links',version);fs.mkdirSync(links,{recursive:true});
 run('yarn',['--cwd',path.join(toolCheckout,'ts'),'install','--frozen-lockfile','--non-interactive'],info.checkout);
 const packages=['borsh','anchor-errors','anchor','spl-associated-token-account','spl-token'];
 const names=[];
 for(const name of packages){const root=path.join(toolCheckout,'ts/packages',name);const pkg=JSON.parse(fs.readFileSync(path.join(root,'package.json')));names.push(pkg.name);run('yarn',['--cwd',root,name==='borsh'||name==='anchor-errors'?'build':'build:node'],info.checkout);run('yarn',['--cwd',root,'link','--link-folder',links],info.checkout)}
 for(const group of ['examples/tutorial','tests']){const root=path.join(info.checkout,group);run('yarn',['--cwd',root,'install','--frozen-lockfile','--non-interactive'],info.checkout);run('yarn',['--cwd',root,'link','--link-folder',links,...names.filter(n=>!n.endsWith('/errors')&&!n.endsWith('/anchor-errors'))],info.checkout);recorderDependencies(root,info.clientPackage);console.log(`Recorder dependencies verified: Anchor ${version}, ${group}`)}
 if(!fs.existsSync(info.anchor))run('cargo',['install','--path',path.join(toolCheckout,'cli'),'--locked','--root',path.join(here,`tools/anchor-${version}`),'--bin','anchor',...(version.startsWith('1.')?['--no-default-features','--features','solana-v4']:[])],toolCheckout);
 checkCli(info);
}
