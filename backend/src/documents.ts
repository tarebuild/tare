import { readFile, stat } from 'node:fs/promises'
import type { z } from 'zod'
import { fail } from './errors.js'

// A request shares any in-flight read. Publish only a stable, validated revision;
// a producer replacing the file never displaces the last good document halfway.
export function fileDocument<T>(path: string, schema: z.ZodType<T>, empty: T) {
  let cached: { revision: string; value: T } | undefined
  let rejectedRevision: string | undefined
  let inFlight: Promise<T> | undefined

  async function revision() {
    try {
      const info = await stat(path, { bigint: true })
      return `${info.dev}:${info.ino}:${info.mtimeNs}:${info.ctimeNs}:${info.size}`
    }
    catch (error) {
      if ((error as NodeJS.ErrnoException).code === 'ENOENT') return null
      throw error
    }
  }

  async function load(): Promise<T> {
    const before = await revision()
    if (before === null) return cached?.value ?? empty
    if (cached && (cached.revision === before || rejectedRevision === before)) return cached.value
    let text: string
    try { text = await readFile(path, 'utf8') }
    catch (error) {
      if ((error as NodeJS.ErrnoException).code === 'ENOENT') return cached?.value ?? empty
      throw error
    }
    const after = await revision()
    if (after === null) return cached?.value ?? empty
    if (before !== after) {
      if (cached) return cached.value
      fail('INVALID_REPORT', 'The generated data file is being updated. Try again.', { stage: 'read', retryable: true })
    }
    let raw: unknown
    try { raw = JSON.parse(text) }
    catch {
      console.error(JSON.stringify({ level: 'error', event: 'data_validation_failed', file: path, path: '$', reason: 'invalid_json' }))
      if (cached) { rejectedRevision = before; return cached.value }
      fail('INVALID_REPORT', 'The generated data file is not valid JSON.')
    }
    const parsed = schema.safeParse(raw)
    if (!parsed.success) {
      const issue = parsed.error.issues[0]!
      const parts = [...issue.path]
      if (issue.code === 'unrecognized_keys' && issue.keys[0]) parts.push(issue.keys[0])
      const firstPath = parts.join('.') || '$'
      console.error(JSON.stringify({ level: 'error', event: 'data_validation_failed', file: path, path: firstPath, reason: issue.code }))
      if (cached) { rejectedRevision = before; return cached.value }
      fail('INVALID_REPORT', `The generated data file does not match its contract at ${firstPath}.`, {
        stage: 'read', fieldErrors: { [firstPath]: [issue.message] },
      })
    }
    const value = parsed.data
    cached = { revision: before, value }
    rejectedRevision = undefined
    return value
  }

  return (): Promise<T> => {
    inFlight ??= load().finally(() => { inFlight = undefined })
    return inFlight
  }
}
