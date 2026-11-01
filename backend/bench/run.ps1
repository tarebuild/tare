$ErrorActionPreference = 'Stop'
& node (Join-Path $PSScriptRoot 'run.mjs') @args
exit $LASTEXITCODE
