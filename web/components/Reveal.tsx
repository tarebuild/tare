'use client'

import { useEffect } from 'react'

const grouped = new Intl.NumberFormat('en-US')

/** Counts a figure from `data-from` to `data-to` once, eased out. */
function count(el: HTMLElement) {
  const from = Number(el.dataset.from)
  const to = Number(el.dataset.to)
  const t0 = performance.now()
  const run = (now: number) => {
    const p = Math.min((now - t0) / 1300, 1)
    const e = 1 - Math.pow(1 - p, 4)
    el.textContent = grouped.format(Math.round(from + (to - from) * e))
    if (p < 1) requestAnimationFrame(run)
  }
  requestAnimationFrame(run)
}

/**
 * The landing's arrivals. Every `[data-reveal]` still under the fold is armed (its parts wait in
 * their start state) and let in when it comes into view. The server's HTML is the finished state,
 * so nothing depends on this running. Also tells the page when the first view has been left, and
 * where the pointer is.
 */
export function Reveal() {
  useEffect(() => {
    const root = document.documentElement
    const hero = document.getElementById('hero')
    const past = hero
      ? new IntersectionObserver(([e]) => root.toggleAttribute('data-past', !e?.isIntersecting), { rootMargin: '-45% 0px 0px 0px' })
      : null
    if (hero) past?.observe(hero)

    // The pointer: the source leans away from it, the title toward it, and the source is lit where it is.
    let raf = 0
    let to = { x: 0, y: 0, mx: innerWidth / 2, my: -innerHeight }
    const at = { ...to }
    const step = () => {
      at.x += (to.x - at.x) * 0.07
      at.y += (to.y - at.y) * 0.07
      at.mx += (to.mx - at.mx) * 0.14
      at.my += (to.my - at.my) * 0.14
      hero?.style.setProperty('--px', at.x.toFixed(4))
      hero?.style.setProperty('--py', at.y.toFixed(4))
      hero?.style.setProperty('--mx', `${at.mx.toFixed(1)}px`)
      hero?.style.setProperty('--my', `${at.my.toFixed(1)}px`)
      const still = Math.abs(to.x - at.x) + Math.abs(to.y - at.y) < 0.002 && Math.abs(to.mx - at.mx) + Math.abs(to.my - at.my) < 0.5
      raf = still ? 0 : requestAnimationFrame(step)
    }
    const move = (e: PointerEvent) => {
      if (e.pointerType !== 'mouse') return
      to = { x: (e.clientX / innerWidth) * 2 - 1, y: (e.clientY / innerHeight) * 2 - 1, mx: e.clientX, my: e.clientY }
      raf ||= requestAnimationFrame(step)
    }
    const leave = () => {
      to = { x: 0, y: 0, mx: to.mx, my: -innerHeight }
      raf ||= requestAnimationFrame(step)
    }

    let io: IntersectionObserver | null = null
    if (!matchMedia('(prefers-reduced-motion: reduce)').matches) {
      addEventListener('pointermove', move, { passive: true })
      root.addEventListener('pointerleave', leave)
      io = new IntersectionObserver(
        (entries) => {
          for (const e of entries) {
            if (!e.isIntersecting) continue
            const el = e.target as HTMLElement
            el.setAttribute('data-in', '')
            el.querySelectorAll<HTMLElement>('[data-count]').forEach(count)
            io?.unobserve(el)
          }
        },
        { rootMargin: '0px 0px -18% 0px' },
      )
      for (const el of document.querySelectorAll<HTMLElement>('[data-reveal]')) {
        if (el.getBoundingClientRect().top < innerHeight * 0.9) continue
        el.setAttribute('data-armed', '')
        el.querySelectorAll<HTMLElement>('[data-count]').forEach((c) => (c.textContent = grouped.format(Number(c.dataset.from))))
        io.observe(el)
      }
    }
    return () => {
      past?.disconnect()
      io?.disconnect()
      root.removeAttribute('data-past')
      removeEventListener('pointermove', move)
      root.removeEventListener('pointerleave', leave)
      cancelAnimationFrame(raf)
      for (const p of ['--px', '--py', '--mx', '--my']) hero?.style.removeProperty(p)
    }
  }, [])
  return null
}
