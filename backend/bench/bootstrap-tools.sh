#!/bin/sh
set -eu
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
mkdir -p "$here/tools/node" "$here/tools/yarn"
if [ ! -x "$here/tools/node/bin/node" ]; then
  curl --fail --location --retry 3 https://nodejs.org/dist/v22.23.4/node-v22.23.4-linux-x64.tar.gz -o "$here/tools/node.tar.gz"
  echo "b8492284c76a54baf58d61668dd0e41ee0fc98adabf9503d30f05d08f55b4406  $here/tools/node.tar.gz" | sha256sum --check --strict
  tar -xzf "$here/tools/node.tar.gz" --strip-components=1 -C "$here/tools/node"
  rm "$here/tools/node.tar.gz"
fi
PATH="$here/tools/node/bin:$here/tools/yarn/node_modules/.bin:$PATH"
export PATH
if [ ! -x "$here/tools/yarn/node_modules/.bin/yarn" ]; then
  npm install --prefix "$here/tools/yarn" --no-save --package-lock=false yarn@1.22.22
fi
