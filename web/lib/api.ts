// The backend's API, read through its contract. Server components call these; a failed read is null,
// and each page says in its own words what could not be read.

import { ApiClientError, createClient } from '@tare/contract'

export const API_URL = process.env.API_URL ?? process.env.NEXT_PUBLIC_API_URL ?? 'http://127.0.0.1:7810'

export const api = createClient(API_URL, {
  fetch: (input, init) => fetch(input, { ...init, cache: 'no-store' }),
})

export { ApiClientError }

/** Runs a read and returns null when the API cannot be reached or refuses. */
export async function read<T>(call: () => Promise<T>): Promise<T | null> {
  try {
    return await call()
  } catch {
    return null
  }
}

export const registryTotals = () => read(() => api.registryTotals())

/** A dev-only fixture scenario passed through from `?fixture=`; ignored by the real API. */
export const scenario = (value: string | string[] | undefined) => (typeof value === 'string' && value ? { fixture: value } : undefined)
