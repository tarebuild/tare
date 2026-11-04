import {terminalText} from '../terminal.mjs';
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {selectPrograms,workspaces,cohort,checkCli,testArgs} from './cohorts.mjs';
import {testCounts,writeReports} from './results.mjs';
import {recorderDependencies} from './dependencies.mjs';
import {runSuite,stopSuite,validatorWrapper} from './suite-process.mjs';
const here=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const manifest=JSON.parse(fs.readFileSync(path.join(here,'manifest.json')));
const selected=selectPrograms(manifest,process.argv.slice(2));
if(!selected.length){console.log("No upstream validator suites selected; local registry tests run in bench/run.mjs.");process.exit(0)}
const find = spawnSync('sh', ['-c', 'command -v solana-test-validator'], { encoding: 'utf8' });
if (find.status !== 0) throw new Error('solana-test-validator is required inside the Linux container');
const realValidator = find.stdout.trim();
const runDir = fs.mkdtempSync(path.join(here, 'upstream-run-'));
const wallet = path.join(runDir, 'wallet.json');
const keygen = spawnSync('solana-keygen', ['new', '--no-bip39-passphrase', '--silent', '--outfile', wallet], { encoding: 'utf8' });
if (keygen.status !== 0) {
  if (fs.existsSync(wallet)) fs.unlinkSync(wallet);
  fs.rmdirSync(runDir);
  throw new Error(keygen.stderr || 'Failed to create isolated test wallet');
}
const wrapper = path.join(runDir, 'solana-test-validator');
// Matches this round's SBPF-v0 target and the mainnet feature state in build notes.
fs.writeFileSync(wrapper, validatorWrapper());
fs.chmodSync(wrapper, 0o755);
const environment = { ...process.env, TARE_REAL_VALIDATOR: realValidator, TARE_OWNED_VALIDATOR_PIDS:path.join(runDir,'validators.pid'), PATH: `${runDir}:${process.env.PATH}` };
const reportFile = path.join(here, 'upstream-tests.json');
const selectedWorkspaces = new Set(workspaces(selected).map(p => `${p.checkout}/${p.workspace}`));
const previous = fs.existsSync(reportFile) ? JSON.parse(fs.readFileSync(reportFile, 'utf8')).reports : [];
const reports = previous.filter(r => !selectedWorkspaces.has(`${r.checkout}/${r.workspace}`));
fs.mkdirSync(path.join(here,'transcripts'),{recursive:true});
try {
  for (const selectedProgram of workspaces(selected)) {
    const workspace=selectedProgram.workspace;
    const root=path.join(here,selectedProgram.checkout,workspace);
    const programs=manifest.programs.filter(p=>p.workspace===workspace&&p.checkout===selectedProgram.checkout);
    const backups = [];
    try {
      const info=cohort(here,selectedProgram);checkCli(info);const anchor=info.anchor;const {toml}=recorderDependencies(root,info.clientPackage);const config=toml.parse(fs.readFileSync(path.join(root,'Anchor.toml'),'utf8'));
      for (const p of programs) {
        const stock = path.join(root, 'target/deploy', `${p.name}.so`);
        const tare = path.join(root, 'target/tare', `${p.name}.so`);
        const idl = path.join(root, 'target/idl', `${p.name}.json`);
        if (!fs.existsSync(tare) || !fs.existsSync(stock) || !fs.existsSync(idl)) throw new Error(`Build all workspace programs first; missing output for ${p.id}`);
        backups.push([stock, fs.readFileSync(stock)], [idl, fs.readFileSync(idl)]);
        fs.mkdirSync(path.join(root, 'target/types'), { recursive: true });
        const typeResult = spawnSync(anchor, ['idl', 'type', idl, '--out', path.join(root, 'target/types', `${p.name}.ts`)], { encoding: 'utf8' });
        if (typeResult.status !== 0) throw new Error(typeResult.stderr || `IDL type generation failed for ${p.id}`);
        fs.copyFileSync(tare, stock);
      }
      // Anchor preloads at Anchor.toml addresses, funds this wallet, and starts
      // each Test.toml suite with its own inherited fixtures and fresh ledger.
      const preflightFile=path.join(runDir,'preflight.json');fs.writeFileSync(preflightFile,JSON.stringify({url:'http://127.0.0.1:8899',targets:programs.map(p=>({address:typeof config.programs.localnet[p.name]==='string'?config.programs.localnet[p.name]:config.programs.localnet[p.name].address,elf:path.join(root,'target/deploy',`${p.name}.so`)}))}));
      const args=testArgs(info,wallet);
      const result = await runSuite(anchor, args, { cwd: root, env: {...environment,PATH:`${runDir}:${path.dirname(anchor)}:${process.env.PATH}`,NODE_OPTIONS:`${process.env.NODE_OPTIONS||''} --require=${JSON.stringify(path.join(here,'recording/preload.cjs'))}`,TARE_VALIDATOR_PREFLIGHT:path.join(here,'recording/preflight.mjs'),TARE_VALIDATOR_PREFLIGHT_CONFIG:preflightFile} });
      const output = `${result.stdout || ''}${result.stderr || ''}`;
      const transcript = terminalText(`$ anchor ${args.join(' ')}\n${output}\n[exit ${result.status}]\n`,root);
      fs.appendFileSync(path.join(here, 'transcripts', `${selectedProgram.id}-upstream-tests.txt`), transcript);
      process.stdout.write(transcript);
      const counts = testCounts(output);
      reports.push({ workspace, checkout:selectedProgram.checkout, sourceCommit:selectedProgram.revision, anchorVersion:selectedProgram.anchorVersion, programs: programs.map(p => ({ id: p.id, tareBuildHash: crypto.createHash('sha256').update(fs.readFileSync(path.join(root, 'target/tare', `${p.name}.so`))).digest('hex') })), commandExit: result.status, tests: counts });
      if (result.error) throw result.error;
      if (result.status !== 0 || counts.failed > 0 || counts.ran === 0) process.exitCode = 1;
    } catch (error) {
      process.exitCode = 1;
      reports.push({ workspace, checkout:selectedProgram.checkout, sourceCommit:selectedProgram.revision, anchorVersion:selectedProgram.anchorVersion, error: error.message });
      const text=terminalText(`Upstream tests failed for ${selectedProgram.id}: ${error.message}\n`,root);
      fs.appendFileSync(path.join(here,'transcripts',`${selectedProgram.id}-upstream-tests.txt`),text);
      process.stderr.write(text);
    } finally {
      for (const [filename, bytes] of backups) fs.writeFileSync(filename, bytes);
    }
  }
} finally {
  await stopSuite();
  // Delete only this invocation's generated wallet and executable wrapper.
  fs.unlinkSync(wallet);
  fs.unlinkSync(wrapper);
  if(fs.existsSync(path.join(runDir,'validators.pid')))fs.unlinkSync(path.join(runDir,'validators.pid'));
  if(fs.existsSync(path.join(runDir,'preflight.json')))fs.unlinkSync(path.join(runDir,'preflight.json'));
  fs.rmdirSync(runDir);
  writeReports(reportFile, reports);
}