import { PublicKey } from '@solana/web3.js'
import { z } from 'zod'
import { Address, Count, ProgramSample, type WeighResponse } from '@tare/contract'
import { measureTransaction, groupMeasurements, type InstructionMeasurement } from './compute.js'
import { normalizeTransaction } from './engines/indexer/transaction.js'
import { type Rpc, RpcError } from './engines/rpc/client.js'
import { legacyIdlAddress, metadataIdlAddress, parseLegacyIdl, parseMetadataIdl, type ProgramIdl } from './program-idl.js'

const SIGNATURE_PAGE_SIZE = 100
const MAX_SIGNATURE_PAGES = 10
// A program whose recent signatures mostly reference it without running it, or mostly fail, would
// otherwise cost a getTransaction for every successful row on ten pages. These bound one sample.
const MAX_TRANSACTIONS_FETCHED = 300
const MAX_FETCHED_WITHOUT_MEASUREMENT = 100
const FAILED_CALLS_CHECKED = 3
const UPGRADEABLE_LOADER = 'BPFLoaderUpgradeab1e11111111111111111111111'
// Account data is binary payload, not a value to validate with a backtracking regex.
const accountSchema = z.object({
  owner: Address, executable: z.boolean(), space: Count,
  data: z.tuple([z.string(), z.literal('base64')]),
})
const accountReply = z.object({ value: accountSchema.nullable() })
const signaturesSchema = z.array(z.object({ signature: z.string(), err: z.unknown() }))
type RawAccount = z.output<typeof accountSchema>
export type SampleResult = Exclude<WeighResponse, { status: 'sampling' }>
export type SampleMaterial = {
  program: string
  programAccount: RawAccount | null
  accountReplies: { address: string; reply: unknown }[]
  rentQueries: { sizeBytes: number; result: unknown }[]
  signatureReply: unknown
  signaturePages: { before: string | null; reply: unknown }[]
  selectedSignatures: string[]
  programDataAccount: RawAccount | null
  idlAccount: RawAccount | null
  metadataIdlAccount: RawAccount | null
  rentLamports: number
  signatures: z.output<typeof signaturesSchema>
  transactions: unknown[]
  transactionReplies: { signature: string; reply: unknown }[]
  skippedVersions: number
  batchFallback: boolean
}

const bytes = (account: RawAccount) => Buffer.from(account.data[0], 'base64')
type TransactionReply = { result: unknown } | { error: { code: number; message: string } }

/**
 * Account reads transfer at most a program header and no program-data bytes.
 * The full allocation size is supplied by RPC's space field, independently of dataSlice.
 * Optional IDLs add at most two reads. A rejected batch switches once to single reads.
 */
export async function sampleProgram(rpc: Rpc, program: string, limit: number): Promise<{ result: SampleResult; material: SampleMaterial }> {
  const material: SampleMaterial = {
    program, programAccount: null, accountReplies: [], rentQueries: [], signatureReply: null, signaturePages: [], selectedSignatures: [],
    programDataAccount: null, idlAccount: null, metadataIdlAccount: null, rentLamports: 0,
    signatures: [], transactions: [], transactionReplies: [], skippedVersions: 0, batchFallback: false,
  }

  async function readAccount(address: string, slice?: { offset: number; length: number }) {
    const reply = await rpc.call('getAccountInfo', [address, {
      encoding: 'base64', commitment: 'confirmed', ...(slice ? { dataSlice: slice } : {}),
    }])
    material.accountReplies.push({ address, reply })
    return reply
  }

  async function readIdl(address: string, parse: typeof parseLegacyIdl): Promise<{ raw: RawAccount | null; idl: ProgramIdl | null }> {
    const reply = await readAccount(address)
    const parsed = accountReply.safeParse(reply)
    if (!parsed.success || !parsed.data.value) return { raw: null, idl: null }
    const raw = parsed.data.value
    return { raw, idl: parse(bytes(raw), raw.owner, program) }
  }

  material.programAccount = accountReply.parse(await readAccount(program, { offset: 0, length: 36 })).value
  if (!material.programAccount?.executable) return { result: { status: 'not_program', program, records: [] }, material }
  const programData = bytes(material.programAccount)
  let binarySize = material.programAccount.space
  let rentSizes = [material.programAccount.space]
  if (material.programAccount.owner === UPGRADEABLE_LOADER) {
    if (material.programAccount.space !== 36 || programData.length !== 36 || programData.readUInt32LE(0) !== 2)
      throw new RpcError('getAccountInfo', -1, 'Invalid upgradeable program account.')
    const programDataAddress = new PublicKey(programData.subarray(4, 36)).toBase58()
    material.programDataAccount = accountReply.parse(await readAccount(programDataAddress, { offset: 0, length: 0 })).value
    if (!material.programDataAccount || material.programDataAccount.owner !== UPGRADEABLE_LOADER || material.programDataAccount.space < 45)
      throw new RpcError('getAccountInfo', -1, 'Program data account is unavailable or invalid.')
    binarySize = material.programDataAccount.space - 45
    rentSizes = [material.programAccount.space, material.programDataAccount.space]
  }
  for (const size of rentSizes) {
    const result = await rpc.call('getMinimumBalanceForRentExemption', [size, { commitment: 'confirmed' }])
    material.rentQueries.push({ sizeBytes: size, result })
    material.rentLamports += Count.parse(result)
  }
  Count.parse(material.rentLamports)

  const legacy = await readIdl(await legacyIdlAddress(program), parseLegacyIdl)
  material.idlAccount = legacy.raw
  let idl = legacy.idl
  if (!idl) {
    const metadata = await readIdl(metadataIdlAddress(program), parseMetadataIdl)
    material.metadataIdlAccount = metadata.raw
    idl = metadata.idl
  }

  const selected: InstructionMeasurement[][] = []
  const seen = new Set<string>()
  const transactionParams = (signature: string) => [signature, { encoding: 'json', commitment: 'confirmed', maxSupportedTransactionVersion: 1 }]

  async function single(signature: string): Promise<TransactionReply> {
    try { return { result: await rpc.call('getTransaction', transactionParams(signature)) } }
    catch (error) {
      if (error instanceof RpcError) return { error: { code: error.code, message: error.message } }
      throw error
    }
  }

  const exhausted = () => material.transactionReplies.length >= MAX_TRANSACTIONS_FETCHED ||
    (!selected.length && material.transactionReplies.length >= MAX_FETCHED_WITHOUT_MEASUREMENT)
  let before: string | undefined
  for (let pageNumber = 0; pageNumber < MAX_SIGNATURE_PAGES && selected.length < limit && !exhausted(); pageNumber++) {
    const reply = await rpc.call('getSignaturesForAddress', [program, {
      limit: SIGNATURE_PAGE_SIZE, commitment: 'confirmed', ...(before ? { before } : {}),
    }])
    material.signatureReply ??= reply
    material.signaturePages.push({ before: before ?? null, reply })
    const rows = signaturesSchema.max(SIGNATURE_PAGE_SIZE).parse(reply)
    if (!rows.length) break
    const page = rows.filter((row) => {
      if (seen.has(row.signature)) return false
      seen.add(row.signature)
      return true
    })
    material.signatures.push(...page)
    // A signature error means no committed invocation can contribute. Never fetch it.
    const candidates = page.filter((row) => row.err == null)
    for (let start = 0; start < candidates.length && selected.length < limit && !exhausted(); start += 20) {
      const batch = candidates.slice(start, start + 20)
      let replies: TransactionReply[]
      if (material.batchFallback) {
        replies = await Promise.all(batch.map(({ signature }) => single(signature)))
      } else {
        try {
          replies = await rpc.batch<unknown>(batch.map(({ signature }) => ({ method: 'getTransaction', params: transactionParams(signature) })))
        } catch (error) {
          // An unscoped batch refusal is retried once as individual reads.
          if (!(error instanceof RpcError) || (error.code !== -32600 && error.code !== -32015)) throw error
          material.batchFallback = true
          replies = await Promise.all(batch.map(({ signature }) => single(signature)))
        }
      }
      for (const [index, transaction] of replies.entries()) {
        const signature = batch[index]!.signature
        material.transactionReplies.push({ signature, reply: transaction })
        if ('error' in transaction) {
          if (transaction.error.code === -32015) { material.skippedVersions++; continue }
          if (selected.length < limit) throw new RpcError('getTransaction', transaction.error.code, transaction.error.message)
          continue
        }
        if (transaction.result === null) continue
        material.transactions.push(transaction.result)
        if (selected.length === limit) continue
        const measurements = measureTransaction(transaction.result, program)
        if (measurements?.length) {
          selected.push(measurements)
          material.selectedSignatures.push(signature)
        }
      }
    }
    if (rows.length < SIGNATURE_PAGE_SIZE) break
    before = rows.at(-1)!.signature
  }
  // Nothing measurable: a few failed rows tell a program whose calls all fail from one never called.
  const failed = material.signatures.filter((row) => row.err != null)
  let callsFailed = false
  if (!selected.length && failed.length) {
    for (const { signature } of failed.slice(0, FAILED_CALLS_CHECKED)) {
      const reply = await single(signature)
      if (!('result' in reply) || reply.result === null) continue
      if (normalizeTransaction(reply.result).logs.some((line) => line.startsWith(`Program ${program} invoke [`))) { callsFailed = true; break }
    }
  }
  const { anchor: observedAnchor, ...grouped } = groupMeasurements(selected, idl?.names ?? new Map())
  const sample = ProgramSample.parse({
    program: { address: program, name: idl?.name ?? null }, anchor: Boolean(idl) || observedAnchor, hasIdl: Boolean(idl),
    binarySize, rentLamports: material.rentLamports, sampledAt: new Date().toISOString(),
    signaturesExamined: material.signatures.length, signaturesFailed: failed.length, ...grouped,
  })
  const status = sample.transactions ? 'ready' : callsFailed ? 'calls_failed' : 'no_transactions'
  return { result: { status, sample, records: [] }, material }
}
