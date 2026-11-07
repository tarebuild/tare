// The places the nav carries. A plain module, so server and client components both read it.
export const PLACES = [
  { href: '/weigh', label: 'Weigh' },
  { href: '/bench', label: 'Bench' },
  { href: '/builds', label: 'Builds' },
  { href: '/docs', label: 'Docs' },
] as const
