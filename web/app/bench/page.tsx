import type { Metadata } from 'next'
import { Scale } from '@/components/Scale'
import { Chip, Facts, Page, PageHead, Section, Stat, Stats, State } from '@/components/ui'
import { Weighing } from '@/components/Weighing'
import { bench, measured, rows, weighed, type MeasuredProgram } from '@/lib/bench'
import { day, kb, percent, short, sol, units } from '@/lib/format'
import styles from './page.module.css'

export const metadata: Metadata = {
  title: 'The benchmark',
  description: 'Tare run on open-source Anchor programs nobody wrote for it: gross, tare and net by instruction, their own tests and the proof corpus.',
}

const KIND = { anchor_test: 'Anchor test program', anchor_example: 'Anchor example', production: 'Production program' } as const

/** Version lines in order, each with its measured programs, heaviest first. */
function byVersion() {
  const groups = new Map<string, MeasuredProgram[]>()
  for (const p of measured) groups.set(p.anchorVersion, [...(groups.get(p.anchorVersion) ?? []), p])
  return [...groups.entries()]
    .sort(([a], [b]) => a.localeCompare(b, undefined, { numeric: true }))
    .map(([version, programs]) => ({ version, programs: programs.sort((a, b) => b.weighing.totals.gross - a.weighing.totals.gross) }))
}

function Program({ p }: { p: MeasuredProgram }) {
  const w = p.weighing
  const t = w.totals
  const corpus = p.proof.corpus.counts
  const smaller = w.builds.tare.sizeBytes < w.builds.stock.sizeBytes
  return (
    <details className={styles.program} id={p.id}>
      <summary className={styles.row}>
        <span className={styles.name}>{p.name}</span>
        <span className={styles.kind}>{KIND[p.kind]}</span>
        <span className={`num ${styles.count}`}>{units(weighed(p))}</span>
        <span className={`num ${styles.fig}`}>{units(t.gross)}</span>
        <span className={`num ${styles.fig} ${styles.tare}`}>{percent(t.tare, t.gross)}</span>
        <span className={`num ${styles.fig} ${styles.net}`}>{units(t.net)}</span>
        <span className={styles.bar} aria-hidden="true">
          <span style={{ width: `${t.gross > 0 ? (t.net / t.gross) * 100 : 0}%` }} />
        </span>
        <span className={styles.caret} aria-hidden="true" />
      </summary>
      <div className={styles.open}>
        <Weighing rows={rows(w)} totals={t} caption={`${p.name}: gross, tare and net compute units by instruction`} />
        <Facts
          rows={[
            {
              label: 'Source',
              value: p.source.repository ? (
                <a className="link" href={p.source.repository} target="_blank" rel="noreferrer">
                  {p.source.repository.replace(/^https?:\/\/(www\.)?/, '').replace(/\.git$/, '')}
                </a>
              ) : (
                'Not stated'
              ),
            },
            { label: 'Commit', value: <span className="num">{p.source.commit ? p.source.commit.slice(0, 12) : 'Not stated'}</span> },
            ...(smaller
              ? [
                  {
                    label: 'Binary size',
                    value: (
                      <span className="num">
                        {kb(w.builds.stock.sizeBytes)} to <b className={styles.net}>{kb(w.builds.tare.sizeBytes)}</b>
                      </span>
                    ),
                  },
                  {
                    label: 'Rent locked',
                    value: (
                      <span className="num">
                        {sol(w.builds.stock.rentLamports)} to <b className={styles.net}>{sol(w.builds.tare.rentLamports)}</b> SOL
                      </span>
                    ),
                  },
                ]
              : []),
            ...(p.tests.ran > 0
              ? [
                  {
                    label: 'Its own tests',
                    value: (
                      <span className="num">
                        {units(p.tests.passed)} of {units(p.tests.ran)} pass on the Tare build
                      </span>
                    ),
                  },
                ]
              : []),
            {
              label: 'Proof',
              value: (
                <span className={styles.proof}>
                  {p.proof.status === 'matched' ? <Chip c="green">Matched</Chip> : <Chip c="red">Diverged</Chip>}
                  <span className="num">
                    {[
                      corpus.tests > 0 && `${units(corpus.tests)} own tests`,
                      corpus.generated > 0 && `${units(corpus.generated)} generated`,
                      corpus.mainnet > 0 && `${units(corpus.mainnet)} mainnet`,
                    ]
                      .filter(Boolean)
                      .join(', ')}
                  </span>
                </span>
              ),
            },
            { label: 'Corpus hash', value: <span className="num">{short(p.proof.corpus.hash, 10, 10)}</span> },
            ...(p.record
              ? [
                  {
                    label: 'Record',
                    value: p.recordUrl ? (
                      <a href={p.recordUrl} className="link num">
                        {short(p.record, 6, 6)}
                      </a>
                    ) : (
                      <span className="num">{short(p.record, 6, 6)}</span>
                    ),
                  },
                ]
              : []),
          ]}
        />
      </div>
    </details>
  )
}

export default function Bench() {
  const t = bench.totals
  const versions = byVersion()
  return (
    <Page>
      <PageHead
        title={[{ t: 'the benchmark', c: 'cyan' }]}
        lead="Tare run on open-source Anchor programs nobody wrote for it. Each is fetched at a pinned commit, built unchanged by both pipelines, weighed and proven."
      >
        {bench.generatedAt && <span className="t-small">Run {day(bench.generatedAt)}</span>}
        {bench.toolchain && <Chip>tare {bench.toolchain}</Chip>}
      </PageHead>

      {t && t.measured > 0 && (
        <Scale
          gross={t.weights.gross}
          net={t.weights.net}
          notes={['every measured instruction, stock', "the framework’s own weight", 'the same instructions, built with Tare']}
        />
      )}

      {t && t.measured > 0 ? (
        <Stats>
          <Stat value={units(t.measured)} label={t.measured === 1 ? 'Program built unchanged' : 'Programs built unchanged'} />
          <Stat value={units(t.instructions)} label="Instructions weighed" />
          {t.binarySize.tare > 0 && <Stat value={percent(t.binarySize.tare, t.binarySize.gross)} label="Off the binaries" c="magenta" />}
          {t.rentLamports.tare > 0 && <Stat value={sol(t.rentLamports.tare)} unit="SOL" label="Rent no longer locked" />}
          <Stat value={units(t.corpus.tests + t.corpus.generated + t.corpus.mainnet)} label="Transactions compared" />
        </Stats>
      ) : (
        <State c="yellow" word="On the scale" title="The corpus is being built and weighed">
          The first programs are going through both pipelines now. Each one lands here with its weighing and its proof corpus as soon as
          its run completes.
        </State>
      )}

      {versions.length > 1 && (
        <nav aria-label="Anchor versions" className={styles.index}>
          {versions.map((v) => (
            <a key={v.version} href={`#anchor-${v.version}`} className="btn" data-kind="secondary" data-size="sm">
              Anchor {v.version}
              <span className="num">{v.programs.length}</span>
            </a>
          ))}
        </nav>
      )}

      {versions.map((v) => (
        <Section
          key={v.version}
          id={`anchor-${v.version}`}
          title={`Anchor ${v.version}`}
          side={`${units(v.programs.length)} programs, heaviest first. Open one for its instructions.`}
        >
          <div className={styles.list}>
            <div className={`${styles.row} ${styles.head}`} aria-hidden="true">
              <span>Program</span>
              <span className={styles.kind}>Kind</span>
              <span className={styles.count}>Instructions</span>
              <span className={styles.fig}>Gross</span>
              <span className={styles.fig}>Tare</span>
              <span className={styles.fig}>Net</span>
              <span className={styles.barHead} />
              <span />
            </div>
            {v.programs.map((p) => (
              <Program key={p.id} p={p} />
            ))}
          </div>
        </Section>
      ))}
    </Page>
  )
}
