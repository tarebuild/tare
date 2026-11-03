#!/usr/bin/env bash
# Run the corpus benchmark and the programs' own validator test suites inside the pinned
# image, on a copy of backend/bench in the volume tare-bench-work (so a Windows build tree
# and a Linux one never share a target directory). No ports are published.
#
# usage: dev/bench-in-image.sh <log-file> <program> [<program> ...]
# Results are copied back to backend/bench/linux/.
set -u
export MSYS_NO_PATHCONV=1
root=$(cd "$(dirname "$0")/.." && pwd)
command -v cygpath >/dev/null && root=$(cygpath -m "$root")
log=$1; shift
image=${TARE_IMAGE:-tare-toolchain:0.1.0}
name=tare-bench-$$

docker volume create tare-bench-work >/dev/null
# Copy the sources in, leaving build output and installed tools behind.
(cd "$root/backend" && tar --exclude='target' --exclude='node_modules' --exclude='bench/tools' \
  --exclude='bench/linux' --exclude='bench/corpus' -cf - bench contract/schema) |
  docker run --rm -i --entrypoint tar -v tare-bench-work:/work "$image" -xf - -C /work

docker run --name "$name" --entrypoint bash -e TARE_ISOLATED_TEST_NETWORK=1 \
  -v tare-bench-work:/work -w /work "$image" -c '
    set -e
    sed -i "s/\r$//" bench/*.sh bench/run
    git config --global --add safe.directory "*"
    sh bench/fetch.sh
    sh bench/setup-upstream-tests.sh
    apt-get -o Acquire::Check-Valid-Until=false update >/dev/null
    apt-get install -y --no-install-recommends python3 python3-jsonschema >/dev/null
    export PYTHON=python3
    export PATH="/work/bench/tools/node/bin:/work/bench/tools/yarn/node_modules/.bin:$PATH"
    export TARE_BIN=/usr/local/bin/tare
    node bench/run.mjs "$@"
    sh bench/run-upstream-tests.sh "$@"
  ' bench "$@" > "$log" 2>&1
status=$?

mkdir -p "$root/backend/bench/linux"
for f in results.json compat.json upstream-tests.json transcripts corpus-data; do
  docker cp "$name:/work/bench/$f" "$root/backend/bench/linux/" >/dev/null 2>&1
done
docker rm "$name" >/dev/null
echo "exit=$status log=$log"
exit $status
