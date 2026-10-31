// Start the built fixture API independently of MODE in .env.
process.env.MODE = 'fixture'
await import('../dist/server.js')
