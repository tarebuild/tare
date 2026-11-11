# Rebuilds the site and restarts the standing production server on 127.0.0.1:7801.
# Stops only the server this script started before (its PIDs are kept in .next-prod.pid).
$ErrorActionPreference = 'Continue'
$web = Split-Path -Parent $PSScriptRoot
$pidFile = Join-Path $web '.next-prod.pid'
if (Test-Path $pidFile) {
  foreach ($id in (Get-Content $pidFile)) {
    if ($id) { try { Stop-Process -Id ([int]$id) -Force -ErrorAction Stop } catch {} }
  }
}
$env:CIRCLE_NODE_TOTAL = '5'
$env:NEXT_PUBLIC_SITE_URL = 'http://127.0.0.1:7801'
$env:API_URL = 'http://127.0.0.1:7810'
# The local install line, as the API's /v1/config gives it. In production it carries the API's public origin.
$env:NEXT_PUBLIC_INSTALL_LINE = 'curl -fsSL http://127.0.0.1:7810/install.sh | sh'
Set-Location $web
cmd /c "npx next build > .next-build.log 2>&1"
if ($LASTEXITCODE -ne 0) { Get-Content .next-build.log -Tail 30; exit 1 }
$p = Start-Process cmd.exe -ArgumentList '/c', 'npx next start --hostname 127.0.0.1 --port 7801 > .next-prod.log 2>&1' -WorkingDirectory $web -WindowStyle Hidden -PassThru
Start-Sleep -Seconds 5
$ids = @($p.Id)
$queue = @($p.Id)
while ($queue.Count -gt 0) {
  $next = @()
  foreach ($q in $queue) {
    $kids = Get-CimInstance Win32_Process -Filter "ParentProcessId=$q" | Select-Object -ExpandProperty ProcessId
    foreach ($k in $kids) { $ids += $k; $next += $k }
  }
  $queue = $next
}
$ids | Out-File -Encoding ascii $pidFile
try { (Invoke-WebRequest -UseBasicParsing http://127.0.0.1:7801/ -TimeoutSec 20).StatusCode } catch { $_.Exception.Message }
