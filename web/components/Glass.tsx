'use client'

import { useEffect, useId, useRef, useState, type ReactNode } from 'react'
import styles from './Glass.module.css'

// Displacement map for a w x h rounded rectangle: neutral inside, pulled inward across the bezel.
function lensMap(w: number, h: number, r: number, bezel: number) {
  const c = Object.assign(document.createElement('canvas'), { width: w, height: h })
  const g = c.getContext('2d')!
  const img = g.createImageData(w, h)
  const hw = w / 2
  const hh = h / 2
  r = Math.min(r, hw, hh)
  for (let y = 0; y < h; y++)
    for (let x = 0; x < w; x++) {
      const px = x + 0.5 - hw
      const py = y + 0.5 - hh
      const qx = Math.abs(px) - (hw - r)
      const qy = Math.abs(py) - (hh - r)
      const d = -(Math.hypot(Math.max(qx, 0), Math.max(qy, 0)) + Math.min(Math.max(qx, qy), 0) - r)
      let nx = 0
      let ny = 0
      let m = 0
      if (d >= 0 && d < bezel) {
        if (qx > 0 && qy > 0) {
          const l = Math.hypot(qx, qy) || 1
          nx = qx / l
          ny = qy / l
        } else if (qx > qy) nx = 1
        else ny = 1
        nx *= Math.sign(px) || 1
        ny *= Math.sign(py) || 1
        const t = 1 - d / bezel
        m = t * t * (3 - 2 * t) * t
      }
      const i = (y * w + x) * 4
      img.data[i] = 128 - nx * m * 127
      img.data[i + 1] = 128 - ny * m * 127
      img.data[i + 2] = 128
      img.data[i + 3] = 255
    }
  g.putImageData(img, 0, 0)
  return c.toDataURL()
}

type Lens = { w: number; h: number; map: string; bend: number }

/**
 * A piece of glass over whatever passes behind it: its edge bends the backdrop, its body frosts it,
 * and a rim catches the light. Lensing runs where the engine takes an SVG filter as a backdrop
 * (Chromium); elsewhere the glass frosts without bending.
 */
export function Glass({ children, className }: { children: ReactNode; className?: string }) {
  const el = useRef<HTMLSpanElement>(null)
  const id = `lens-${useId().replace(/[^a-zA-Z0-9]/g, '')}`
  const [lens, setLens] = useState<Lens | null>(null)

  useEffect(() => {
    const node = el.current
    if (!node || !('chrome' in window)) return
    let timer = 0
    const draw = () => {
      const w = Math.round(node.offsetWidth)
      const h = Math.round(node.offsetHeight)
      if (w < 8 || h < 8) return
      const radius = parseFloat(getComputedStyle(node).borderTopLeftRadius) || 0
      const bezel = Math.max(8, Math.min(h * 0.3, 34))
      setLens({ w, h, map: lensMap(w, h, radius, bezel), bend: Math.round(bezel * 1.5) })
    }
    const ro = new ResizeObserver(() => {
      clearTimeout(timer)
      timer = window.setTimeout(draw, 120)
    })
    ro.observe(node)
    draw()
    return () => {
      ro.disconnect()
      clearTimeout(timer)
    }
  }, [])

  const filter = lens ? `url(#${id}) blur(9px) saturate(180%) brightness(0.9)` : undefined

  return (
    <span
      ref={el}
      className={className ? `${styles.glass} ${className}` : styles.glass}
      style={filter ? { backdropFilter: filter, WebkitBackdropFilter: filter } : undefined}
    >
      {lens && (
        <svg className={styles.defs} width="0" height="0" aria-hidden="true">
          <filter id={id} filterUnits="userSpaceOnUse" colorInterpolationFilters="sRGB" x="0" y="0" width={lens.w} height={lens.h}>
            <feImage href={lens.map} x="0" y="0" width={lens.w} height={lens.h} preserveAspectRatio="none" result="map" />
            <feDisplacementMap in="SourceGraphic" in2="map" scale={lens.bend} xChannelSelector="R" yChannelSelector="G" />
          </filter>
        </svg>
      )}
      {children}
    </span>
  )
}
