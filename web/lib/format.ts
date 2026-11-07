// How Tare's figures are written, decided once.

const grouped = new Intl.NumberFormat('en-US')

/** Compute units and counts: grouped with commas, never abbreviated. */
export const units = (n: number) => grouped.format(Math.round(n))

/** A share as a whole percent. */
export const percent = (part: number, whole: number) => (whole > 0 ? `${Math.round((part / whole) * 100)}%` : '0%')

/** Binary size in bytes as [value, unit]: KB with one decimal, MB with two from 1 MB up. */
export const size = (bytes: number): [string, string] => (bytes >= 1024 * 1024 ? [(bytes / 1024 / 1024).toFixed(2), 'MB'] : [(bytes / 1024).toFixed(1), 'KB'])

/** Binary size as one string. */
export const kb = (bytes: number) => size(bytes).join(' ')

/** Lamports as SOL, up to four decimals. */
export const sol = (lamports: number) => {
  const v = lamports / 1_000_000_000
  return v.toFixed(v >= 100 ? 1 : v >= 1 ? 2 : 4)
}

/** An address or hash shortened around its middle. */
export const short = (s: string, head = 4, tail = 4) => (s.length <= head + tail + 1 ? s : `${s.slice(0, head)}…${s.slice(-tail)}`)

/** A UTC date as 2026-11-14. */
export const day = (iso: string | number | Date) => new Date(iso).toISOString().slice(0, 10)

/** A UTC date and time as 2026-11-14 09:30 UTC. */
export const stamp = (iso: string | number | Date) => `${new Date(iso).toISOString().slice(0, 16).replace('T', ' ')} UTC`

const BASE58 = /^[1-9A-HJ-NP-Za-km-z]{32,44}$/
/** Whether a string has the form of a Solana address. */
export const isAddress = (s: string) => BASE58.test(s)
