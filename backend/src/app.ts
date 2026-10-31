import { createApi } from './engines/server/app.js'
import { writeLimit } from './engines/server/limits.js'
import * as s from '@tare/contract'
import { env } from './config.js'
import { fixtureRoutes } from './fixture-routes.js'

export function buildApp() {
  const app = createApi({
    origins: env.CORS_ORIGINS, allowHeaders: ['X-Fixture'], contractVersion: s.CONTRACT_VERSION,
    maxBodyBytes: 1024 * 1024,
  })
  app.use('*', writeLimit(12, ['POST']))
  app.route('/', fixtureRoutes())
  return { app, start: async () => {}, close: async () => {} }
}
