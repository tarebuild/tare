import type { Metadata } from 'next'
import { notFound } from 'next/navigation'
import { Article } from '@/components/docs/Article'
import { CONTENT } from '@/content/docs'
import { FLAT, findDoc } from '@/lib/docs'

type Props = { params: Promise<{ path?: string[] }> }

export const dynamicParams = false

export function generateStaticParams() {
  return FLAT.map((d) => ({ path: d.path ? d.path.split('/') : [] }))
}

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const path = ((await params).path ?? []).join('/')
  const doc = findDoc(path)
  if (!doc) return {}
  const title = doc.path === doc.chapter.path ? doc.title : `${doc.title} · ${doc.chapter.title}`
  return { title: `${title} · Docs`, description: CONTENT[path]?.lead }
}

export default async function DocPage({ params }: Props) {
  const path = ((await params).path ?? []).join('/')
  if (!findDoc(path) || !CONTENT[path]) notFound()
  return <Article path={path} />
}
