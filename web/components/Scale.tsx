import { B } from './Block'
import { percent, units } from '@/lib/format'
import styles from './Scale.module.css'

/**
 * The scale: gross, tare and net as three figures, and the beam they share. The beam is the gross;
 * its green part is what remains, its magenta part is what was taken off.
 */
export function Scale({
  gross,
  net,
  notes = ['built with stock Anchor', "the framework’s own weight", 'built with Tare'],
  align = 'left',
}: {
  gross: number
  net: number
  notes?: [string, string, string]
  align?: 'left' | 'centre'
}) {
  const tare = gross - net
  return (
    <div className={styles.scale} data-align={align} style={{ '--chars': Math.max(units(gross).length, units(tare).length, units(net).length) } as React.CSSProperties}>
      <dl className={styles.figures}>
        <div>
          <dt>
            <B c="line">Gross</B>
          </dt>
          <dd className="num">{units(gross)}</dd>
          <dd className={styles.note}>{notes[0]}</dd>
        </div>
        <div data-c="tare">
          <dt>
            <B c="magenta">Tare</B>
          </dt>
          <dd className="num">{units(tare)}</dd>
          <dd className={styles.note}>
            {percent(tare, gross)} of gross, {notes[1]}
          </dd>
        </div>
        <div data-c="net">
          <dt>
            <B c="green">Net</B>
          </dt>
          <dd className="num" data-count data-from={gross} data-to={net}>
            {units(net)}
          </dd>
          <dd className={styles.note}>{notes[2]}</dd>
        </div>
      </dl>
      <Beam parts={[{ share: gross > 0 ? net / gross : 0, c: 'net' }, { share: gross > 0 ? tare / gross : 0, c: 'tare' }]} />
    </div>
  )
}

export type BeamPart = { share: number; c: 'net' | 'tare' | 'ink'; label?: string }

/** One horizontal beam split into parts by share. */
export function Beam({ parts, tall }: { parts: BeamPart[]; tall?: boolean }) {
  return (
    <div className={styles.beam} data-tall={tall || undefined} aria-hidden="true">
      {parts.map((p, i) => (
        <span key={i} data-c={p.c} style={{ '--part': i, flexGrow: Math.max(p.share, 0.004), opacity: p.c === 'ink' ? Math.max(1 - i * 0.13, 0.22) : undefined } as React.CSSProperties} title={p.label} />
      ))}
    </div>
  )
}
