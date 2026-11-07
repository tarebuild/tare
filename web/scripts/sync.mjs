// Copies backend's benchmark results and compatibility matrix into the web app,
// so the site ships them and states no number the benchmark doesn't hold.
//   node scripts/sync.mjs
import { copyFileSync, existsSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const web = join(dirname(fileURLToPath(import.meta.url)), '..')
const pairs = [
  ['../backend/bench/results.json', 'lib/bench.json'],
  ['../backend/bench/compat.json', 'lib/compatibility.json'],
]
for (const [from, to] of pairs) {
  const src = join(web, from)
  if (!existsSync(src)) {
    console.log(`skip  ${from} (not written yet)`)
    continue
  }
  copyFileSync(src, join(web, to))
  console.log(`copy  ${from} -> ${to}`)
}
