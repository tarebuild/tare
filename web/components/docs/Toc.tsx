'use client'

import { useEffect, useRef, useState } from 'react'
import styles from './docs.module.css'

/** The page's sections, with a mark that follows the one being read. */
export function Toc({ items }: { items: { id: string; title: string }[] }) {
  const nav = useRef<HTMLElement>(null)
  const [active, setActive] = useState(items[0]?.id)
  const [mark, setMark] = useState<{ y: number; h: number } | null>(null)

  useEffect(() => {
    let raf = 0
    const read = () => {
      raf = 0
      const line = innerHeight * 0.3
      let id = items[0]?.id
      for (const it of items) {
        const el = document.getElementById(it.id)
        if (el && el.getBoundingClientRect().top <= line) id = it.id
      }
      if (innerHeight + scrollY >= document.documentElement.scrollHeight - 4) id = items[items.length - 1]?.id
      setActive(id)
    }
    const on = () => (raf ||= requestAnimationFrame(read))
    read()
    addEventListener('scroll', on, { passive: true })
    addEventListener('resize', on)
    return () => {
      removeEventListener('scroll', on)
      removeEventListener('resize', on)
      cancelAnimationFrame(raf)
    }
  }, [items])

  useEffect(() => {
    const a = nav.current?.querySelector<HTMLElement>('[aria-current]')
    setMark(a ? { y: a.offsetTop, h: a.offsetHeight } : null)
  }, [active])

  return (
    <nav ref={nav} className={styles.toc} aria-label="On this page">
      <span className={styles.tocLabel}>On this page</span>
      {mark && <span className={styles.tocMark} style={{ height: mark.h, transform: `translateY(${mark.y}px)` }} aria-hidden="true" />}
      {items.map((s) => (
        <a key={s.id} href={`#${s.id}`} aria-current={active === s.id ? 'location' : undefined}>
          {s.title}
        </a>
      ))}
    </nav>
  )
}
