// One RPC endpoint for a service: a web3.js Connection and raw JSON-RPC calls over one transport,
// so both are paced by the same budget, time out the same way and log one JSON line per call with
// the method name only. The endpoint URL carries the provider key; it never reaches a log or an
// error message.

import { Connection, type Commitment } from '@solana/web3.js'
import { parseExact } from './json.js'
import { pacer, type RpcCall } from './pace.js'

export class RpcError extends Error {
  constructor(
    readonly method: string,
    /** HTTP status, JSON-RPC error code, or -1 when no answer arrived */
    readonly code: number,
    message: string,
  ) {
    super(`RPC ${method}: ${message}`)
    this.name = 'RpcError'
  }
}

export type RpcLog = { event: 'rpc'; service: string; method: string; status: number | 'failed'; ms: number }

export type RpcOptions = {
  url: string
  /** the budget's name: clients with the same name in one process share one pace */
  service: string
  /** required for a remote endpoint; localnet defaults to 50 */
  rps?: number
  /** required for a remote endpoint; localnet defaults to 0 */
  scanIntervalMs?: number
  timeoutMs?: number
  commitment?: Commitment
  log?: (entry: RpcLog) => void
}

export type Rpc = ReturnType<typeof createRpc>

const LOCAL = new Set(['localhost', '127.0.0.1', '[::1]'])

function callsIn(body: unknown): RpcCall[] {
  try {
    const parsed = JSON.parse(String(body))
    return (Array.isArray(parsed) ? parsed : [parsed]).map((r) => ({
      method: typeof r?.method === 'string' ? r.method : 'unknown',
      params: Array.isArray(r?.params) ? r.params : undefined,
    }))
  } catch {
    return [{ method: 'unknown' }]
  }
}

export function createRpc(options: RpcOptions) {
  const { url, service, timeoutMs = 15_000, commitment = 'confirmed', log = (entry: RpcLog) => console.log(JSON.stringify(entry)) } = options
  const local = LOCAL.has(new URL(url).hostname)
  const rps = options.rps ?? (local ? 50 : undefined)
  const scanIntervalMs = options.scanIntervalMs ?? (local ? 0 : undefined)
  if (!rps || scanIntervalMs === undefined) {
    throw new Error(`${service}: set rps and scanIntervalMs. A remote endpoint is a budget shared with every service on its key.`)
  }
  const pace = pacer(service, { rps, scanIntervalMs })

  async function timed(calls: RpcCall[], send: () => Promise<Response>) {
    const started = performance.now()
    let status: RpcLog['status'] = 'failed'
    try {
      const response = await send()
      status = response.status
      return response
    } finally {
      const method = calls.length === 1 ? calls[0]!.method : `batch:${calls.length}`
      log({ event: 'rpc', service, method, status, ms: Math.round(performance.now() - started) })
    }
  }

  const deadline = (signal?: AbortSignal | null) => (signal ? AbortSignal.any([signal, AbortSignal.timeout(timeoutMs)]) : AbortSignal.timeout(timeoutMs))

  const transport: typeof fetch = async (input, init) => {
    const calls = callsIn(init?.body)
    await pace.wait(calls)
    return timed(calls, () => fetch(input, { ...init, signal: deadline(init?.signal) }))
  }

  // web3.js otherwise retries a 429 on its own with backoff, multiplying calls on a spent budget.
  const connection = new Connection(url, { commitment, disableRetryOnRateLimit: true, fetch: transport })

  let id = 0
  async function call<T>(method: string, params: readonly unknown[] = []): Promise<T> {
    let response: Response
    try {
      response = await transport(url, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ jsonrpc: '2.0', id: ++id, method, params }) })
    } catch (error) {
      throw new RpcError(method, -1, error instanceof Error ? error.name : 'transport failure')
    }
    let body: unknown
    try {
      body = parseExact(await response.text())
    } catch {
      throw new RpcError(method, response.ok ? -1 : response.status, response.ok ? 'malformed response' : `HTTP ${response.status}`)
    }
    if (body && typeof body === 'object' && !Array.isArray(body) && 'error' in body) {
      const error = body.error
      if (error && typeof error === 'object' && 'code' in error && 'message' in error &&
        typeof error.code === 'number' && Number.isInteger(error.code) && typeof error.message === 'string') {
        throw new RpcError(method, error.code, error.message)
      }
    }
    if (!response.ok) throw new RpcError(method, response.status, `HTTP ${response.status}`)
    if (!body || typeof body !== 'object' || Array.isArray(body) || !Object.hasOwn(body, 'result')) throw new RpcError(method, -1, 'response without a result')
    return (body as { result: T }).result
  }


  /** A bounded JSON-RPC batch; each method reserves its own budget turn. */
  async function batch<T>(requests: readonly { method: string; params: readonly unknown[] }[]): Promise<({ result: T } | { error: { code: number; message: string } })[]> {
    if (!requests.length) return []
    const payload = requests.map((request) => ({ jsonrpc: '2.0', id: ++id, ...request }))
    let response: Response
    try {
      response = await transport(url, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(payload) })
    } catch (error) {
      throw new RpcError('batch', -1, error instanceof Error ? error.name : 'transport failure')
    }
    let value: unknown
    try {
      value = parseExact(await response.text())
    } catch {
      throw new RpcError('batch', response.ok ? -1 : response.status, response.ok ? 'malformed batch response' : `HTTP ${response.status}`)
    }
    // Providers that refuse batching answer one JSON-RPC error, often with id:null
    // and HTTP 400. Preserve its code so the caller can choose a bounded fallback.
    if (value && typeof value === 'object' && !Array.isArray(value) && 'error' in value) {
      const error = value.error
      if (error && typeof error === 'object' && 'code' in error && 'message' in error &&
        typeof error.code === 'number' && Number.isInteger(error.code) && typeof error.message === 'string') {
        throw new RpcError('batch', error.code, error.message)
      }
    }
    if (!response.ok) throw new RpcError('batch', response.status, `HTTP ${response.status}`)
    if (!Array.isArray(value) || value.length !== payload.length) throw new RpcError('batch', -1, 'malformed batch response')
    const rows = new Map(value.map((row) => [row?.id, row]))
    return payload.map((request) => {
      const row = rows.get(request.id)
      if (row?.error && typeof row.error.code === 'number' && typeof row.error.message === 'string') return { error: row.error }
      if (row && Object.hasOwn(row, 'result')) return { result: row.result as T }
      throw new RpcError(request.method, -1, 'response without a result')
    })
  }

  /**
   * One request passed through as text (the relay), so the provider's numbers reach the browser
   * byte for byte. Refused with code 429 when the budget's queue is longer than maxWaitMs.
   */
  async function forward(request: { id: string | number; method: string; params: readonly unknown[] }, maxWaitMs: number): Promise<string> {
    const calls = [{ method: request.method, params: request.params }]
    const delay = pace.reserve(calls, maxWaitMs)
    if (delay === null) throw new RpcError(request.method, 429, 'budget exhausted')
    if (delay > 0) await new Promise((resolve) => setTimeout(resolve, delay))
    const response = await timed(calls, () =>
      fetch(url, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ jsonrpc: '2.0', ...request }), signal: deadline() }),
    ).catch((error) => {
      throw new RpcError(request.method, -1, error instanceof Error ? error.name : 'transport failure')
    })
    if (!response.ok) throw new RpcError(request.method, response.status, `HTTP ${response.status}`)
    return response.text()
  }

  // The genesis hash identifies the cluster and never changes: read once per process, failures not kept.
  let genesis: Promise<string> | undefined
  const genesisHash = () =>
    (genesis ??= call<string>('getGenesisHash').catch((error) => {
      genesis = undefined
      throw error
    }))

  return { service, connection, call, batch, forward, genesisHash }
}
