#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const here = path.dirname(fileURLToPath(import.meta.url));
const manifest = JSON.parse(fs.readFileSync(path.join(here, 'manifest.json'), 'utf8'));
const files = process.argv.slice(2);
const inputs = files.length ? files.map(file => path.resolve(file)) : [path.join(here, 'results.json')];
const rows = new Map();
for (const file of inputs) {
  if (!fs.existsSync(file)) continue;
  const document = JSON.parse(fs.readFileSync(file, 'utf8'));
  for (const row of document.programs) {
    const entry = manifest.programs.find(entry => entry.id === row.id);
    if (!entry || entry.anchorVersion !== row.anchorVersion || entry.revision !== row.source.commit) throw new Error(`Manifest mismatch: ${row.id}`);
    if (rows.has(row.id)) throw new Error(`Duplicate partition row: ${row.id}`);
    rows.set(row.id, row);
  }
}
const staging = path.join(here, '.results-merge.json');
fs.writeFileSync(staging, JSON.stringify({programs: [...rows.values()]}, null, 2) + '\n');
const refresh = spawnSync(process.execPath, [path.join(here, 'run.mjs')], {
  env: {...process.env, TARE_BENCH_REFRESH: '1', TARE_BENCH_RESULTS: staging}, stdio: 'inherit'
});
if (refresh.status === 0) {
  fs.renameSync(staging, path.join(here, 'results.json'));
  fs.renameSync(staging.replace(/\.json$/, '-compat.json'), path.join(here, 'compat.json'));
}
process.exitCode = refresh.status;
