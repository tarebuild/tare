'use client'

import { useEffect, useRef } from 'react'
import type { Colour } from './Block'
import type { Stanza, Token } from '@/lib/lines'
import styles from './Cascade.module.css'

const SHUFFLE: Colour[] = ['green', 'cyan', 'magenta', 'yellow', 'red', 'blue', 'white']
const ARMS = [0, 60, 120]

// A small deterministic hash, so the server's still and the first client frame agree.
function hash(a: number, b: number, c: number) {
  let h = (a * 374761393 + b * 668265263 + c * 2147483647) | 0
  h = (h ^ (h >>> 13)) * 1274126177
  return ((h ^ (h >>> 16)) >>> 0) / 4294967296
}

function colourAt(stanza: number, word: number, beat: number, token: Token): Colour {
  if (token.role) return token.role
  const r = hash(stanza + 1, word + 1, beat + 1)
  if (r < (token.ast ? 0.3 : 0.46)) return 'none'
  return SHUFFLE[Math.floor(hash(word + 3, beat + 7, stanza + 5) * SHUFFLE.length)] ?? 'white'
}

/** The lit state of the build sequence: most words take a block, each its own colour for good. */
function litAt(stanza: number, word: number, token: Token): Colour {
  if (token.role) return token.role
  if (hash(stanza + 11, word + 5, 3) < 0.3) return 'none'
  return SHUFFLE[Math.floor(hash(word + 3, 7, stanza + 5) * SHUFFLE.length)] ?? 'white'
}

/** Whether the edit takes this word away. */
const cutAt = (stanza: number, word: number, token: Token) => !token.role && hash(stanza + 2, word + 9, 4) < 0.36

/** The built state: what the edit leaves, a few words still on green. */
const builtAt = (stanza: number, word: number): Colour => (hash(stanza + 4, word + 1, 8) < 0.16 ? 'green' : 'none')

const TICK = 62 // ms: one word is typed per tick
const BEAT = 5 // ticks a block keeps its colour
const LAG = 7 // ticks between one stanza starting and the next

/**
 * The weighing cascade: stanzas of block lines stepping diagonally down a black field,
 * typed on and off word by word in a wave. Its lines are the product's own: a weighing's
 * rows or a command's output.
 */
export function Cascade({
  stanzas,
  amplitude = 4,
  delay = 0,
  hold: held = 26,
  spread = false,
  mode = 'shuffle',
  className,
}: {
  stanzas: Stanza[]
  amplitude?: number
  /** Ticks this field runs ahead, so neighbouring fields are out of step. */
  delay?: number
  /** Ticks a stanza stays whole before it is typed off. */
  hold?: number
  /** Spreads the stanzas' starts over the whole loop, so the field is never dark at once. */
  spread?: boolean
  /**
   * `shuffle` types stanzas on and off in a wave while their blocks change colour.
   * `build` runs the field as one program: written in order, lit in order, then edited down in order.
   */
  mode?: 'shuffle' | 'build'
  className?: string
}) {
  const root = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const el = root.current
    if (!el || matchMedia('(prefers-reduced-motion: reduce)').matches) return

    const groups = Array.from(el.querySelectorAll<HTMLElement>('[data-stanza]')).map((s) =>
      Array.from(s.querySelectorAll<HTMLElement>('[data-w]')),
    )
    const flat = stanzas.map((s) => s.flat())
    const longest =Math.max(...groups.map((g) => g.length))
    const hold = longest + held
    const period = spread ? hold + longest + 36 : hold + longest + groups.length * LAG + 10
    const lag = spread ? period / groups.length : LAG
    const shown = groups.map((g) => g.map((w) => w.style.visibility !== 'hidden'))
    const colour = groups.map((g) => g.map((w) => w.dataset.c ?? 'none'))

    let raf = 0
    let visible = true
    let last = -1
    const start = performance.now()

    // The build sequence over the whole field, in words: write, light, edit, hold, clear.
    const first: number[] = []
    let total = 0
    for (const g of groups) {
      first.push(total)
      total += g.length
    }
    const WRITE = 3 // words per tick
    const SWEEP = 5
    const tLight = Math.ceil(total / WRITE) + 14
    const tEdit = tLight + Math.ceil(total / SWEEP) + 22
    const tClear = tEdit + Math.ceil(total / SWEEP) + 70
    const loop = tClear + Math.ceil(total / 12) + 10

    const build = (tick: number) => {
      // Starts where the server's still stands, the built state, just before it clears.
      const u = (tick + tClear - 14) % loop
      for (let s = 0; s < groups.length; s++) {
        const words = groups[s]!
        for (let w = 0; w < words.length; w++) {
          const g = first[s]! + w
          const el = words[w]!
          const token = flat[s]![w]!
          const written = g < u * WRITE
          const lit = g < (u - tLight) * SWEEP
          const edited = g < (u - tEdit) * SWEEP
          const cleared = g < (u - tClear) * 12
          const cut = cutAt(s, w, token)
          const show = written && !cleared && !(edited && cut)
          if (shown[s]![w] !== show) {
            shown[s]![w] = show
            el.style.visibility = show ? 'visible' : 'hidden'
          }
          if (!show) continue
          const c: Colour = edited ? builtAt(s, w) : lit ? litAt(s, w, token) : 'none'
          if (colour[s]![w] !== c) {
            colour[s]![w] = c
            el.dataset.c = c
          }
        }
      }
    }

    const frame = (now: number) => {
      raf = requestAnimationFrame(frame)
      if (!visible || document.hidden) return
      const tick = Math.floor(Math.max(0, now - start) / TICK) + delay
      if (tick === last) return
      last = tick
      if (mode === 'build') return build(tick)
      const beat = Math.floor(tick / BEAT)
      for (let s = 0; s < groups.length; s++) {
        const words = groups[s]!
        const u = (((tick - Math.round(s * lag)) % period) + period) % period
        const on = u // words typed on so far
        const off = u - hold // words typed off again, from the start of the stanza
        for (let w = 0; w < words.length; w++) {
          const el = words[w]!
          const show = w < on && w >= off
          if (shown[s]![w] !== show) {
            shown[s]![w] = show
            el.style.visibility = show ? 'visible' : 'hidden'
          }
          if (!show) continue
          const c = colourAt(s, w, beat, flat[s]![w]!)
          if (colour[s]![w] !== c) {
            colour[s]![w] = c
            el.dataset.c = c
          }
        }
      }
    }
    raf = requestAnimationFrame(frame)

    const io = new IntersectionObserver(([e]) => {
      visible = e?.isIntersecting ?? true
    })
    io.observe(el)
    return () => {
      cancelAnimationFrame(raf)
      io.disconnect()
    }
  }, [stanzas, delay, held, spread, mode])

  return (
    <div ref={root} className={className ? `${styles.cascade} ${className}` : styles.cascade} aria-hidden="true">
      {stanzas.map((stanza, s) => {
        // The indent climbs and falls in a triangle wave, one step per stanza.
        const span = amplitude * 2
        const m = span === 0 ? 0 : s % span
        const indent = span === 0 ? 0 : m <= amplitude ? m : span - m
        let w = 0
        return (
          <div key={s} data-stanza className={styles.stanza} style={{ '--i': indent } as React.CSSProperties}>
            {stanza.map((line, l) => (
              <div key={l} className={styles.line}>
                {line.map((token) => {
                  const i = w++
                  return (
                    <span key={i} data-w className="b" data-c={mode === 'build' ? builtAt(s, i) : colourAt(s, i, 0, token)} style={{ marginLeft: token.gap ? `${token.gap}ch` : undefined, visibility: mode === 'build' && cutAt(s, i, token) ? 'hidden' : undefined }}>
                      {token.ast ? (
                        <svg className="ast" viewBox="-50 -50 100 100">
                          {ARMS.map((a) => (
                            <rect key={a} x={-9.5} y={-50} width={19} height={100} transform={`rotate(${a})`} />
                          ))}
                        </svg>
                      ) : (
                        token.t
                      )}
                    </span>
                  )
                })}
              </div>
            ))}
          </div>
        )
      })}
    </div>
  )
}
