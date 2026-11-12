import { createHash } from 'node:crypto'
import { gunzipSync, inflateSync } from 'node:zlib'
import { PublicKey } from '@solana/web3.js'
import { z } from 'zod'

// Legacy layout/namespace: coral-xyz/anchor v0.32.1 lang/src/idl.rs and
// v0.29.0 lang/attribute/account/src/lib.rs. Modern JSON: v0.30.1 idl/spec/src/lib.rs.
// PMP layout/derivation: https://github.com/otter-sec/anchor/blob/master/ts/packages/anchor/src/idl.ts
// and https://github.com/solana-program/program-metadata/blob/main/clients/js/src/generated/accounts/metadata.ts
export const PROGRAM_METADATA_ID = 'ProgM6JCCvbYkfKqJYHePx4xxSUSqJp7rh8Lyv7nk7S'
const LEGACY_DISCRIMINATOR = createHash('sha256').update('internal:IdlAccount').digest().subarray(0, 8)
const MAX_IDL_BYTES = 1024 * 1024
const METADATA_SEED = Buffer.alloc(16)
METADATA_SEED.write('idl', 'utf8')
const decoder = new TextDecoder('utf-8', { fatal: true })
const instruction = z.object({ name: z.string().min(1), accounts: z.array(z.unknown()), args: z.array(z.unknown()) })
const legacyDocument = z.object({ name: z.string().min(1), version: z.string().min(1), instructions: z.array(instruction) })
const modernDocument = z.object({
  address: z.string(),
  metadata: z.object({ name: z.string().min(1), version: z.string().min(1), spec: z.string().min(1) }),
  instructions: z.array(instruction.extend({ discriminator: z.array(z.int().min(0).max(255)).min(1).max(32) })),
})

export type ProgramIdl = { name: string; names: Map<string, string>; anchor: true }

export async function legacyIdlAddress(program: string): Promise<string> {
  const key = new PublicKey(program)
  const base = PublicKey.findProgramAddressSync([], key)[0]
  return (await PublicKey.createWithSeed(base, 'anchor:idl', key)).toBase58()
}

export function metadataIdlAddress(program: string): string {
  // The canonical authority seed is an empty byte array, not 32 zero bytes.
  return PublicKey.findProgramAddressSync([new PublicKey(program).toBuffer(), Buffer.alloc(0), METADATA_SEED], new PublicKey(PROGRAM_METADATA_ID))[0].toBase58()
}

function parseDocument(data: Uint8Array, program: string): ProgramIdl | null {
  const value: unknown = JSON.parse(decoder.decode(data))
  const modern = modernDocument.safeParse(value)
  if (modern.success) {
    if (modern.data.address !== program) return null
    return {
      name: modern.data.metadata.name, anchor: true,
      // Preserve explicit discriminators. A consumer using an eight-byte prefix should leave
      // non-eight-byte discriminators unnamed instead of inventing a legacy hash for them.
      names: new Map(modern.data.instructions.map((ix) => [Buffer.from(ix.discriminator).toString('hex'), ix.name])),
    }
  }
  const legacy = legacyDocument.safeParse(value)
  if (!legacy.success) return null
  const names = new Map<string, string>()
  for (const ix of legacy.data.instructions) {
    // Anchor <=0.29 hashes the snake_case Rust handler name in the global namespace.
    const rustName = ix.name.replace(/([A-Z])([A-Z][a-z])/g, '$1_$2').replace(/([a-z0-9])([A-Z])/g, '$1_$2').toLowerCase()
    names.set(createHash('sha256').update(`global:${rustName}`).digest().subarray(0, 8).toString('hex'), ix.name)
  }
  return { name: legacy.data.name, names, anchor: true }
}

/** An absent, foreign, incomplete or malformed IDL is optional metadata, never a sampling failure. */
export function parseLegacyIdl(rawData: Uint8Array, owner: string, program: string): ProgramIdl | null {
  if (owner !== program) return null
  const data = Buffer.from(rawData.buffer, rawData.byteOffset, rawData.byteLength)
  if (data.length < 44 || !data.subarray(0, 8).equals(LEGACY_DISCRIMINATOR)) return null
  const size = data.readUInt32LE(40)
  if (!size || size > MAX_IDL_BYTES || size > data.length - 44) return null
  try {
    return parseDocument(inflateSync(data.subarray(44, 44 + size), { maxOutputLength: MAX_IDL_BYTES }), program)
  } catch {
    return null
  }
}

/** Only authenticated canonical, directly embedded UTF-8 JSON is read; no URL/account dereferencing. */
export function parseMetadataIdl(rawData: Uint8Array, owner: string, program: string): ProgramIdl | null {
  if (owner !== PROGRAM_METADATA_ID) return null
  const data = Buffer.from(rawData.buffer, rawData.byteOffset, rawData.byteLength)
  // u8 discriminator, program[32], zeroable authority[32], mutable, canonical, seed[16],
  // encoding, compression, format, dataSource, u32 dataLength, reserved[5], then payload.
  if (data.length < 96 || data[0] !== 2 || data[66] !== 1 || !data.subarray(67, 83).equals(METADATA_SEED)
    || data[83] !== 1 || data[85] !== 1 || data[86] !== 0) return null
  const size = data.readUInt32LE(87)
  if (!size || size > MAX_IDL_BYTES || size > data.length - 96) return null
  try {
    if (!data.subarray(1, 33).equals(new PublicKey(program).toBuffer())) return null
    const payload = data.subarray(96, 96 + size)
    const compression = data[84]
    const decoded = compression === 0 ? payload
      : compression === 1 ? gunzipSync(payload, { maxOutputLength: MAX_IDL_BYTES })
      : compression === 2 ? inflateSync(payload, { maxOutputLength: MAX_IDL_BYTES }) : null
    return decoded ? parseDocument(decoded, program) : null
  } catch {
    return null
  }
}
