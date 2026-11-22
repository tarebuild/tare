'use client'

import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { useEffect, useRef, useState } from 'react'
import { LINKS } from '@/lib/facts'
import { PLACES } from '@/lib/places'
import { GitHubMark } from './GitHubMark'
import { Lockup } from './Logo'
import styles from './Nav.module.css'

export function Nav() {
  const path = usePathname()
  const [open, setOpen] = useState(false)
  const [scrolled, setScrolled] = useState(false)
  const [hover, setHover] = useState<string | null>(null)
  const [pill, setPill] = useState({ x: 0, w: 0, on: false, slide: false })
  const places = useRef<HTMLElement>(null)
  const sheet = useRef<HTMLDivElement>(null)
  const toggle = useRef<HTMLButtonElement>(null)

  useEffect(() => setOpen(false), [path])

  useEffect(() => {
    const on = () => setScrolled(window.scrollY > 4)
    on()
    window.addEventListener('scroll', on, { passive: true })
    return () => window.removeEventListener('scroll', on)
  }, [])

  // The phone sheet: holds focus, closes on Escape, stops the page scrolling under it.
  useEffect(() => {
    if (!open) return
    const prev = document.body.style.overflow
    document.body.style.overflow = 'hidden'
    const focusables = () => Array.from(sheet.current?.querySelectorAll<HTMLElement>('a, button') ?? [])
    focusables()[0]?.focus()
    const key = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setOpen(false)
        toggle.current?.focus()
      }
      if (e.key !== 'Tab') return
      const f = [toggle.current!, ...focusables()]
      const i = f.indexOf(document.activeElement as HTMLElement)
      const next = e.shiftKey ? (i <= 0 ? f.length - 1 : i - 1) : i === f.length - 1 ? 0 : i + 1
      e.preventDefault()
      f[next]?.focus()
    }
    document.addEventListener('keydown', key)
    return () => {
      document.body.style.overflow = prev
      document.removeEventListener('keydown', key)
    }
  }, [open])

  const current = (href: string) => path === href || path.startsWith(`${href}/`)

  // One pill under the places: it rests on the current one and travels to whichever is pointed at.
  const target = hover ?? PLACES.find((p) => current(p.href))?.href ?? null
  useEffect(() => {
    const el = places.current
    if (!el) return
    const measure = () => {
      const a = target ? el.querySelector<HTMLElement>(`a[href="${target}"]`) : null
      setPill((was) => (a ? { x: a.offsetLeft, w: a.offsetWidth, on: true, slide: was.on } : { ...was, on: false, slide: false }))
    }
    measure()
    const ro = new ResizeObserver(measure)
    ro.observe(el)
    return () => ro.disconnect()
  }, [target])

  return (
    <>
      <header className={styles.bar} data-scrolled={scrolled || open}>
        <div className={styles.capsule}>
          <Link href="/" className={styles.home} aria-label="Tare, home">
            <Lockup height={15} />
          </Link>
          <nav ref={places} className={styles.places} aria-label="Places" data-pill={pill.w > 0 || undefined} onPointerLeave={() => setHover(null)}>
            <span
              className={styles.pill}
              data-on={pill.on}
              data-slide={pill.slide}
              style={{ width: pill.w, transform: `translateX(${pill.x}px)` }}
              aria-hidden="true"
            />
            {PLACES.map((p) => (
              <Link
                key={p.href}
                href={p.href}
                prefetch={false}
                className={styles.place}
                aria-current={current(p.href) ? 'page' : undefined}
                onPointerEnter={(e) => e.pointerType === 'mouse' && setHover(p.href)}
                onFocus={() => setHover(p.href)}
                onBlur={() => setHover(null)}
              >
                {p.label}
              </Link>
            ))}
          </nav>
          <a href={LINKS.source} className={styles.source} target="_blank" rel="noreferrer" aria-label="Source on GitHub" title="Source on GitHub">
            <GitHubMark size={18} />
          </a>
          <Link href="/docs/install" prefetch={false} className={`btn ${styles.action}`} data-size="sm">
            Install
          </Link>
          <button
            ref={toggle}
            type="button"
            className={styles.toggle}
            aria-expanded={open}
            aria-controls="nav-sheet"
            onClick={() => setOpen((v) => !v)}
          >
            {open ? 'Close' : 'Menu'}
          </button>
        </div>
      </header>
      <div id="nav-sheet" ref={sheet} className={styles.sheet} data-state={open ? 'open' : 'closed'} hidden={!open}>
        <nav className={styles.sheetPlaces} aria-label="Places">
          {PLACES.map((p) => (
            <Link key={p.href} href={p.href} onClick={() => setOpen(false)} className={styles.sheetPlace} aria-current={current(p.href) ? 'page' : undefined}>
              {p.label}
            </Link>
          ))}
        </nav>
        <div className={styles.sheetActions}>
          <Link href="/docs/install" onClick={() => setOpen(false)} className="btn">
            Install
          </Link>
          <a href={LINKS.source} className="btn" data-kind="secondary" target="_blank" rel="noreferrer">
            <GitHubMark /> Source on GitHub
          </a>
        </div>
      </div>
    </>
  )
}
