import Link from 'next/link'
import { PLACES } from '@/lib/places'

export default function NotFound() {
  return (
    <div className="wrap nf">
      <h1 className="nf-lines" data-land>
        <span className="b" data-c="white">
          404
        </span>
        <span className="b" data-c="magenta">
          nothing to weigh here
        </span>
      </h1>
      <p className="t-lead">This address holds no page. The places Tare has:</p>
      <nav className="nf-places" aria-label="Places">
        {PLACES.map((p) => (
          <Link key={p.href} href={p.href} className="btn" data-kind="secondary">
            {p.label}
          </Link>
        ))}
      </nav>
    </div>
  )
}
