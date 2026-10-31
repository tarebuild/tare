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
