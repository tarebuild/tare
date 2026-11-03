#!/bin/sh
set -eu
if [ "${TARE_ISOLATED_TEST_NETWORK:-}" != 1 ]; then
  echo "Run in the Tare image with TARE_ISOLATED_TEST_NETWORK=1 and no published ports." >&2
  exit 1
fi
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$here/.."
export TARE_BIN="${TARE_BIN:-/usr/local/bin/tare}"
export PYTHON="${PYTHON:-python3}"
export NO_COLOR=1 CARGO_TERM_COLOR=never
export NODE_DISABLE_COLORS=1
status=0
sh "$here/bootstrap-tools.sh" || status=1
export PATH="$here/tools/node/bin:$here/tools/yarn/node_modules/.bin:$PATH"
if ! command -v node >/dev/null 2>&1; then
  printf 'Program | Recorded tx | Stock tests | Tare tests | Status\n'
  printf '%s | 0 | not run | not run | bootstrap failed\n' "${*:-all selected programs}"
  exit 1
fi
node --input-type=module -e 'import fs from "node:fs"; const marker="bench/tools/linux-run.json"; if(!fs.existsSync(marker)){fs.mkdirSync("bench/history/imported",{recursive:true}); for(const name of ["results.json","compat.json","upstream-tests.json","stock-tests.json","linux-summary.json"]){const src=`bench/${name}`;if(fs.existsSync(src))fs.renameSync(src,`bench/history/imported/${Date.now()}-${name}`)} fs.writeFileSync(marker,JSON.stringify({startedAt:new Date().toISOString(),platform:process.platform})+"\n")}'
if ! sh "$here/fetch.sh"; then
  TARE_LINUX_STAGES_PASSED=0 node "$here/linux-summary.mjs" "$@" || true
  exit 1
fi
# The production corpus helper shares the prover packaged in the image.
# A fresh work volume contains bench/ and contract/schema/, not toolchain/.
if [ ! -e "$here/../toolchain" ]; then
  ln -s "${TARE_HOME:-/opt/tare}" "$here/../toolchain"
fi
node "$here/production/prepare.mjs" "$@" || status=1
if [ -d /opt/tare-registry ]; then
  if [ ! -d "$here/corpus/tare-registry" ]; then cp -a /opt/tare-registry "$here/corpus/tare-registry"; fi
  export TARE_REGISTRY_SOURCE="$here/corpus/tare-registry"
fi
versions=$(node --input-type=module -e 'import fs from "node:fs"; const m=JSON.parse(fs.readFileSync("bench/manifest.json")); const ids=process.argv.slice(1); for(const id of ids) if(!m.programs.some(p=>p.id===id)) throw Error(`Unknown program: ${id}`); console.log([...new Set(m.programs.filter(p=>(!ids.length||ids.includes(p.id))&&!p.local&&!p.id.startsWith("production-")).map(p=>p.anchorVersion))].join(" "))' "$@")
# Versions are fixed numeric identifiers from the pinned manifest.
mkdir -p "$here/transcripts"
for version in $versions; do
  sh "$here/setup-upstream-tests.sh" "$version" >"$here/transcripts/setup-$version.txt" 2>&1 || status=1
  node --input-type=module -e 'import fs from "node:fs";import{terminalText}from"./bench/terminal.mjs";const file=process.argv[1];const text=terminalText(fs.readFileSync(file,"utf8"));fs.writeFileSync(file,text);process.stdout.write(text)' "$here/transcripts/setup-$version.txt"
done
node "$here/run.mjs" "$@" || status=1
if [ -n "$versions" ]; then
  if "$PYTHON" "$here/check-linux.py" >"$here/transcripts/validator-preflight.txt" 2>&1; then
    cat "$here/transcripts/validator-preflight.txt"
    sh "$here/record-tests.sh" "$@" || status=1
    sh "$here/run-upstream-tests.sh" "$@" || status=1
  else
    cat "$here/transcripts/validator-preflight.txt" >&2
    status=1
  fi
fi
node "$here/run.mjs" "$@" || status=1
TARE_LINUX_STAGES_PASSED=$((1-status)) node "$here/linux-summary.mjs" "$@" || status=1
exit "$status"
