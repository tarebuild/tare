$ErrorActionPreference = 'Stop'
& node (Join-Path $PSScriptRoot 'fetch.mjs')
exit $LASTEXITCODE
