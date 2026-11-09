'use client'

import { useRouter } from 'next/navigation'
import { useState } from 'react'
import { isAddress } from '@/lib/format'
import styles from './page.module.css'

/** Filters the registry to one program's records. */
export function ProgramFilter({ value }: { value: string }) {
  const router = useRouter()
  const [text, setText] = useState(value)
  const [bad, setBad] = useState(false)

  const submit = (e: React.FormEvent) => {
    e.preventDefault()
    const address = text.trim()
    if (!address) return router.push('/builds')
    if (!isAddress(address)) return setBad(true)
    router.push(`/builds?program=${address}`)
  }

  return (
    <form className={styles.filter} onSubmit={submit} noValidate>
      <label className="sr-only" htmlFor="program-filter">
        Filter by program address
      </label>
      <input
        id="program-filter"
        className={`field ${styles.filterInput}`}
        value={text}
        onChange={(e) => {
          setText(e.target.value)
          setBad(false)
        }}
        placeholder="Filter by program address"
        autoComplete="off"
        spellCheck={false}
        aria-invalid={bad || undefined}
      />
      <button type="submit" className="btn" data-kind="secondary" data-size="sm">
        Filter
      </button>
      {value && (
        <button type="button" className="link t-small" onClick={() => router.push('/builds')}>
          Clear
        </button>
      )}
      <span className="sr-only" role="alert">
        {bad ? 'That is not a Solana address.' : ''}
      </span>
    </form>
  )
}
