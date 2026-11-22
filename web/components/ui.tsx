import type { ReactNode } from 'react'
import { B, type Colour } from './Block'
import styles from './ui.module.css'

export type TitleBlock = { t: string; c: Colour }

/**
 * The title row of a working page. The title is a line in the hero's texture: mono words on blocks,
 * the kind of thing first and then its name.
 */
export function PageHead({ title, lead, children }: { title: TitleBlock[]; lead?: ReactNode; children?: ReactNode }) {
  return (
    <header className={styles.head}>
      <h1 className={styles.title} data-land>
        {title.map((b, i) => (
          <B key={i} c={b.c}>
            {b.t}
          </B>
        ))}
      </h1>
      {(lead || children) && (
        <div className={styles.headRow}>
          {lead && <p className={styles.lead}>{lead}</p>}
          {children && <div className={styles.headSide}>{children}</div>}
        </div>
      )}
    </header>
  )
}

/** A working page. `wide` is for a view in regions, which uses more of a wide frame than reading pages do. */
export function Page({ children, narrow, wide }: { children: ReactNode; narrow?: boolean; wide?: boolean }) {
  return (
    <div className={`wrap ${styles.page}`} data-narrow={narrow || undefined} data-wide={wide || undefined}>
      {children}
    </div>
  )
}

/**
 * A working view's two regions on a wide screen: the work, and what belongs to the thing in hand beside it,
 * which stays in view with its own scroll. On a phone both open into the page's one column, each `Slot` at its
 * place in the task's order.
 */
export function Regions({ work, aside }: { work: ReactNode; aside: ReactNode }) {
  return (
    <div className={styles.regions}>
      <div className={styles.work}>{work}</div>
      <aside className={styles.aside}>{aside}</aside>
    </div>
  )
}

/** One part of a region, with its place in the phone's single column. */
export function Slot({ order, children }: { order: number; children: ReactNode }) {
  return (
    <div className={styles.slot} style={{ '--order': order } as React.CSSProperties}>
      {children}
    </div>
  )
}

export function Section({ title, side, children, id }: { title: ReactNode; side?: ReactNode; children: ReactNode; id?: string }) {
  return (
    <section className={styles.section} aria-labelledby={id}>
      <div className={styles.sectionHead}>
        <h2 id={id} className={styles.kicker}>
          {title}
        </h2>
        {side && <div className={`t-small ${styles.sectionSide}`}>{side}</div>}
      </div>
      {children}
    </section>
  )
}

export function Panel({ children, flush }: { children: ReactNode; flush?: boolean }) {
  return (
    <div className={styles.panel} data-flush={flush || undefined}>
      {children}
    </div>
  )
}

/** A key figure: the number large, its unit small beside it, its label under it. */
export function Stat({ value, unit, label, c }: { value: ReactNode; unit?: string; label: string; c?: Colour }) {
  return (
    <div className={styles.stat}>
      <dd className={`num ${styles.statValue}`} data-c={c}>
        {c && c !== 'green' && c !== 'magenta' ? <B c={c}>{value}</B> : value}
        {unit && <span className={styles.unit}>{unit}</span>}
      </dd>
      <dt className="t-small">{label}</dt>
    </div>
  )
}

export function Stats({ children, wide }: { children: ReactNode; wide?: boolean }) {
  return (
    <dl className={styles.stats} data-wide={wide || undefined}>
      {children}
    </dl>
  )
}

/** A small word on a block: a state or a kind. */
export function Chip({ c = 'line', children }: { c?: Colour; children: ReactNode }) {
  return (
    <B c={c} className={styles.chip}>
      {children}
    </B>
  )
}

/** What a place is for when it holds nothing yet, a read that failed, or a thing that does not exist. */
export function State({
  c = 'line',
  word,
  title,
  children,
  action,
}: {
  c?: Colour
  word: string
  title: string
  children?: ReactNode
  action?: ReactNode
}) {
  return (
    <div className={styles.state}>
      <span data-live={c === 'yellow' || undefined}>
        <Chip c={c}>{word}</Chip>
      </span>
      <p className={styles.stateTitle}>{title}</p>
      {children && <div className={`t-body ${styles.stateBody}`}>{children}</div>}
      {action && <div className={styles.stateAction}>{action}</div>}
    </div>
  )
}

/** Definition rows: a label and its value, for hashes, addresses and sources. */
export function Facts({ rows }: { rows: { label: string; value: ReactNode }[] }) {
  return (
    <dl className={styles.facts}>
      {rows.map((r) => (
        <div key={r.label}>
          <dt className="t-small">{r.label}</dt>
          <dd>{r.value}</dd>
        </div>
      ))}
    </dl>
  )
}

export function Skeleton({ w = '100%', h = 14 }: { w?: string | number; h?: number }) {
  return <span className="skel" style={{ width: w, height: h }} aria-hidden="true" />
}
