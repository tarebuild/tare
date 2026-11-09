'use client'

import { useRouter } from 'next/navigation'
import { useEffect } from 'react'

/** Reads the weighing again after the service's own delay, while the tab is in view. */
export function Resample({ seconds }: { seconds: number }) {
  const router = useRouter()
  useEffect(() => {
    const t = setInterval(() => {
      if (!document.hidden) router.refresh()
    }, seconds * 1000)
    return () => clearInterval(t)
  }, [router, seconds])
  return (
    <>
      <span className="tick" style={{ '--t': `${seconds}s` } as React.CSSProperties} aria-hidden="true" />
      <span className="sr-only" role="status">
        Sampling in progress
      </span>
    </>
  )
}
