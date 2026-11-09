import type { RecordSummary } from '@tare/contract'
import Link from 'next/link'
import { day, percent, short, units } from '@/lib/format'
import { Chip } from './ui'
import styles from './Records.module.css'

export const corpusCount = (r: RecordSummary) => r.corpus.counts.tests + r.corpus.counts.generated + r.corpus.counts.mainnet

/**
 * Registry records, one line each: the program, its gross going to its net on one beam,
 * the corpus the proof was shown on, who attested it and when.
 */
export function Records({ items, showProgram = true }: { items: RecordSummary[]; showProgram?: boolean }) {
  return (
    <div className={styles.list} role="table" aria-label="Records">
      <div className={`${styles.row} ${styles.head}`} role="row">
        <span role="columnheader">{showProgram ? 'Program' : 'Record'}</span>
        <span role="columnheader" className={styles.r}>
          Gross
        </span>
        <span role="columnheader" className={styles.beamCol} />
        <span role="columnheader" className={styles.r}>
          Net
        </span>
        <span role="columnheader" className={styles.r}>
          Tare
        </span>
        <span role="columnheader" className={`${styles.r} ${styles.wide}`}>
          Corpus
        </span>
        <span role="columnheader" className={styles.wide}>
          Attester
        </span>
        <span role="columnheader" className={styles.wide}>
          Attested
        </span>
      </div>
      {items.map((r, i) => (
        <Link key={r.address} href={`/builds/${r.address}`} className={styles.row} role="row" style={{ '--row': i } as React.CSSProperties}>
          <span role="cell" className={styles.who}>
            <span className={styles.name}>
              {showProgram ? (r.program.name ?? (r.program.address ? short(r.program.address) : 'Unnamed program')) : short(r.address, 6, 6)}
            </span>
            <span className={styles.sub}>
              {showProgram && <span className="num">{short(r.address)}</span>}
              {r.source.commit && <span className="num">{r.source.commit.slice(0, 7)}</span>}
              {r.flags > 0 && (
                <Chip c="red">
                  {r.flags} {r.flags === 1 ? 'flag' : 'flags'}
                </Chip>
              )}
            </span>
          </span>
          <span role="cell" className={`num ${styles.r}`}>
            {units(r.gross)}
          </span>
          <span role="cell" className={styles.beamCol}>
            <span className={styles.beam} aria-hidden="true">
              <span style={{ width: `${r.gross > 0 ? (r.net / r.gross) * 100 : 0}%` }} />
            </span>
          </span>
          <span role="cell" className={`num ${styles.r} ${styles.net}`}>
            {units(r.net)}
          </span>
          <span role="cell" className={`num ${styles.r} ${styles.tare}`}>
            {percent(r.tare, r.gross)}
          </span>
          <span role="cell" className={`num ${styles.r} ${styles.wide}`}>
            {units(corpusCount(r))}
          </span>
          <span role="cell" className={`num ${styles.wide} ${styles.quiet}`}>
            {short(r.attester)}
          </span>
          <span role="cell" className={`num ${styles.wide} ${styles.quiet}`}>
            {day(r.createdAt)}
          </span>
        </Link>
      ))}
    </div>
  )
}
