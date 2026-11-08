import { percent, units } from '@/lib/format'
import styles from './Weighing.module.css'

export type WeighingRow = { name: string; gross: number | null; net: number | null }
export type WeighingTotals = { gross: number; net: number; inputs: number }

/**
 * The weighing: gross, tare and net for each instruction, and the same three as one bar.
 * The bar is the gross; its green part is what remains, its magenta part is what Tare took off.
 */
export function Weighing({ rows, totals, caption, limit }: { rows: WeighingRow[]; totals: WeighingTotals; caption: string; limit?: number }) {
  const shown = limit ? rows.slice(0, limit) : rows
  const max = Math.max(...rows.map((r) => r.gross ?? 0), 1)
  const { gross, net } = totals

  return (
    <div className={styles.scroll}>
      <table className={styles.table}>
        <caption className="sr-only">{caption}</caption>
        <thead>
          <tr>
            <th scope="col">
              Instruction<span className={styles.wideOnly}>, median units per call</span>
            </th>
            <th scope="col" className={styles.barCol}>
              <span className="sr-only">Share of gross that is tare and net</span>
            </th>
            <th scope="col" className={styles.r}>
              <span className="b" data-c="line">
                Gross
              </span>
            </th>
            <th scope="col" className={styles.r}>
              <span className="b" data-c="magenta">
                Tare
              </span>
            </th>
            <th scope="col" className={styles.r}>
              <span className="b" data-c="green">
                Net
              </span>
            </th>
          </tr>
        </thead>
        <tbody>
          {shown.map((r, i) =>
            r.gross === null || r.net === null ? (
              <tr key={r.name}>
                <th scope="row" className={styles.name}>
                  {r.name}
                </th>
                <td className={styles.barCol} />
                <td colSpan={3} className={`${styles.r} ${styles.unmeasured}`}>
                  No call in the corpus
                </td>
              </tr>
            ) : (
              <tr key={r.name} style={{ '--row': i } as React.CSSProperties}>
                <th scope="row" className={styles.name}>
                  {r.name}
                </th>
                <td className={styles.barCol}>
                  <Bar gross={r.gross} net={r.net} max={max} />
                </td>
                <td className={`num ${styles.r}`}>{units(r.gross)}</td>
                <td className={`num ${styles.r} ${styles.tare}`}>{units(r.gross - r.net)}</td>
                <td className={`num ${styles.r} ${styles.net}`}>{units(r.net)}</td>
              </tr>
            ),
          )}
        </tbody>
        <tfoot>
          <tr>
            <th scope="row" className={styles.name}>
              One call of each, over {units(totals.inputs)} {totals.inputs === 1 ? 'input' : 'inputs'}
            </th>
            <td className={styles.barCol}>
              <span className={styles.share}>
                <span className="num">{percent(gross - net, gross)}</span> of gross was tare
              </span>
            </td>
            <td className={`num ${styles.r}`}>{units(gross)}</td>
            <td className={`num ${styles.r} ${styles.tare}`}>{units(gross - net)}</td>
            <td className={`num ${styles.r} ${styles.net}`}>{units(net)}</td>
          </tr>
        </tfoot>
      </table>
    </div>
  )
}

export function Bar({ gross, net, max }: { gross: number; net: number; max: number }) {
  const width = (gross / max) * 100
  const kept = gross > 0 ? (net / gross) * 100 : 0
  return (
    <span className={styles.bar} style={{ width: `${Math.max(width, 1.5)}%` }} aria-hidden="true">
      <span className={styles.barNet} style={{ width: `${kept}%` }} />
    </span>
  )
}

/** The place a weighing lands while its numbers are still on the scale. */
export function WeighingPending({ rows = 5, note }: { rows?: number; note: string }) {
  return (
    <div className={styles.pending}>
      <div className={styles.pendingRows} aria-hidden="true">
        {Array.from({ length: rows }, (_, i) => (
          <span key={i} className="skel" style={{ width: `${88 - i * 13}%`, height: 14, animationDelay: `${i * 120}ms` }} />
        ))}
      </div>
      <p className={styles.pendingNote}>
        <span className="b" data-c="yellow">
          On the scale
        </span>
        <span className="t-small">{note}</span>
      </p>
    </div>
  )
}
