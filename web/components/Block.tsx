import type { ReactNode } from 'react'

export type Colour = 'white' | 'green' | 'magenta' | 'yellow' | 'cyan' | 'red' | 'blue' | 'none' | 'line'

/** A word or phrase on its own solid ground. */
export function B({ c = 'white', children, className }: { c?: Colour; children: ReactNode; className?: string }) {
  return (
    <span className={className ? `b ${className}` : 'b'} data-c={c}>
      {children}
    </span>
  )
}

const ARMS = [0, 60, 120]

/** The six-armed asterisk that separates phrases. Drawn, so it holds its weight in every face and size. */
export function Ast({ c, label }: { c?: Colour; label?: string }) {
  const glyph = (
    <svg className="ast" viewBox="-50 -50 100 100" aria-hidden={label ? undefined : true} role={label ? 'img' : undefined} aria-label={label}>
      {ARMS.map((a) => (
        <rect key={a} x={-9.5} y={-50} width={19} height={100} transform={`rotate(${a})`} />
      ))}
    </svg>
  )
  return c ? <B c={c}>{glyph}</B> : glyph
}

/** One line of blocks, kept on a single row. */
export function Row({ children, className }: { children: ReactNode; className?: string }) {
  return <span className={className} style={{ display: 'flex', flexWrap: 'wrap', gap: '0 0.14em', alignItems: 'baseline' }}>{children}</span>
}
