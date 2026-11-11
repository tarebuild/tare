import { createHash, randomUUID } from 'node:crypto'
import { mkdir, readFile, rename, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { z } from 'zod'
import { errors, UploadFlagInput, UploadProofReport, type FlagInput, type ProofReport, type ErrorCode, type UploadResult } from '@tare/contract'
import { env } from './config.js'
import type { Database } from './db.js'
import { failer, type Fail } from './engines/server/errors.js'
import { readDocument } from './content-response.js'

type Kind = 'report' | 'flag_input'
type Document = UploadProofReport | UploadFlagInput
const fail: Fail<ErrorCode> = failer(errors.status)
const cid = z.string().regex(/^(?:Qm[1-9A-HJ-NP-Za-km-z]{44}|b[a-z2-7]{20,120})$/)
const pinResponse = z.object({ data: z.object({ cid }) })
const maxBytes = 1024 * 1024

// Object keys are sorted recursively; array order remains part of the document.
function canonical(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`
  if (value !== null && typeof value === 'object') {
    return `{${Object.entries(value).sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0).map(([key, item]) => `${JSON.stringify(key)}:${canonical(item)}`).join(',')}}`
  }
  return JSON.stringify(value)!
}

const digest = (bytes: string | Uint8Array) => createHash('sha256').update(bytes).digest('hex')
const folder = (kind: Kind) => kind === 'report' ? 'reports' : 'flag-inputs'

async function readOptional(file: string) {
  try {
    return await readFile(file)
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') return null
    throw error
  }
}

function unavailable(kind: Kind): never {
  return fail(kind === 'report' ? 'REPORT_UNAVAILABLE' : 'FLAG_INPUT_UNAVAILABLE',
    kind === 'report' ? 'The proof report is unavailable.' : 'The flag input is unavailable.',
    { retryable: true, stage: 'read' })
}

function documentFrom(bytes: Uint8Array, kind: Kind, hash: string): Document {
  if (bytes.byteLength > maxBytes || digest(bytes) !== hash) return unavailable(kind)
  let value: unknown
  try {
    value = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes))
  } catch {
    return unavailable(kind)
  }
  const parsed = (kind === 'report' ? UploadProofReport : UploadFlagInput).safeParse(value)
  if (!parsed.success) return unavailable(kind)
  return parsed.data
}

async function readBounded(response: Response): Promise<Uint8Array> {
  if (!response.ok || Number(response.headers.get('content-length') ?? 0) > maxBytes || !response.body) {
    throw new Error('Pinned content could not be read.')
  }
  const reader = response.body.getReader()
  const chunks: Uint8Array[] = []
  let bytes = 0
  try {
    while (true) {
      const part = await reader.read()
      if (part.done) break
      bytes += part.value.byteLength
      if (bytes > maxBytes) {
        await reader.cancel()
        throw new Error('Pinned content is too large.')
      }
      chunks.push(part.value)
    }
  } finally {
    reader.releaseLock()
  }
  return Buffer.concat(chunks, bytes)
}

// A record may carry an IPFS URI or an earlier gateway URL. Only its CID is used;
// requests always go to the configured gateway, never to a caller-supplied host.
function contentCid(location: string) {
  let value = location
  if (location.startsWith('ipfs://')) value = location.slice('ipfs://'.length)
  else if (location.startsWith('https://') || location.startsWith('http://')) {
    if (!URL.canParse(location)) return null
    value = new URL(location).pathname.match(/^\/ipfs\/([^/]+)\/?$/)?.[1] ?? ''
  }
  const parsed = cid.safeParse(value)
  return parsed.success ? parsed.data : null
}

export function createContentStore(db: Database['db']) {
  const directory = (kind: Kind) => path.resolve('.data', folder(kind))
  const writeLocal = async (kind: Kind, hash: string, bytes: Uint8Array) => {
    await mkdir(directory(kind), { recursive: true })
    const target = path.join(directory(kind), `${hash}.json`)
    const temporary = `${target}.${randomUUID()}.tmp`
    await writeFile(temporary, bytes)
    await rename(temporary, target)
  }

  async function readContent(kind: Kind, hash: string, pinnedLocation?: string) {
    const stored = await db.pinnedReport.findUnique({ where: { hash } })
    if (stored && stored.kind !== kind) return unavailable(kind)
    const location = stored?.location ?? pinnedLocation
    let bytes: Uint8Array
    if (!location || location.startsWith('/')) {
      const local = await readOptional(path.join(directory(kind), `${hash}.json`))
      if (local === null) return unavailable(kind)
      bytes = local
    } else {
      const content = contentCid(location)
      if (!content || !env.PINATA_GATEWAY) return unavailable(kind)
      try {
        const gateway = env.PINATA_GATEWAY.replace(/\/+$/, '')
        const response = await fetch(`${gateway}/ipfs/${content}`, { signal: AbortSignal.timeout(15_000), redirect: 'error' })
        bytes = await readBounded(response)
      } catch {
        return unavailable(kind)
      }
    }
    return { bytes, value: documentFrom(bytes, kind, hash) }
  }

  return {
    /** rawBytes must be the request body already parsed and validated as value. */
    async put(kind: Kind, value: Document, rawBytes?: Uint8Array): Promise<UploadResult> {
      const bytes = rawBytes ?? Buffer.from(canonical(value), 'utf8')
      if (bytes.byteLength > maxBytes) fail('PAYLOAD_TOO_LARGE', 'The request body is too large.', { stage: 'validation' })
      const hash = digest(bytes)
      const existing = await db.pinnedReport.findUnique({ where: { hash } })
      if (existing) {
        if (existing.location.startsWith('/') && await readOptional(path.join(directory(kind), `${hash}.json`)) === null) {
          await writeLocal(kind, hash, bytes)
        }
        return { hash, location: existing.location }
      }
      let location = `/v1/${folder(kind)}/${hash}`

      if (env.PINATA_JWT) {
        const data = new FormData()
        data.set('file', new File([new Uint8Array(bytes)], `${hash}.json`, { type: 'application/json' }))
        data.set('network', 'public')
        try {
          const response = await fetch('https://uploads.pinata.cloud/v3/files', {
            method: 'POST',
            headers: { Authorization: `Bearer ${env.PINATA_JWT}` },
            body: data,
            signal: AbortSignal.timeout(15_000),
          })
          const bytes = await readBounded(response)
          const pinned = pinResponse.parse(JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes)))
          location = pinned.data.cid
        } catch {
          fail('PINNING_UNAVAILABLE', 'The report pinning service is unavailable.', { retryable: true, stage: 'upload' })
        }
      } else {
        await writeLocal(kind, hash, bytes)
      }

      const stored = await db.pinnedReport.upsert({
        where: { hash }, update: {},
        create: { hash, kind, location, bytes: bytes.byteLength },
      })
      return { hash, location: stored.location }
    },

    async get(kind: Kind, hash: string, pinnedLocation?: string): Promise<ProofReport | FlagInput> {
      return readDocument(kind, (await readContent(kind, hash, pinnedLocation)).value)
    },

    async getBytes(kind: Kind, hash: string, pinnedLocation?: string): Promise<Uint8Array> {
      return (await readContent(kind, hash, pinnedLocation)).bytes
    },
  }
}
