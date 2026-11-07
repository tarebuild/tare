'use client'

import Lenis from 'lenis'
import 'lenis/dist/lenis.css'
import { useEffect } from 'react'

/** The landing's scroll, eased. Working pages and the docs keep the native scroll. */
export function SmoothScroll() {
  useEffect(() => {
    if (matchMedia('(prefers-reduced-motion: reduce)').matches) return
    const lenis = new Lenis({ autoRaf: true, lerp: 0.11, anchors: true })
    return () => lenis.destroy()
  }, [])
  return null
}
