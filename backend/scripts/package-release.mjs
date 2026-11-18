import { createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import { chmod, copyFile, mkdir, mkdtemp, readFile, rename, rm, stat, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const targets = new Set([
  'x86_64-unknown-linux-gnu', 'aarch64-unknown-linux-gnu',
  'x86_64-apple-darwin', 'aarch64-apple-darwin', 'x86_64-pc-windows-msvc',
])
const root = fileURLToPath(new URL('../', import.meta.url))
const releases = path.join(root, 'releases')

async function main() {
  const [source, target, version, ...extra] = process.argv.slice(2)
  if (!source || !target || !version || extra.length) {
    throw new Error('Usage: node scripts/package-release.mjs <binary> <target> <version>')
  }
  if (!targets.has(target)) throw new Error(`Unsupported release target: ${target}`)
  if (!/^[A-Za-z0-9][A-Za-z0-9._+-]*$/.test(version)) throw new Error('The release version is invalid.')
  const binary = path.resolve(source)
  if (!(await stat(binary)).isFile()) throw new Error('The binary path must name an existing file.')
  const tar = spawnSync('tar', ['--version'], { windowsHide: true, encoding: 'utf8' })
  if (tar.error || tar.status !== 0) throw new Error('tar is required to package a release.')

  await mkdir(releases, { recursive: true })
  const stage = await mkdtemp(path.join(releases, '.package-'))
  if (!path.resolve(stage).startsWith(path.resolve(releases) + path.sep)) throw new Error('The release staging path is invalid.')
  try {
    const name = target.endsWith('windows-msvc') ? 'tare.exe' : 'tare'
    const archive = `tare-${version}-${target}.tar.gz`
    const stagedBinary = path.join(stage, name)
    const stagedArchive = path.join(stage, archive)
    await copyFile(binary, stagedBinary)
    await chmod(stagedBinary, 0o755)
    const result = spawnSync('tar', ['-czf', archive, name], { cwd: stage, windowsHide: true, encoding: 'utf8' })
    if (result.error || result.status !== 0) throw new Error('tar could not create the release archive.')
    const sha256 = createHash('sha256').update(await readFile(stagedArchive)).digest('hex')
    await writeFile(path.join(stage, `${archive}.sha256`), `${sha256}  ${archive}\n`, 'utf8')
    await rename(stagedArchive, path.join(releases, archive))
    await rename(path.join(stage, `${archive}.sha256`), path.join(releases, `${archive}.sha256`))
    await writeFile(path.join(stage, 'latest'), `${version}\n`, 'utf8')
    await rename(path.join(stage, 'latest'), path.join(releases, 'latest'))
    console.log(`Packaged ${archive}`)
  } finally {
    await rm(stage, { recursive: true, force: true })
  }
}

try {
  await main()
} catch (error) {
  console.error(`tare release: ${error instanceof Error ? error.message : 'packaging failed.'}`)
  process.exitCode = 1
}
