'use client'

import { useRouter } from 'next/navigation'
import { useId, useState } from 'react'
import { isAddress } from '@/lib/format'
import styles from './AddressField.module.css'

/** Takes a program address and goes to its weighing. */
export function AddressField({ size = 'md', autoFocus = false }: { size?: 'md' | 'lg'; autoFocus?: boolean }) {
  const router = useRouter()
  const id = useId()
  const [value, setValue] = useState('')
  const [error, setError] = useState('')
  const [refused, setRefused] = useState(false)
  const [going, setGoing] = useState(false)
  const valid = isAddress(value.trim())

  const refuse = (message: string) => {
    setError(message)
    setRefused(true)
  }

  const submit = (e: React.FormEvent) => {
    e.preventDefault()
    const address = value.trim()
    if (!address) return refuse('Enter a program address.')
    if (!isAddress(address)) return refuse('That is not a Solana address: 32 to 44 base58 characters.')
    setGoing(true)
    router.push(`/weigh/${address}`)
  }

  return (
    <form className={styles.form} data-size={size} data-valid={valid || undefined} onSubmit={submit} noValidate>
      <label htmlFor={id} className="t-label">
        Program address
      </label>
      <div className={styles.row} data-refused={refused || undefined} onAnimationEnd={() => setRefused(false)}>
        <input
          id={id}
          className={`field ${styles.input}`}
          value={value}
          onChange={(e) => {
            setValue(e.target.value)
            if (error) setError('')
          }}
          placeholder="Enter address..."
          autoComplete="off"
          autoCapitalize="off"
          autoCorrect="off"
          spellCheck={false}
          autoFocus={autoFocus}
          aria-invalid={error ? true : undefined}
          aria-describedby={error ? `${id}-error` : undefined}
        />
        <button type="submit" className={`btn ${styles.go}`} disabled={going} data-going={going || undefined}>
          {going && <span className={styles.beam} aria-hidden="true" />}
          {going ? 'Weighing' : 'Weigh'}
        </button>
      </div>
      <p id={`${id}-error`} className={styles.error} role="alert">
        {error}
      </p>
    </form>
  )
}
