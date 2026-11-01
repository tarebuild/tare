#!/usr/bin/env node
import { spawn, spawnSync } from 'node:child_process';
import {createHash} from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {terminalText,canonicalRepository} from './terminal.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const manifest = JSON.parse(fs.readFileSync(path.join(here, 'manifest.json'), 'utf8'));
const checkout = path.join(here, manifest.checkout);
const args = process.argv.slice(2);
if (args.includes('--list')) {
  for (const program of manifest.programs) console.log(`${program.id}\t${program.manifest}`);
  process.exit(0);
}
const jobsArg = args.indexOf('--jobs');
const jobs = jobsArg >= 0 ? Number(args.splice(jobsArg, 2)[1]) : 1;
if (!Number.isInteger(jobs) || jobs < 1 || jobs > 4) throw new Error('--jobs must be between 1 and 4');
const binArg = args.indexOf('--tare');
const tare = binArg >= 0 ? path.resolve(args.splice(binArg, 2)[1]) : process.env.TARE_BIN || path.join(here, '../toolchain/target/release', process.platform === 'win32' ? 'tare.exe' : 'tare');
const selected = args.length ? manifest.programs.filter(program => args.includes(program.id)) : [...manifest.programs];
// Exercise the formerly MAX_PATH-limited native IDL build first in a full run.
if (!args.length) selected.sort((a,b) => Number(b.id === 'v0_32_3-tests-zero-copy-programs-zero-cpi') - Number(a.id === 'v0_32_3-tests-zero-copy-programs-zero-cpi'));
function diskCheck(completed) {
  const disk = fs.statfsSync(here, {bigint:true});
  const free = disk.bavail * disk.bsize;
  console.log(`Disk free after ${completed} programs: ${free} bytes (${(Number(free)/1e9).toFixed(2)} GB)`);
  if (free < 40_000_000_000n) { console.error('Stopped: disk free is below 40 GB.'); return false; }
  return true;
}
if (!diskCheck(0)) process.exit(2);
for (const id of args) if (!manifest.programs.some(program => program.id === id)) throw new Error(`Unknown corpus program: ${id}`);
if (!fs.existsSync(tare)) throw new Error(`Build the CLI first: cargo build --release --manifest-path ${path.join(here, '../toolchain/Cargo.toml')} -p tare`);
const repoFor = program => ({ ...manifest, ...program, ...(program.local && process.env.TARE_REGISTRY_SOURCE ? {checkout:path.relative(here,process.env.TARE_REGISTRY_SOURCE)} : {}) });
const cwdFor = program => path.join(here,repoFor(program).checkout,program.workspace);
const checked = new Set();
for (const program of selected) {
  const repo = repoFor(program), checkout = path.join(here, repo.checkout);
  if (program.local) continue;
  if (checked.has(checkout)) continue;
  checked.add(checkout);
  const revision = spawnSync('git', ['-c',`safe.directory=${checkout}`,'-C', checkout, 'rev-parse', 'HEAD'], { encoding: 'utf8' });
  if (revision.status !== 0 || revision.stdout.trim() !== repo.revision) throw new Error(`Corpus must be at ${repo.revision}: ${checkout}`);
  const dirty = spawnSync('git', ['-c',`safe.directory=${checkout}`,'-C', checkout, 'diff', 'HEAD', '--name-only', '--', '*.rs', '*Cargo.toml', '*Cargo.lock'], { encoding: 'utf8' });
  if (dirty.status !== 0 || dirty.stdout.trim()) throw new Error(`Corpus source or manifests changed: ${dirty.stdout.trim()}`);
  const untracked = spawnSync('git', ['-c',`safe.directory=${checkout}`,'-C',checkout,'ls-files','--others','--exclude-standard','--','*.rs','*Cargo.toml','*Cargo.lock'], {encoding:'utf8'});
  if(untracked.status!==0||untracked.stdout.trim()) throw new Error(`Untracked corpus source or manifests: ${untracked.stdout.trim()}`);
}
const versionRun = spawnSync(tare, ['--version'], { encoding: 'utf8' });
if (versionRun.status !== 0) throw new Error(versionRun.stderr || 'Could not read tare version');
const version = versionRun.stdout.trim().replace(/^tare\s+/, '');
fs.mkdirSync(path.join(here,'transcripts'),{recursive:true});
fs.writeFileSync(path.join(here,'transcripts','tare-version.txt'),'$ tare --version\n'+terminalText(versionRun.stdout));
const resultFile = process.env.TARE_BENCH_RESULTS ? path.resolve(process.env.TARE_BENCH_RESULTS) : path.join(here, 'results.json');
const previous = fs.existsSync(resultFile) ? JSON.parse(fs.readFileSync(resultFile, 'utf8')) : { programs: [] };
const rows = new Map(previous.programs.filter(p => manifest.programs.some(m => m.id === p.id)).map(p => [p.id, p]));
fs.mkdirSync(path.join(here, 'transcripts'), { recursive: true });

function validate(schemaName, file) {
  const result = spawnSync(process.env.PYTHON || 'python', [path.join(here, 'validate.py'), schemaName, file], { encoding: 'utf8' });
  if (result.status !== 0) throw new Error(`Schema validation failed: ${result.stderr || result.stdout}`);
  process.stdout.write(result.stdout);
}
function writeJson(file, value) {
  const temporary = file + ".tmp";
  fs.writeFileSync(temporary, JSON.stringify(value, null, 2) + "\n");
  fs.renameSync(temporary, file);
}
function sumWeights(out, values) {
  for (const key of ['gross', 'net', 'tare', 'inputs']) out[key] += values[key] || 0;
}
function writeResults() {
  const programs = manifest.programs.flatMap(program => rows.has(program.id) ? [rows.get(program.id)] : []);
  for (const program of programs) {
    for (const source of [program.source, program.weighing?.source]) {
      if (typeof source?.repository === 'string') source.repository = canonicalRepository(source.repository);
    }
  }
  // The benchmark feed carries measured facts; full replay bundles remain in each archived proof.
  for (const program of programs) if (program.status === 'measured') delete program.weighing.replay;
  const measured = programs.filter(program => program.status === 'measured');
  const totals = { programs: programs.length, measured: measured.length, notBuilt: programs.length - measured.length, instructions: 0, weights: { gross: 0, net: 0, tare: 0, inputs: 0 }, binarySize: { gross: 0, net: 0, tare: 0 }, rentLamports: { gross: 0, net: 0, tare: 0 }, corpus: { tests: 0, generated: 0, mainnet: 0 } };
  for (const program of measured) {
    const weighing = program.weighing;
    totals.instructions += weighing.instructions.length;
    sumWeights(totals.weights, weighing.totals);
    for (const [key, field] of [['binarySize', 'sizeBytes'], ['rentLamports', 'rentLamports']]) {
      const gross = weighing.builds.stock[field], net = weighing.builds.tare[field];
      totals[key].gross += gross; totals[key].net += net; totals[key].tare += gross - net;
    }
    for (const key of ['tests', 'generated', 'mainnet']) totals.corpus[key] += program.proof.corpus.counts[key];
  }
  const generatedAt = new Date().toISOString();
  writeJson(resultFile, { schemaVersion: '1', fixture: false, toolchainVersion: version, generatedAt, programs, totals });
  const versions = [...new Set(programs.map(program => program.anchorVersion))];
  const names = [...new Set([...manifest.programs.flatMap(p => p.constructs), ...programs.filter(p => p.status === 'not_built' && p.construct).map(p => p.construct)])].sort();
  const constructs = names.map(construct => ({ construct, versions: versions.map(anchorVersion => {
    const exercises = measured.filter(program => program.anchorVersion === anchorVersion && program.proof.status === 'matched' && program.weighing.totals.inputs > 0 && manifest.programs.find(m => m.id === program.id).constructs.includes(construct)).map(program => program.id);
    const refusals = programs.filter(program => program.anchorVersion === anchorVersion && program.status === 'not_built' && program.construct === construct);
    return { anchorVersion, supported: exercises.length > 0, programs: exercises, note: exercises.length ? 'Compiled in programs with matched proofs and successful generated instruction samples; this is not branch coverage or full language conformance.' : refusals.length ? refusals.map(program => `${program.id}: ${program.reason}`).join('\n') : 'Not exercised by a matched corpus run.' };
  }) }));
  const compatFile = process.env.TARE_BENCH_RESULTS ? resultFile.replace(/\.json$/, '-compat.json') : path.join(here, 'compat.json');
  writeJson(compatFile, { schemaVersion: '1', fixture: false, toolchainVersion: version, generatedAt, anchorVersions: versions, constructs });
  validate('BenchResults', resultFile);
  validate('Compat', compatFile);
}
function testResults(program, report) {
  const file = path.join(here, 'upstream-tests.json');
  if (program.local) {
    const native=path.join(here,repoFor(program).checkout,'target/native-tests.json');
    if(fs.existsSync(native)){const value=JSON.parse(fs.readFileSync(native));if(value.tareBuildHash===report.builds.tare.hash)return {ran:value.transactions,passed:value.status==='matched'?value.transactions:0,failed:value.status==='matched'?0:value.transactions};}
    return {ran:0,passed:0,failed:0};
  }
  if (!fs.existsSync(file)) return { ran: 0, passed: 0, failed: 0 };
  const runs = JSON.parse(fs.readFileSync(file, 'utf8')).reports;
  const run = runs.findLast(run => run.sourceCommit === program.revision &&
    run.programs?.some(p => p.id === program.id && p.tareBuildHash === report.builds.tare.hash));
  return run?.tests || { ran: 0, passed: 0, failed: 0 };
}
async function command(program, name, log) {
  const checkout = path.join(here, repoFor(program).checkout);
  const cwd=path.join(checkout,program.workspace);
  const commandArgs=[name,'--host','--manifest-path',path.relative(cwd,path.join(checkout,program.manifest))];
  if(program.suppliedIdl)commandArgs.push('--idl',path.resolve(here,program.suppliedIdl));
  if(program.cargoFeatures?.length)commandArgs.push('--features',program.cargoFeatures.join(','));
  if(program.productionCorpus && name !== 'build')commandArgs.push('--replay','target/tare/production-replay.json');
  if(program.local&&name!=='build')commandArgs.push('--replay','target/native-replay.json');
  const headline=`$ tare ${commandArgs.map(value=>path.isAbsolute(value)?path.relative(cwd,value):value).join(' ')}\n`;
  process.stdout.write(headline);log.write(headline);
  const child=spawn(tare,commandArgs,{cwd,env:{...process.env,...program.environment},stdio:['ignore','pipe','pipe']});
  const chunks=[];
  for(const stream of [child.stdout,child.stderr])stream.on('data',chunk=>{chunks.push(chunk);process.stdout.write(chunk)});
  const code=await new Promise((resolve,reject)=>{child.once('error',reject);child.once('close',resolve)});
  const output=Buffer.concat(chunks).toString('utf8');
  const captured=terminalText(output,cwd)+`\n[exit ${code}]\n`;
  log.write(captured);
  fs.writeFileSync(path.join(here,'transcripts',`${program.id}-${name}.txt`),headline+captured);
  return {code,output};
}
if (process.env.TARE_BENCH_REFRESH === '1') { writeResults(); process.exit(0); }
let failed = false;
let completed = 0;
let diskStopped = false;
const measureOnly = process.env.TARE_BENCH_MEASURE_ONLY === '1';
async function runProgram(program) {
  if (process.env.TARE_BENCH_STOP_FILE && fs.existsSync(process.env.TARE_BENCH_STOP_FILE)) { console.log('Stopped at a completed program boundary.'); diskStopped = true; return; }
  if (measureOnly && rows.get(program.id)?.status !== 'measured') return;
  const transcript = fs.createWriteStream(path.join(here, 'transcripts', `${program.id}.txt`));
  const repo = repoFor(program);
  const checkout = path.join(here, repo.checkout);
  if(measureOnly){const saved=path.join(here,'transcripts',`${program.id}-build.txt`);if(fs.existsSync(saved))transcript.write(fs.readFileSync(saved,'utf8'));}
  if(!measureOnly)transcript.write("$ tare --version\n"+terminalText(versionRun.stdout));
  transcript.write(`Source: ${program.local ? "local registry workspace" : `${canonicalRepository(repo.repository)}/tree/${repo.revision}/${program.workspace}`}\nRecorded: ${new Date().toISOString()}\n`);
  const common = { id: program.id, name: program.name, kind: program.kind, anchorVersion: program.anchorVersion, source: { repository: canonicalRepository(repo.repository), commit: repo.revision } };
  try {
    if (measureOnly) {
      const expected=rows.get(program.id).weighing.builds;
      for(const [kind,folder] of [['stock','deploy'],['tare','tare']]) {
        const file=path.join(checkout,program.workspace,'target',folder,`${program.name}.so`);
        const hash=createHash('sha256').update(fs.readFileSync(file)).digest('hex');
        if(hash!==expected[kind].hash)throw new Error(`Existing ${kind} artifact changed; run the full benchmark first.`);
      }
    }
    const build = measureOnly ? {code:0,output:''} : await command(program, 'build', transcript);
    if (build.code !== 0) {
      const construct = build.output.match(/Tare does not (?:yet )?(?:support(?: construct)?|implement)\s+[`'"]?([^`'"\n]+)|unsupported (?:construct|Anchor version)[: ]+[`'"]?([^`'"\n]+)/i);
      const lines = build.output.split(/\r?\n/);
      const errors = lines.filter(line => /^(?:error(?:\[|:)|.*Tare does not|.*unsupported)/i.test(line));
      const unsupportedLine = lines.findIndex(line => /Tare does not (?:yet )?(?:support|implement)/i.test(line));
      const firstDiagnostic = lines.findIndex(line => /^error\[/.test(line));
      const reason = unsupportedLine >= 0 ? lines.slice(unsupportedLine, unsupportedLine + 12).join('\n').trim() : firstDiagnostic >= 0 ? lines.slice(firstDiagnostic, firstDiagnostic + 18).join('\n').trim() : errors.slice(0, 8).join('\n');
      const named = /unsupported Anchor version ([0-9.]+)/.test(build.output) ? `Anchor ${build.output.match(/unsupported Anchor version ([0-9.]+)/)[1]}` : construct ? (construct[1] || construct[2]).trim().split(/\s+at\s+/)[0]
        : /RefMut[\s\S]*DataRefMut/.test(build.output) ? 'std::cell::RefMut account-data guard'
        : /ExtraAccountMetaList[\s\S]*AccountInfo/.test(build.output) ? 'SPL AccountInfo interoperability'
        : /overflows the maximum allowed frame space/.test(build.output) ? 'SBF stack frame > 4096 bytes'
        : /build stock IDL:/.test(build.output) ? 'stock IDL validation'
        : /failed to select a version for/.test(build.output) ? 'upstream dependency resolution'
        : /all-zero or empty discriminators/.test(build.output) ? 'stock all-zero discriminator rejection'
        : /evaluation panicked.*instruction|expects MORE args/s.test(build.output) ? 'stock instruction validation' : null;
      rows.set(program.id, { ...common, status: 'not_built', reason: reason || `Build exited ${build.code}; see transcripts/${program.id}.txt.`, construct: named });
      failed = true;
    } else {
      if(program.productionCorpus){
        const generate=spawnSync('cargo',['run','--locked','--manifest-path',path.join(here,'production/Cargo.toml'),'--',program.productionCorpus,cwdFor(program),path.join(cwdFor(program),'target/tare/production-replay.json')],{cwd:here,env:{...process.env,CARGO_TARGET_DIR:path.join(process.env.TARE_HOME||path.join(here,'../toolchain'),'target/native-versions')},encoding:'utf8',maxBuffer:64*1024*1024});
        if(generate.status!==0)throw new Error(generate.stderr||generate.stdout||'Production corpus generation failed');
      }
      if(program.local){
        const env={...process.env,TARE_REGISTRY_SOURCE:checkout,CARGO_TARGET_DIR:path.join(process.env.TARE_HOME||path.join(here,'../toolchain'),'target/native-versions')};
        const native=spawnSync('cargo',['run','--manifest-path',path.join(checkout,'tests/Cargo.toml')],{cwd:path.dirname(here),env,encoding:'utf8',maxBuffer:64*1024*1024});
        const captured=terminalText(`$ tare build --host --manifest-path ${program.manifest}\nNative registry verification of the preceding successful build:\n$ cargo run --manifest-path registry/tests/Cargo.toml\n${native.stdout||''}${native.stderr||''}\n[exit ${native.status}]\n`,checkout);
        fs.writeFileSync(path.join(here,'transcripts',`${program.id}-tests.txt`),captured);transcript.write(captured);
        if(native.status!==0)throw new Error('registry native tests failed');
      }
      const target = path.join(checkout, program.workspace, 'target', 'tare');
      fs.rmSync(path.join(target, 'weigh.json'), { force: true });
      const weigh = await command(program, 'weigh', transcript);
      if (!fs.existsSync(path.join(target, 'weigh.json'))) {
        const reason = weigh.output.split(/\r?\n/).filter(line => /^error:/.test(line)).join('\n');
        throw new Error(reason || `weigh exited ${weigh.code}`);
      }
      validate('Weighing', path.join(target, 'weigh.json'));
      const proofFile = path.join(target, 'proof.json');
      fs.rmSync(proofFile, { force: true });
      const proof = await command(program, 'prove', transcript);
      validate('ProofReport', proofFile);
      const report = JSON.parse(fs.readFileSync(proofFile, 'utf8'));
      if (!process.env.TARE_BENCH_NO_ARCHIVE) {
      const archive=path.join(here,'artifacts',program.id);fs.mkdirSync(archive,{recursive:true});
      const copies=[['stock.so',path.join(checkout,program.workspace,'target/deploy',`${program.name}.so`)],['tare.so',path.join(target,`${program.name}.so`)],['linked.so',path.join(target,`${program.name}.linked.so`)],['tare-symbols.so',path.join(target,`${program.name}.symbols.so`)],['stock-symbols.so',path.join(target,`${program.name}.stock-symbols.so`)],['idl.json',path.join(checkout,program.workspace,'target/idl',`${program.name}.json`)],['corpus.json',path.join(target,`${program.name}.corpus.json`)],['proof.json',proofFile]];
      for(const [name,source]of copies)if(fs.existsSync(source))fs.copyFileSync(source,path.join(archive,name));
      fs.writeFileSync(path.join(archive,'builds.json'),JSON.stringify(report.builds,null,2)+'\n');
      }
      const { firstDivergence, ...summary } = report.proof;
      const previousRow = rows.get(program.id);
      const sameBuild = previousRow?.status === 'measured' && ['stock','tare'].every(kind => previousRow.weighing.builds[kind].hash === report.builds[kind].hash) && previousRow.proof.corpus.hash === summary.corpus.hash;
      rows.set(program.id, { ...common, status: 'measured', weighing: report, tests: testResults(repo, report), proof: summary, record: sameBuild ? previousRow.record : null, ...(sameBuild && previousRow.recordCluster ? {recordCluster:previousRow.recordCluster} : {}), ...(sameBuild && previousRow.recordUrl ? {recordUrl:previousRow.recordUrl} : {}) });
      if (proof.code !== 0 || report.proof.status !== 'matched') failed = true;
    }
    writeResults();
  } catch (error) {
    failed = true;
    transcript.write(terminalText(`Runner error: ${error.stack || error.message}\n`));
    rows.set(program.id, { ...common, status: 'not_built', reason: `Pipeline incomplete: ${error.message}; see transcripts/${program.id}.txt.`, construct: /(?:generated corpus|IDL (?:type|definition|const generic|array))/.test(error.message) ? 'generated corpus account/type resolution' : null });
    writeResults();
  } finally { await new Promise(resolve => transcript.end(resolve)); }
  completed++;
  if (completed % 20 === 0 && !diskCheck(completed)) diskStopped = true;
}
// Each Anchor version has exactly one active build against its shared cache.
const queues = [...Map.groupBy(selected, program => program.anchorVersion).values()];
async function worker() {
  while (!diskStopped && queues.length) {
    const queue = queues.shift();
    for (const program of queue) {
      if (diskStopped) break;
      await runProgram(program);
    }
  }
}
await Promise.all(Array.from({length:jobs}, worker));
if (!diskStopped && completed % 20 !== 0) diskCheck(completed);
const finalRows = [...rows.values()];
const matchingProofs = finalRows.filter(row => row.status === 'measured' && row.proof.status === 'matched').length;
const divergingProofs = finalRows.filter(row => row.status === 'measured' && row.proof.status !== 'matched').length;
const notBuilt = finalRows.filter(row => row.status === 'not_built').length;
console.log(`Recorded ${rows.size} programs in ${resultFile}: ${matchingProofs} matching proofs, ${divergingProofs} diverging proofs, ${notBuilt} not built`);
process.exitCode = diskStopped ? 2 : failed ? 1 : 0;
