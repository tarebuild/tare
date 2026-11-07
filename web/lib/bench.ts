// The benchmark, as the site reads it. lib/bench.json is a copy of backend/bench/results.json
// (schema: BenchResults in @tare/contract) written by scripts/sync.mjs.

import { BenchResults, open, type Weighing } from '@tare/contract'
import raw from './bench.json'

const parsed = open(BenchResults).safeParse(raw)
// A fixture document is never shown as benchmark evidence.
const doc = parsed.success && !parsed.data.fixture ? parsed.data : null

export type Bench = BenchResults
export type BenchProgram = Bench['programs'][number]
export type MeasuredProgram = Extract<BenchProgram, { status: 'measured' }>
export type NotBuiltProgram = Extract<BenchProgram, { status: 'not_built' }>

export const bench = {
  generatedAt: doc?.generatedAt ?? null,
  toolchain: doc?.toolchainVersion ?? null,
  programs: doc?.programs ?? [],
  totals: doc?.totals ?? null,
}

export const measured: MeasuredProgram[] = bench.programs.filter((p): p is MeasuredProgram => p.status === 'measured')
export const notBuilt: NotBuiltProgram[] = bench.programs.filter((p): p is NotBuiltProgram => p.status === 'not_built')

const newest = (a: MeasuredProgram, b: MeasuredProgram) => b.anchorVersion.localeCompare(a.anchorVersion, undefined, { numeric: true })

const most = (a: MeasuredProgram, b: MeasuredProgram) => weighed(b) - weighed(a)

/**
 * The one program every surface features (the landing, the films, the posts), in this order:
 * a production program, the one with the most instructions weighed; then Anchor's own benchmark
 * program `bench` on the newest version line, the program Anchor itself tracks compute with;
 * then an Anchor example; then any measured program.
 */
export const featured: MeasuredProgram | null =
  measured.filter((p) => p.kind === 'production').sort(most)[0] ??
  measured.filter((p) => p.name === 'bench').sort(newest)[0] ??
  measured.filter((p) => p.kind === 'anchor_example').sort(most)[0] ??
  [...measured].sort(most)[0] ??
  null

/** How many of a program's instructions have a measured weight. */
export function weighed(p: MeasuredProgram): number {
  return p.weighing.instructions.filter((ix) => ix.gross !== null).length
}

export type Row = { name: string; gross: number | null; net: number | null; calls: number }

/** A weighing's measured instructions as rows of median compute units per call, heaviest first. */
export function rows(weighing: Pick<Weighing, 'instructions'>): Row[] {
  return weighing.instructions
    .filter((ix) => ix.gross !== null)
    .map((ix) => ({ name: instructionName(ix), gross: ix.gross, net: ix.net, calls: ix.inputs }))
    .sort((a, b) => (b.gross ?? 0) - (a.gross ?? 0))
}

/** An instruction's name, or its discriminator when no IDL names it. */
export const instructionName = (ix: { name: string | null; discriminator: string }) => ix.name ?? `0x${ix.discriminator}`
