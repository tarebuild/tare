import type { Context } from 'hono'
import { FlagInput, ProofReport } from '@tare/contract'
import { fail } from './errors.js'
import { json } from './json.js'

export type ContentKind = 'report' | 'flag_input'

/** A typed view may normalize values; the content-addressed bytes never change. */
export function readDocument(kind: ContentKind, value: unknown) {
  const result = (kind === 'report' ? ProofReport : FlagInput).safeParse(value)
  if (!result.success) {
    const issue = result.error.issues[0]!
    const field = issue.path.join('.') || '$'
    fail(kind === 'report' ? 'REPORT_UNAVAILABLE' : 'FLAG_INPUT_UNAVAILABLE',
      'This document cannot be represented as a numeric JSON view.',
      { stage: 'read', retryable: false, fieldErrors: { [field]: [issue.message] } })
  }
  return result.data
}

export function contentResponse(c: Context, kind: ContentKind, bytes: Uint8Array) {
  c.header('Vary', 'Accept')
  const wantsJson = c.req.header('Accept')?.split(',').some((entry) => {
    const [type, ...parameters] = entry.trim().toLowerCase().split(';')
    return type === 'application/json' && !parameters.some((value) => /^q=0(?:\.0*)?$/.test(value.trim()))
  })
  if (wantsJson) return json(c, readDocument(kind, JSON.parse(new TextDecoder().decode(bytes))))
  return c.newResponse(new Uint8Array(bytes), 200, { 'Content-Type': 'application/octet-stream' })
}
