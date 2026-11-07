import type { Metadata, Viewport } from 'next'
import { Archivo, Geist_Mono } from 'next/font/google'
import { Footer } from '@/components/Footer'
import { Nav } from '@/components/Nav'
import { DESCRIPTION, NAME, SITE_URL } from '@/lib/facts'
import './globals.css'

const archivo = Archivo({ subsets: ['latin'], axes: ['wdth'], variable: '--font-archivo', display: 'swap' })
const mono = Geist_Mono({ subsets: ['latin'], variable: '--font-mono', display: 'swap' })

export const metadata: Metadata = {
  metadataBase: new URL(SITE_URL),
  title: { default: `${NAME}: the Anchor program you wrote, at hand-written cost`, template: `%s · ${NAME}` },
  description: DESCRIPTION,
  openGraph: { title: NAME, description: DESCRIPTION, siteName: NAME, type: 'website' },
  twitter: { card: 'summary_large_image', title: NAME, description: DESCRIPTION },
}

export const viewport: Viewport = { themeColor: '#000000', colorScheme: 'dark' }

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className={`${archivo.variable} ${mono.variable}`}>
      <body>
        <a className="skip" href="#main">
          Skip to content
        </a>
        <Nav />
        <main id="main">{children}</main>
        <Footer />
      </body>
    </html>
  )
}
