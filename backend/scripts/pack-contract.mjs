import { spawnSync } from 'node:child_process'
import { mkdirSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
const root = fileURLToPath(new URL('../', import.meta.url))
mkdirSync(new URL('../contract/pack/', import.meta.url), { recursive: true })
const result = spawnSync(process.execPath, [process.env.npm_execpath, 'pack', '--workspace=@tare/contract', '--pack-destination', 'contract/pack'], { cwd: root, stdio: 'inherit', windowsHide: true })
process.exitCode = result.status ?? 1
