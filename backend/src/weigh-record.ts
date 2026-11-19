import { mkdir, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { Address } from '@tare/contract'
import { createRpc } from './engines/rpc/client.js'
import { sampleProgram } from './sampler.js'

const safeMessage = (error: unknown) => (error instanceof Error ? error.message : 'Recording failed.')
  .replace(/\b[a-z][a-z0-9+.-]*:\/\/\S+/gi, '[redacted]')

async function main() {
  const inputs = process.argv.slice(2)
  if (!inputs.length) {
    console.error(JSON.stringify({ program: null, status: 'error', message: 'Usage: weigh:record <program> [program ...]' }))
    process.exitCode = 1
    return
  }
  for (const input of inputs) {
    const parsed = Address.safeParse(input)
    const program = parsed.success ? parsed.data : null
    try {
      if (!program) throw new Error('The program address is invalid.')
      // Configuration errors belong in the same single-line result as RPC errors.
      const { env } = await import('./config.js')
      if (!env.HELIUS_RPC_URL) throw new Error('HELIUS_RPC_URL is required.')
      const rpc = createRpc({ url: env.HELIUS_RPC_URL, service: 'tare-weigh-record', rps: 5, scanIntervalMs: 10_000, log: () => {} })
      const { result, material } = await sampleProgram(rpc, program, env.WEIGH_SAMPLE_SIZE)
      if (!('sample' in result)) {
        console.log(JSON.stringify({ program, status: result.status, recorded: false }))
        continue
      }
      const folder = path.resolve('contract/fixtures/weigh')
      await mkdir(folder, { recursive: true })
      await writeFile(path.join(folder, `${program}.raw.json`), JSON.stringify(material, null, 2) + '\n', 'utf8')
      await writeFile(path.join(folder, `${program}.sample.json`), JSON.stringify(result.sample, null, 2) + '\n', 'utf8')
      console.log(JSON.stringify({ program, status: result.status, recorded: true, anchor: result.sample.anchor,
        hasIdl: result.sample.hasIdl, transactions: result.sample.transactions, signaturesExamined: result.sample.signaturesExamined, skippedVersions: material.skippedVersions, folder }))
    } catch (error) {
      process.exitCode = 1
      console.error(JSON.stringify({ program, status: 'error', message: safeMessage(error) }))
    }
  }
}

await main()
