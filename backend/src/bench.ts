import { BenchResults, Compat } from '@tare/contract'
import { emptyBench, emptyCompat } from '@tare/contract/fixtures'
import { fileDocument } from './documents.js'
import { fail } from './errors.js'
import type { Database } from './db.js'
import { env } from './config.js'
import { publicConfig } from './public-config.js'

export function createBenchReader(db?: Database['db']) {
  const results = fileDocument('bench/results.json', BenchResults, emptyBench)
  const compatibility = fileDocument('bench/compat.json', Compat, emptyCompat)
  async function measured() {
    const value = await results()
    if (value.fixture) fail('INVALID_REPORT', 'Fixture data cannot be served as measured benchmark data.')
    return value
  }
  return {
    async results() {
      const value = await measured()
      // Links describe this API's indexed registry, never a URL supplied by the file.
      const programs = value.programs.map((program) => {
        if (program.status !== 'measured') return program
        const { recordUrl: _recordUrl, ...row } = program
        return row
      })
      const template = publicConfig().recordUrlTemplate
      if (!db || !template) return { ...value, programs }
      const candidates = programs.filter((row) => row.status === 'measured'
        && (row.recordCluster === undefined ? !row.record
          : (row.recordCluster === 'mainnet-beta' ? 'mainnet' : row.recordCluster) === env.CLUSTER))
      const filters = candidates.flatMap((row) => row.status === 'measured' && (row.record || row.weighing.program.address) ? [{
        ...(row.record ? { address: row.record } : {}),
        ...(row.weighing.program.address ? { program: row.weighing.program.address } : {}),
        stockBuildHash: row.weighing.builds.stock.hash,
        tareBuildHash: row.weighing.builds.tare.hash,
        corpusHash: row.proof.corpus.hash,
        ...(row.source.repository ? { sourceRepository: row.source.repository } : {}),
        ...(row.source.commit ? { sourceCommit: row.source.commit } : {}),
      }] : [])
      if (!filters.length) return { ...value, programs }
      const indexed = await db.record.findMany({ where: { OR: filters },
        orderBy: [{ slot: 'desc' }, { address: 'desc' }],
        select: { address: true, program: true, stockBuildHash: true, tareBuildHash: true, corpusHash: true, sourceRepository: true, sourceCommit: true },
      })
      return { ...value, programs: programs.map((row) => {
        if (row.status !== 'measured' || !candidates.includes(row)) return row
        const record = indexed.find((record) => (!row.record || record.address === row.record)
          && (!row.weighing.program.address || record.program === row.weighing.program.address)
          && record.stockBuildHash === row.weighing.builds.stock.hash
          && record.tareBuildHash === row.weighing.builds.tare.hash && record.corpusHash === row.proof.corpus.hash
          && (!row.source.repository || record.sourceRepository === row.source.repository)
          && (!row.source.commit || record.sourceCommit === row.source.commit))
        return record ? { ...row, record: record.address, recordCluster: row.recordCluster ?? env.CLUSTER,
          recordUrl: template.replace('{record}', record.address) } : row
      }) }
    },
    async totals() { return (await measured()).totals },
    async compatibility() {
      const value = await compatibility()
      if (value.fixture) fail('INVALID_REPORT', 'Fixture data cannot be served as measured compatibility data.')
      return value
    },
  }
}
