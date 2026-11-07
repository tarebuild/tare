// Lines of blocks for the cascade: a weighing's rows and a command's output.

import type { Colour } from '@/components/Block'
import type { Row } from './bench'
import { units } from './format'

/** One word of a line. `role` holds its block colour still; without it the block shuffles. */
export type Token = { t: string; ast?: boolean; role?: Colour; /** Leading indent, in characters. */ gap?: number }
export type Stanza = Token[][]

/** Parses `"TARE BUILD * SAME SOURCE"` into tokens; `*` is the asterisk. */
export function line(text: string, roles: Record<string, Colour> = {}): Token[] {
  return text.split(' ').map((t) => (t === '*' ? { t, ast: true } : { t, role: roles[t] }))
}

/** The product's own stanza: the four commands and what each leaves. */
export const COMMAND_STANZA: Stanza = [
  line('tare build * same source * same IDL'),
  line('tare weigh * gross, tare, net *'),
  line('tare prove * 3 corpora * every byte'),
  line('tare attest * on chain *'),
]

/** A weighing as stanzas: three instructions to a stanza, each a row of gross and net. */
export function weighingStanzas(rows: Row[], perStanza = 3): Stanza[] {
  const lines = rows.flatMap((ix) => (ix.gross === null || ix.net === null ? [] : [[
    { t: ix.name },
    { t: '*', ast: true },
    { t: 'gross' },
    { t: units(ix.gross) },
    { t: '*', ast: true },
    { t: 'net' },
    { t: units(ix.net), role: 'green' as Colour },
  ]]))
  const out: Stanza[] = []
  for (let i = 0; i < lines.length; i += perStanza) out.push(lines.slice(i, i + perStanza))
  return out
}

/** Repeats stanzas until there are `n`. */
export function fill(stanzas: Stanza[], n: number): Stanza[] {
  return Array.from({ length: n }, (_, i) => stanzas[i % stanzas.length]!)
}
