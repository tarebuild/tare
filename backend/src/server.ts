import { env } from './config.js'
import { buildApp } from './app.js'
import { serve } from './engines/server/serve.js'

const { app, start, close } = buildApp()
await start()
const { server } = serve(app, { port: env.PORT, host: env.HOST, close })
server.keepAliveTimeout = 65_000
server.headersTimeout = 66_000
