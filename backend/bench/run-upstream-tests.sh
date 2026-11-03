#!/bin/sh
set -eu
if [ "${TARE_ISOLATED_TEST_NETWORK:-}" != 1 ]; then
  echo "Run in the isolated Linux container from bench/README.md; this command starts validators." >&2
  exit 1
fi
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
sh "$here/bootstrap-tools.sh"
export PATH="$here/tools/node/bin:$here/tools/yarn/node_modules/.bin:$PATH"
exec node "$here/recording/upstream.mjs" "$@"
