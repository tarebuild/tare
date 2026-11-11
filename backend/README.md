# Tare backend services

Node 22, TypeScript, Hono, Zod and Prisma 7 over PostgreSQL. One package runs the API and registry indexer against the same database. The API samples deployed programs on request; it never polls mainnet.

## Run

From `backend/` on Windows:

```powershell
npm.cmd ci
npm.cmd run build
npm.cmd run fixture
```

The fixture command serves `127.0.0.1:7810`. It needs no RPC or database. Set `PORT` for a separate instance. Existing API processes keep running during a build; restart the intended service to load a new build.

For real services, set `MODE=chain` and `DATABASE_URL`, then:

```powershell
$env:MODE='chain'
$env:PORT='7812'
npm.cmd start
```

In another terminal, with its own port:

```powershell
$env:MODE='chain'
$env:PORT='7813'
npm.cmd run start:indexer
```

Both start scripts apply migrations before launching the role. Build runs `prisma generate` without contacting PostgreSQL. Stop with Ctrl+C; the worker stops indexing and releases its leader lease. `npm.cmd run dev` watches API source. `npm.cmd test` runs only the two approved suites: compute grouping and registry decoding.

## Contract 0.7.1

`contract/` is `@tare/contract@0.7.1`: browser-safe Zod schemas, a typed fetch client, fixture data, OpenAPI 3.1, and generated JSON Schema draft 2020-12. File formats retain `schemaVersion: "1"`.

Instruction `gross` and `net` are the median CU for **one call** over successful measured inputs. `tare = gross - net`. An unreached instruction has `inputs: 0` and all three weights null. Totals sum the per-instruction figures once for each reached instruction; they do not multiply by input counts. Registry and corpus totals follow this convention. API aggregate counts and totals remain safe JSON integers; tare may be negative. Uploaded weighings and proof documents accept safe JSON integers or canonical decimal strings for every typed u64: instruction weights and input counts, build sizes and rent, corpus and comparison counts, account lamports and rent epochs, input slots, and optimizer/reproduction counters. Signed tare values also accept exact i64 decimal strings. Upload schemas preserve these representations. Public read schemas convert safe decimal strings to numbers for weights, sizes, rent totals, counts and slots; their inferred output types are numeric. Account balances and rent epochs read consistently as decimal strings. Unsafe arithmetic values produce a read error instead of rounding. The contract build includes type assertions for the site's numeric fields. Compatibility construct names are plain strings.

```powershell
npm.cmd run contract:pack
```

The tarballs are in `contract/pack/`; 0.1.0 through 0.7.0 remain there. Copy `tare-contract-0.7.1.tgz` into the web project's `vendor/`, then install `file:vendor/tare-contract-0.7.1.tgz`. Do not use a backend-relative or symlinked package in the web deployment.

```ts
import { createClient, ApiClientError } from '@tare/contract'
const api = createClient('http://127.0.0.1:7810')
const benchmark = await api.bench()
const result = await api.weigh(program)
```

A weighing or proof report may carry `optimization` (bytecode sizes, hashes and rewrite statistics) and `recordedCorpus` (recorded-input reproduction provenance). The two metadata objects preserve additional producer keys; the rest of the contract remains strict. Their schemas include the current Rust writer fields and retain compatibility with null metadata and earlier recordings.

The client validates responses. `ApiClientError` exposes `code`, `status`, and `body.error`. All failures use `{error:{code,message,retryable,stage,...}}`. Ordinary empty lists and `not_program`, `no_transactions`, `sampling`, and record `report_unavailable` remain successful responses.

## Routes

| Route | Response |
| --- | --- |
| GET `/health` | 200 `{status: "fixture" | "waiting" | "ready"}` |
| GET `/v1/config` | Site URL, record URL template, and nullable `installLine` |
| GET `/install.sh` | POSIX installer with the configured release base URL |
| GET `/releases/{file}` | Release archive, checksum or latest version; plain filenames only |
| GET `/v1/bench` | Benchmark document |
| GET `/v1/bench/totals` | Totals from the same document |
| GET `/v1/compat` | Compatibility document |
| GET `/v1/registry/totals` | Record, program and flag counts, weights, corpus total |
| GET `/v1/records?program=&cursor=&limit=` | Records newest first; default 24, max 100 |
| GET `/v1/records/{record}` | Record, report, flag inputs and check command |
| GET `/v1/weigh?limit=` | Recently sampled programs |
| GET `/v1/weigh/{program}` | Start sampling or return stored result and registry records |
| POST `/v1/reports` | UploadProofReport JSON returns 201 `{hash,location}` |
| GET `/v1/reports/{hash}` | Normalized ProofReport with explicit Accept: application/json; otherwise original bytes |
| POST `/v1/flag-inputs` | UploadFlagInput JSON returns 201 `{hash,location}` |
| GET `/v1/flag-inputs/{hash}` | Normalized FlagInput with explicit Accept: application/json; otherwise original bytes |
| GET `/v1/token` | Token facts; unknown values null |

Records use keyset pagination by descending slot then address. Pass `page.nextCursor` unchanged and retain the program filter. A missing record fails with `RECORD_NOT_FOUND`; missing report content leaves its record and flags visible with `status: "report_unavailable"`.

Both fixture and chain modes serve the current benchmark and compatibility files from `bench/`. Fixture scenarios do not replace these two files. Readers check file identity, timestamps and size on each request, sharing one concurrent read. A revision is published only after the file is stable and the complete document validates. During a partial write, invalid replacement or temporary removal, requests keep the last good document. The next valid revision appears without restarting the API. With no successful read yet, an absent file returns an empty document; an invalid file returns `INVALID_REPORT` (422). Invalid revisions log `data_validation_failed` with the first failing field path once per cached revision. Real routes refuse `fixture: true`; services never write into `bench/`.

Measured benchmark rows retain `record: string | null` and may carry `recordCluster`. In chain mode, `/v1/bench` adds `recordUrl` only when the matching record exists in this API's indexed registry and `SITE_PUBLIC_URL` supplies `recordUrlTemplate`. The lookup checks program address, both build hashes, corpus hash and available source identity. If a row has no attached record, the newest matching indexed record is attached to the response with the current `CLUSTER`. This enrichment never changes the file. An explicit attachment is preserved; if its cluster differs, is unknown, or the indexed record is absent or mismatched, no link is added. The Solana cluster label `mainnet-beta` is treated as `mainnet`. A `recordUrl` supplied by a file is ignored; the API derives it from its own template. Fixture mode preserves attachment data without making registry links. `/v1/bench/totals` reads the same validated file and needs no database query.

Use one registry cluster per database, as for the existing indexer. Record rows are not partitioned by cluster; changing the API's `CLUSTER` while reusing another cluster's indexed database is not a supported deployment. The website should link a row only when its `recordUrl` is present.
