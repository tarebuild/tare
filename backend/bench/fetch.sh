#!/bin/sh
set -eu
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
sh "$here/bootstrap-tools.sh"
export PATH="$here/tools/node/bin:$here/tools/yarn/node_modules/.bin:$PATH"
exec node "$here/fetch.mjs"
