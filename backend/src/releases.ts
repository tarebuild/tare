import { open, readFile } from 'node:fs/promises'
import { Readable } from 'node:stream'
import type { Hono } from 'hono'
import { env } from './config.js'
import { fail } from './errors.js'

export function releaseRoutes(app: Hono<any>) {
  app.get('/install.sh', async (c) => {
    const base = env.RELEASES_BASE_URL ?? (env.API_PUBLIC_URL ? `${env.API_PUBLIC_URL}/releases` : undefined)
    if (!base) fail('CONFIGURATION_REQUIRED', 'Set API_PUBLIC_URL or RELEASES_BASE_URL to serve the installer.')
    const source = await readFile('install/install.sh', 'utf8')
    const script = source.replace('__TARE_RELEASES_BASE_URL__', () => base.replaceAll("'", "'\\''"))
    return c.text(script, 200, { 'Content-Type': 'text/x-shellscript; charset=UTF-8' })
  })
  app.get('/releases/:file', async (c) => {
    const file = c.req.param('file')
    if (!/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(file) || file === '.' || file === '..')
      fail('NOT_FOUND', 'This release file was not found.')
    let handle
    try { handle = await open(`releases/${file}`, 'r') }
    catch (error) {
      if ((error as NodeJS.ErrnoException).code === 'ENOENT') fail('NOT_FOUND', 'This release file was not found.')
      throw error
    }
    const info = await handle.stat()
    if (!info.isFile()) { await handle.close(); fail('NOT_FOUND', 'This release file was not found.') }
    const stream = Readable.toWeb(handle.createReadStream()) as ReadableStream<Uint8Array>
    const type = file.endsWith('.tar.gz') ? 'application/gzip'
      : file.endsWith('.sha256') || file === 'latest' ? 'text/plain; charset=UTF-8' : 'application/octet-stream'
    return c.newResponse(stream, 200, { 'Content-Type': type, 'Content-Length': String(info.size) })
  })
}
