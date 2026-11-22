import type { Metadata } from 'next'
import Image from 'next/image'
import { CopyButton } from '@/components/Copy'
import { Facts, Page, PageHead, Section } from '@/components/ui'
import { SITE_URL } from '@/lib/facts'
import { kb, units } from '@/lib/format'
import { PAPER } from '@/lib/paper'
import cover from './cover.png'
import styles from './page.module.css'

export const metadata: Metadata = {
  title: 'Whitepaper',
  description: `${PAPER.title}. ${PAPER.subtitle}.`,
}

const year = PAPER.date.match(/\d{4}/)?.[0] ?? ''
const bibtex = `@misc{tare${year},
  title  = {${PAPER.title}: ${PAPER.subtitle}},
  author = {${PAPER.authors}},
  year   = {${year}},
  note   = {Version ${PAPER.version}},
  url    = {${SITE_URL}${PAPER.href}}
}`

export default function PaperPage() {
  return (
    <Page>
      <PageHead
        title={[
          { t: 'whitepaper', c: 'line' },
          { t: `v${PAPER.version}`, c: 'green' },
        ]}
        lead={
          <>
            <span className={styles.title}>{PAPER.title}</span>
            <span className={styles.subtitle}>{PAPER.subtitle}</span>
          </>
        }
      >
        <a className="btn" href={PAPER.href} download>
          Download PDF
        </a>
        <a className="btn" data-kind="secondary" href={PAPER.href} target="_blank" rel="noreferrer">
          Open in the browser
        </a>
      </PageHead>

      <div className={styles.paper}>
        <a className={styles.cover} href={PAPER.href} target="_blank" rel="noreferrer" aria-label={`Open ${PAPER.title}, PDF`}>
          <Image src={cover} alt="" sizes="(max-width: 860px) 70vw, 420px" priority />
        </a>
        <div className={styles.side}>
          <Section title="Abstract" id="abstract-h">
            <div className={styles.abstract}>
              {PAPER.abstract.map((p, i) => (
                <p key={i}>{p}</p>
              ))}
            </div>
          </Section>
          <Facts
            rows={[
              { label: 'Authors', value: PAPER.authors },
              { label: 'Version', value: <span className="num">{PAPER.version}</span> },
              { label: 'Date', value: PAPER.date },
              { label: 'Length', value: `${units(PAPER.pages)} pages, PDF, ${kb(PAPER.bytes)}` },
            ]}
          />
        </div>
      </div>

      <Section title="Contents" id="contents-h" side={`${PAPER.sections.filter((x) => /^\d/.test(x.n)).length} sections`}>
        <ol className={styles.contents}>
          {PAPER.sections.map((s) => (
            <li key={s.title}>
              <span className="num">{s.n}</span>
              <span>{/^[A-Z]$/.test(s.n) ? `Appendix: ${s.title}` : s.title}</span>
              {s.page != null && <span className="num">{s.page}</span>}
            </li>
          ))}
        </ol>
      </Section>

      <Section title="Cite it" id="cite-h" side={<CopyButton text={bibtex} label="Copy BibTeX" />}>
        <pre className={styles.bib}>
          <code>{bibtex}</code>
        </pre>
      </Section>
    </Page>
  )
}
