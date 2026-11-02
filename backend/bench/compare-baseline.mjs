#!/usr/bin/env node
// Compare candidate ELFs against round two using the exact archived inputs.
import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const usage = `Usage: node bench/compare-baseline.mjs [--results FILE] [--cohort VERSION] [--helper FILE] [PROGRAM_ID ...]

Defaults: bench/results.json and toolchain/target/release/examples/fixed_corpus.
The frozen input index is bench/gaps/round-three/opt/fixed-corpus/index.json.
Native reports go to bench/gaps/round-three/performance/<id>.json unchanged.
Driver metadata goes to <id>.comparison.json.
Cohort runs write summary-<version>.json; an unfiltered run writes summary.json.
No build or corpus generation is performed.`;
const args = process.argv.slice(2);
let resultFile = process.env.TARE_BENCH_RESULTS ? path.resolve(process.env.TARE_BENCH_RESULTS) : path.join(here, 'results.json');
let helper = process.env.TARE_FIXED_CORPUS || path.join(here, '../toolchain/target/release/examples', process.platform === 'win32' ? 'fixed_corpus.exe' : 'fixed_corpus');
let cohort;
const ids = [];
for (let i = 0; i < args.length; i++) {
  const arg = args[i];
  if (arg === '--help' || arg === '-h') { console.log(usage); process.exit(0); }
  if (['--results', '--cohort', '--helper'].includes(arg)) {
    const value = args[++i];
    if (!value || value.startsWith('--')) throw new Error(`Missing value for ${arg}`);
    if (arg === '--results') resultFile = path.resolve(value);
    if (arg === '--cohort') cohort = value;
    if (arg === '--helper') helper = path.resolve(value);
  } else if (arg.startsWith('-')) {
    throw new Error(`Unknown option ${arg}\n${usage}`);
  } else ids.push(arg);
}
helper = path.resolve(helper);
const readJson = file => JSON.parse(fs.readFileSync(file, 'utf8'));
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const hashFile = file => hash(fs.readFileSync(file));
const writeJson = (file, value) => {
  const temporary = `${file}.${process.pid}.tmp`;
  fs.writeFileSync(temporary, JSON.stringify(value, null, 2) + '\n');
  fs.renameSync(temporary, file);
};
const manifest = readJson(path.join(here, 'manifest.json'));
const baselineIndexPath = path.join(here, 'gaps/round-three/opt/fixed-corpus/index.json');
const baselineIndex = readJson(baselineIndexPath);
if (baselineIndex.errors?.length) throw new Error(`Baseline preparation has ${baselineIndex.errors.length} unresolved errors`);
const baseline = new Map(baselineIndex.programs.map(program => [program.id, program]));
const currentManifest = new Map(manifest.programs.map(program => [program.id, program]));
const newResults = readJson(resultFile);
if (!Array.isArray(newResults.programs)) throw new Error('Candidate result file has no programs array');
const candidates = new Map(newResults.programs.map(program => [program.id, program]));
for (const id of ids) {
  if (!baseline.has(id)) throw new Error(`No frozen round-two baseline for ${id}`);
  if (cohort && baseline.get(id).anchorVersion !== cohort) throw new Error(`${id} is not in cohort ${cohort}`);
}
const selected = baselineIndex.programs.filter(program => (!cohort || program.anchorVersion === cohort) && (!ids.length || ids.includes(program.id)));
if (!selected.length) throw new Error('No baseline programs selected');
if (cohort && !/^[A-Za-z0-9._-]+$/.test(cohort)) throw new Error('Invalid cohort name');
if (!fs.existsSync(helper)) throw new Error(`Build the helper first: cargo build --release --manifest-path toolchain/Cargo.toml -p tare-prove --example fixed_corpus`);
const helperHash = hashFile(helper);
const directory = path.join(here, 'gaps/round-three/performance');
fs.mkdirSync(directory, { recursive: true });
const summaryFile = path.join(directory, cohort ? `summary-${cohort}.json` : ids.length ? 'summary-selected.json' : 'summary.json');
const records = [];
function summary() {
  const counts = {};
  for (const record of records) counts[record.status] = (counts[record.status] || 0) + 1;
  writeJson(summaryFile, {
    schemaVersion: '1', generatedAt: new Date().toISOString(),
    source: { candidateResults: resultFile, baselineIndex: baselineIndexPath, helper, helperSha256: helperHash },
    cohort: cohort || null, requestedPrograms: selected.length, completedPrograms: records.length,
    counts, passed: records.length === selected.length && records.every(record => record.status === 'matched'),
    programs: records,
  });
}
function failure(id, status, message, details = {}) {
  return { id, status, error: message, ...details };
}
function artifactCandidates(program, row) {
  const checkout = path.resolve(here, program.checkout || manifest.checkout);
  const target = path.join(checkout, program.workspace, 'target/tare');
  const name = row.weighing?.program?.name || program.name;
  const preferred = path.join(target, `${name}.so`);
  const files = fs.existsSync(target) ? fs.readdirSync(target).filter(name => name.endsWith('.so')).map(name => path.join(target, name)) : [];
  const archived = path.join(here, 'artifacts', program.id, 'tare.so');
  return [...new Set([archived, preferred, ...files])].filter(file => fs.existsSync(file));
}
async function invoke(args, logPath) {
  const stream = fs.createWriteStream(logPath);
  stream.write(`$ ${[helper, ...args].map(arg => JSON.stringify(arg)).join(' ')}\n`);
  try {
    const child = spawn(helper, args, { stdio: ['ignore', 'pipe', 'pipe'] });
    child.stdout.on('data', chunk => stream.write(chunk));
    child.stderr.on('data', chunk => stream.write(chunk));
    return await new Promise((resolve, reject) => { child.once('error', reject); child.once('close', resolve); });
  } finally {
    await new Promise(resolve => stream.end(resolve));
  }
}

for (const prior of selected) {
  const id = prior.id;
  if (!/^[A-Za-z0-9._-]+$/.test(id)) throw new Error(`Unsafe program ID ${id}`);
  const reportFile = path.join(directory, `${id}.json`);
  const logFile = path.join(directory, `${id}.log`);
  const comparisonFile = path.join(directory, `${id}.comparison.json`);
  fs.rmSync(reportFile, { force: true });
  let record;
  try {
    const row = candidates.get(id);
    if (!row) {
      record = failure(id, 'missing-result', 'Candidate result file has no row for this frozen baseline');
    } else if (row.status !== 'measured') {
      record = failure(id, 'new-program-unavailable', row.reason || `Candidate status is ${row.status}`, { construct: row.construct || null });
    } else if (row.anchorVersion !== prior.anchorVersion) {
      record = failure(id, 'cohort-mismatch', 'Candidate and frozen baseline use different Anchor versions', { baseline: prior.anchorVersion, candidate: row.anchorVersion });
    } else if (!currentManifest.has(id)) {
      record = failure(id, 'missing-manifest', 'Current corpus manifest has no artifact location for this ID');
    } else {
      const expected = row.weighing?.builds?.tare?.hash;
      if (!/^[0-9a-f]{64}$/.test(expected || '')) throw new Error('Candidate result lacks a valid Tare SHA-256');
      if (hashFile(prior.before) !== prior.beforeSha256) throw new Error('Frozen baseline ELF SHA-256 differs from its index');
      if (hashFile(prior.corpus) !== prior.corpusFileSha256) throw new Error('Frozen corpus SHA-256 differs from its index');
      const artifacts = artifactCandidates(currentManifest.get(id), row).map(file => ({ path: file, sha256: hashFile(file), bytes: fs.statSync(file).size }));
      const artifact = artifacts.find(file => file.sha256 === expected);
      if (!artifact) {
        record = failure(id, 'stale-artifact', 'No candidate ELF matches the Tare hash in the new result record', { expectedSha256: expected, artifacts });
      } else if (artifact.bytes !== row.weighing.builds.tare.sizeBytes) {
        record = failure(id, 'result-size-mismatch', 'Candidate hash matches but the result record has a different binary size', { artifact, recordedBytes: row.weighing.builds.tare.sizeBytes });
      } else {
        const exitCode = await invoke([prior.address, prior.before, artifact.path, prior.corpus, prior.programs, reportFile], logFile);
        const reportBytes = fs.existsSync(reportFile) ? fs.readFileSync(reportFile) : null;
        const report = reportBytes ? JSON.parse(reportBytes.toString('utf8')) : null;
        if (!report || !Array.isArray(report.instructions) || !report.comparisons || !Object.hasOwn(report, 'firstDivergence')) {
          record = failure(id, 'helper-error', 'fixed_corpus did not produce a complete RunReport', { exitCode, log: path.relative(here, logFile) });
        } else {
          const regressions = report.instructions.filter(ix => ix.inputs > 0 && Number.isFinite(ix.gross) && Number.isFinite(ix.net) && ix.net > ix.gross).map(ix => ({ instruction: ix.name, inputs: ix.inputs, before: ix.gross, after: ix.net, increase: ix.net - ix.gross }));
          let status = report.firstDivergence !== null ? 'diverged' : regressions.length ? 'regressed' : exitCode !== 0 ? 'helper-error' : 'matched';
          if (report.fixedCorpus?.afterSha256 !== expected) status = 'stale-artifact';
          if (report.fixedCorpus?.beforeSha256 !== prior.beforeSha256 || report.fixedCorpus?.corpusFileSha256 !== prior.corpusFileSha256) status = 'stale-baseline';
          const candidateProofStatus = row.proof?.status || row.weighing?.proof?.status || null;
          if (candidateProofStatus !== 'matched' && status === 'matched') status = 'new-corpus-not-matched';
          const error = status === 'stale-artifact' ? 'Candidate ELF changed between hash selection and helper execution' : status === 'stale-baseline' ? 'Helper input hashes differ from the frozen baseline index' : undefined;
          record = { id, anchorVersion: prior.anchorVersion, status, ...(error ? { error } : {}), inputs: report.comparisons.inputs, measuredInstructions: report.instructions.filter(ix => ix.inputs > 0).length, medianRegressions: regressions, exitCode, beforeSha256: prior.beforeSha256, afterSha256: expected, candidateArtifact: artifact.path, candidateProofStatus, nativeReportSha256: hash(reportBytes), report: path.relative(here, reportFile), log: path.relative(here, logFile) };
        }
      }
    }
  } catch (error) {
    record = failure(id, 'comparison-error', error.message, { log: path.relative(here, logFile) });
  }
  if (!record.report) record.report = fs.existsSync(reportFile) ? path.relative(here, reportFile) : null;
  record.comparison = path.relative(here, comparisonFile);
  // Keep the Rust-written report byte-for-byte: JSON.parse would round u64
  // account fields such as rent_epoch if this object were stringified again.
  writeJson(comparisonFile, { baselineComparison: { ...record, comparedAt: new Date().toISOString(), candidateResults: resultFile } });
  records.push(record);
  summary();
  console.log(`${id}: ${record.status}${record.inputs === undefined ? '' : ` (${record.inputs} inputs)`}`);
}
const failed = records.filter(record => record.status !== 'matched');
console.log(`Compared ${records.length} frozen baselines: ${records.length - failed.length} matched without median regression, ${failed.length} failed. Summary: ${summaryFile}`);
process.exitCode = failed.length ? 1 : 0;
