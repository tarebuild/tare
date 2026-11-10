import type { ReactNode } from 'react'
import { CopyButton } from '@/components/Copy'
import styles from './docs.module.css'

/** A shell block with a copy button. `$` lines are commands; the rest is what they print. */
export function Shell({ children }: { children: string }) {
  const text = children.trim()
  const commands = text
    .split('\n')
    .filter((l) => l.startsWith('$ '))
    .map((l) => l.slice(2))
    .join('\n')
  return (
    <div className={styles.code}>
      <pre>
        <code>
          {text.split('\n').map((l, i) => (
            <span key={i} className={l.startsWith('$ ') ? styles.cmd : l.startsWith('#') ? styles.note : styles.out}>
              {l}
              {'\n'}
            </span>
          ))}
        </code>
      </pre>
      <CopyButton text={commands || text} />
    </div>
  )
}

/** A block of source or data. */
export function Source({ children }: { children: string }) {
  return (
    <div className={styles.code}>
      <pre>
        <code>{children.trim()}</code>
      </pre>
      <CopyButton text={children.trim()} />
    </div>
  )
}

export function C({ children }: { children: ReactNode }) {
  return <code className={styles.inline}>{children}</code>
}

export function Table({ head, rows }: { head: string[]; rows: ReactNode[][] }) {
  return (
    <div className={`table-scroll ${styles.tableWrap}`}>
      <table className="table">
        <thead>
          <tr>
            {head.map((h, i) => (
              <th key={i} scope="col">
                {h}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((r, i) => (
            <tr key={i}>
              {r.map((c, j) => (
                <td key={j}>{c}</td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

export function List({ children }: { children: ReactNode }) {
  return <ul className={styles.list}>{children}</ul>
}

export function Steps({ children }: { children: ReactNode }) {
  return <ol className={styles.steps}>{children}</ol>
}

/** The few things that need setting apart. */
export function Note({ children }: { children: ReactNode }) {
  return <div className={styles.callout}>{children}</div>
}
