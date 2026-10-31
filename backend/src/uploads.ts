import type { Context } from 'hono'
import type { z } from 'zod'
import { parse } from './engines/server/errors.js'
import { fail } from './errors.js'

// Preserve the CLI's serialized bytes: its hash is over the exact uploaded body.
export async function readUpload<S extends z.ZodType>(c: Context, schema: S) {
  if (c.req.header('content-type')?.split(';', 1)[0]?.trim().toLowerCase() !== 'application/json')
    fail('UNSUPPORTED_MEDIA_TYPE', 'Send an application/json body.', { stage: 'validation' })
  const bytes = Buffer.from(await c.req.arrayBuffer())
  let value: unknown
  try { value = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes)) }
  catch { fail('VALIDATION_ERROR', 'The JSON body is invalid.', { stage: 'validation' }) }
  return { bytes, value: parse(schema, value) }
}
