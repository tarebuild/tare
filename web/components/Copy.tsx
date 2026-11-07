'use client'

import { useEffect, useState } from 'react'
import styles from './Copy.module.css'

/** A button that copies `text` and says so in place. */
export function CopyButton({ text, label = 'Copy' }: { text: string; label?: string }) {
  const [done, setDone] = useState(false)
  useEffect(() => {
    if (!done) return
    const t = setTimeout(() => setDone(false), 1600)
    return () => clearTimeout(t)
  }, [done])
  return (
    <button
      type="button"
      className={styles.copy}
      data-done={done}
      onClick={() => navigator.clipboard.writeText(text).then(() => setDone(true))}
    >
      {/* Both labels share one box, so the button keeps its size while one leaves and the other lands. */}
      <span className={styles.labels}>
        <span>{label}</span>
        <span aria-hidden="true">Copied</span>
      </span>
      <span className="sr-only" aria-live="polite">
        {done ? 'Copied' : ''}
      </span>
    </button>
  )
}

/** One shell line with its copy button: the install line, a check line. */
export function CommandLine({ text, prompt = '$' }: { text: string; prompt?: string }) {
  return (
    <div className={styles.line}>
      <code className={styles.code}>
        <span className={styles.prompt} aria-hidden="true">
          {prompt}
        </span>
        {text}
      </code>
      <CopyButton text={text} />
    </div>
  )
}
