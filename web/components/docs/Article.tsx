import Link from 'next/link'
import { B } from '@/components/Block'
import { CONTENT } from '@/content/docs'
import { FLAT, docHref } from '@/lib/docs'
import { Toc } from './Toc'
import styles from './docs.module.css'

/** One docs page: where it sits, its opening line, its sections, its chapter's pages, and the way on. */
export function Article({ path }: { path: string }) {
  const i = FLAT.findIndex((d) => d.path === path)
  const meta = FLAT[i]!
  const doc = CONTENT[path]!
  const prev = FLAT[i - 1]
  const next = FLAT[i + 1]
  const isChapter = meta.path === meta.chapter.path
  const pages = isChapter ? (meta.chapter.children ?? []) : []

  return (
    <>
      <article key={path} className={styles.article}>
        <header className={styles.header}>
          <nav className={styles.crumbs} aria-label="Breadcrumb">
            <Link href="/docs">Docs</Link>
            <span aria-hidden="true">/</span>
            <span>{meta.group}</span>
            {!isChapter && (
              <>
                <span aria-hidden="true">/</span>
                <Link href={docHref(meta.chapter.path)}>{meta.chapter.title}</Link>
              </>
            )}
          </nav>
          <h1 className={styles.title}>{meta.title}</h1>
          <div className={styles.banner} aria-hidden="true" data-land>
            {doc.banner.map((b) => (
              <B key={b.t} c={b.c}>
                {b.t}
              </B>
            ))}
          </div>
          <p className={styles.lead}>{doc.lead}</p>
        </header>

        <div className={styles.body}>
          {doc.sections.map((s) => (
            <section key={s.id} id={s.id} className={styles.section} aria-labelledby={`${s.id}-h`}>
              <h2 id={`${s.id}-h`}>
                <a href={`#${s.id}`}>{s.title}</a>
              </h2>
              {s.body}
            </section>
          ))}

          {pages.length > 0 && (
            <section className={styles.section} aria-labelledby="chapter-h">
              <h2 id="chapter-h">In this chapter</h2>
              <ul className={styles.pages}>
                {pages.map((p) => (
                  <li key={p.path}>
                    <Link href={docHref(p.path)}>
                      <b>{p.title}</b>
                      <span>{p.says}</span>
                    </Link>
                  </li>
                ))}
              </ul>
            </section>
          )}
        </div>

        <nav className={styles.pager} aria-label="Previous and next">
          {prev ? (
            <Link href={docHref(prev.path)}>
              <span>Previous</span>
              <b>{prev.title}</b>
            </Link>
          ) : (
            <span />
          )}
          {next && (
            <Link href={docHref(next.path)} className={styles.next}>
              <span>Next</span>
              <b>{next.title}</b>
            </Link>
          )}
        </nav>
      </article>

      <Toc
        key={`toc/${path}`}
        items={[...doc.sections.map((s) => ({ id: s.id, title: s.title })), ...(pages.length > 0 ? [{ id: 'chapter-h', title: 'In this chapter' }] : [])]}
      />
    </>
  )
}
