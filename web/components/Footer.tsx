import Link from 'next/link'
import { LINKS } from '@/lib/facts'
import { DOCS } from '@/lib/docs'
import { Lockup } from './Logo'
import styles from './Footer.module.css'

const PRODUCT = [
  { href: '/weigh', label: 'Weigh a program' },
  { href: '/bench', label: 'The benchmark' },
  { href: '/builds', label: 'The registry' },
]

export function Footer() {
  const outside = [
    LINKS.source && { href: LINKS.source, label: 'Source' },
    LINKS.x && { href: LINKS.x, label: 'X' },
    { href: LINKS.solar, label: 'Solar, the idea Tare carries' },
  ].filter((l): l is { href: string; label: string } => Boolean(l))

  return (
    <footer className={styles.footer}>
      <div className={`wrap ${styles.grid}`}>
        <div className={styles.brand}>
          <Link href="/" aria-label="Tare, home" className={styles.home}>
            <Lockup height={22} />
          </Link>
          <p className="t-small">A build tool for Anchor programs on Solana, and a public registry of what it built.</p>
        </div>
        <nav aria-label="Product" className={styles.col}>
          <h2 className="t-label">Product</h2>
          <ul>
            {PRODUCT.map((l) => (
              <li key={l.href}>
                <Link href={l.href}>{l.label}</Link>
              </li>
            ))}
          </ul>
        </nav>
        <nav aria-label="Docs" className={styles.col}>
          <h2 className="t-label">Docs</h2>
          <ul className={styles.two}>
            {DOCS.map((d) => (
              <li key={d.slug}>
                <Link href={d.slug ? `/docs/${d.slug}` : '/docs'}>{d.title}</Link>
              </li>
            ))}
          </ul>
        </nav>
        <nav aria-label="Elsewhere" className={styles.col}>
          <h2 className="t-label">Elsewhere</h2>
          <ul>
            {outside.map((l) => (
              <li key={l.href}>
                <a href={l.href} rel="noreferrer" target="_blank">
                  {l.label}
                </a>
              </li>
            ))}
          </ul>
        </nav>
      </div>
    </footer>
  )
}
