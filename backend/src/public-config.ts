import { env } from './config.js'

export function publicConfig() {
  return {
    siteUrl: env.SITE_PUBLIC_URL ?? null,
    recordUrlTemplate: env.SITE_PUBLIC_URL ? `${env.SITE_PUBLIC_URL.replace(/\/+$/, '')}/builds/{record}` : null,
    installLine: env.API_PUBLIC_URL ? `curl -fsSL ${env.API_PUBLIC_URL}/install.sh | sh` : null,
  }
}
