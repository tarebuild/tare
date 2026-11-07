import path from 'node:path'
import type { NextConfig } from 'next'

const config: NextConfig = {
  turbopack: { root: path.resolve(__dirname) },
  devIndicators: false,
  poweredByHeader: false,
  // The token is told inside the docs; the old address still leads there.
  redirects: async () => [{ source: '/token', destination: '/docs/token', permanent: false }],
}

export default config
