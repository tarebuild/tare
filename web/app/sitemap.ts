import type { MetadataRoute } from 'next'
import { FLAT, docHref } from '@/lib/docs'
import { SITE_URL } from '@/lib/facts'

export default function sitemap(): MetadataRoute.Sitemap {
  return ['/', '/weigh', '/bench', '/builds', '/paper', ...FLAT.map((d) => docHref(d.path))].map((path) => ({ url: `${SITE_URL}${path}` }))
}
