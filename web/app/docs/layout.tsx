import { Rail, type SearchEntry } from '@/components/docs/Rail'
import styles from '@/components/docs/docs.module.css'
import { CONTENT } from '@/content/docs'
import { FLAT, docHref } from '@/lib/docs'

const index: SearchEntry[] = FLAT.flatMap((d) => {
  const doc = CONTENT[d.path]
  const page = d.path === d.chapter.path ? d.group : d.chapter.title
  return [
    { href: docHref(d.path), page, section: d.title, text: `${d.title} ${d.says} ${doc?.lead ?? ''} ${doc?.banner.map((b) => b.t).join(' ') ?? ''}`.toLowerCase() },
    ...(doc?.sections ?? []).map((s) => ({
      href: `${docHref(d.path)}#${s.id}`,
      page: d.title,
      section: s.title,
      text: `${s.title} ${d.title} ${d.chapter.title}`.toLowerCase(),
    })),
  ]
})

export default function DocsLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className={`wrap ${styles.shell}`}>
      <Rail index={index} />
      {children}
    </div>
  )
}
