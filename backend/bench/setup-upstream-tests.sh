#!/bin/sh
set -eu
if [ "${TARE_ISOLATED_TEST_NETWORK:-}" != 1 ]; then
  echo "Run inside the isolated Linux test container without published ports." >&2
  exit 1
fi
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
sh "$here/bootstrap-tools.sh"
export PATH="$here/tools/node/bin:$here/tools/yarn/node_modules/.bin:$PATH"
exec node "$here/recording/setup.mjs" "$@"
