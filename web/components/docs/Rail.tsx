'use client'

import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { useEffect, useRef, useState } from 'react'
import { GROUPS, docHref } from '@/lib/docs'
import styles from './docs.module.css'

export type SearchEntry = { href: string; page: string; section: string; text: string }

/** The docs rail: search over every page and section, then the chapters with the open one's pages. */
export function Rail({ index }: { index: SearchEntry[] }) {
  const path = usePathname()
  const [q, setQ] = useState('')
  const [open, setOpen] = useState(false)
  const input = useRef<HTMLInputElement>(null)
  const term = q.trim().toLowerCase()
  const words = term.split(/\s+/).filter(Boolean)
  const hits = term ? index.filter((e) => words.every((w) => e.text.includes(w))).slice(0, 10) : []

  useEffect(() => setOpen(false), [path])

  // "/" focuses the search from anywhere on a docs page.
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      const el = e.target as HTMLElement
      if (e.key !== '/' || el.tagName === 'INPUT' || el.tagName === 'TEXTAREA') return
      e.preventDefault()
      setOpen(true)
      input.current?.focus()
    }
    document.addEventListener('keydown', key)
    return () => document.removeEventListener('keydown', key)
  }, [])

  return (
    <aside className={styles.rail} data-open={open}>
      <div className={styles.railTop}>
        <div role="search" className={styles.searchBox}>
          <label className="sr-only" htmlFor="docs-search">
            Search the docs
          </label>
          <input
            ref={input}
            id="docs-search"
            className={`field ${styles.search}`}
            type="search"
            value={q}
            onChange={(e) => setQ(e.target.value)}
            onKeyDown={(e) => e.key === 'Escape' && setQ('')}
            placeholder="Search the docs"
            autoComplete="off"
          />
          <kbd className={styles.key} aria-hidden="true">
            /
          </kbd>
        </div>
        <button type="button" className={styles.railToggle} aria-expanded={open} onClick={() => setOpen((v) => !v)}>
          {open ? 'Close' : 'Chapters'}
        </button>
      </div>

      {term ? (
        <nav aria-label="Search results" className={styles.results}>
          {hits.length === 0 ? (
            <p className={styles.none}>Nothing in the docs matches &ldquo;{q.trim()}&rdquo;.</p>
          ) : (
            hits.map((h) => (
              <Link key={h.href} href={h.href} className={styles.hit} onClick={() => setQ('')}>
                {h.section}
                <span>{h.page}</span>
              </Link>
            ))
          )}
        </nav>
      ) : (
        <nav aria-label="Docs" className={styles.tree}>
          {GROUPS.map((g) => (
            <div key={g.label} className={styles.group}>
              <span className={styles.groupLabel}>{g.label}</span>
              {g.pages.map((c) => {
                const here = path === docHref(c.path)
                const inside = here || (c.children ?? []).some((p) => path === docHref(p.path))
                return (
                  <div key={c.path}>
                    <Link href={docHref(c.path)} className={styles.chapter} aria-current={here ? 'page' : undefined} data-inside={inside}>
                      {c.title}
                    </Link>
                    {inside && c.children && (
                      <div className={styles.children}>
                        {c.children.map((p) => (
                          <Link key={p.path} href={docHref(p.path)} className={styles.child} aria-current={path === docHref(p.path) ? 'page' : undefined}>
                            {p.title}
                          </Link>
                        ))}
                      </div>
                    )}
                  </div>
                )
              })}
            </div>
          ))}
        </nav>
      )}
    </aside>
  )
}
