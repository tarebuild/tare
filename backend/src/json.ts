import type { Context } from 'hono'
import type { ContentfulStatusCode } from 'hono/utils/http-status'

// The contract owns response types; avoid recursively expanding arbitrary chain error JSON in Hono.
export function json(c: Context, value: unknown, status: ContentfulStatusCode = 200) {
  return c.newResponse(JSON.stringify(value), status, { 'Content-Type': 'application/json; charset=UTF-8' })
}
