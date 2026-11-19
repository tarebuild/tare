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

## Registry worker and waiting

Set `REGISTRY_PROGRAM_ID`, `RPC_URL` (or the retained `SOLANA_RPC_URL` alias), and optionally `REGISTRY_IDL_PATH` (default `registry/target/idl/tare_registry.json`). The IDL address must match the configured program. Missing IDL or program ID gives healthy waiting and empty registry reads. A missing IDL is checked locally every 30 seconds so its arrival needs no restart.

All roles share one `ServiceReadiness` reservation: at most one `getMultipleAccounts` call every 30 seconds checks the executable program and its IDL-decoded Config. They become ready automatically after deployment and initialization. Health requests themselves make no RPC calls.

Localnet defaults are commitment `confirmed`, indexer polling every 2 seconds, and 50 RPC methods/second. Other clusters default to `finalized`, 15 seconds and 5 methods/second. `REGISTRY_COMMITMENT`, `INDEXER_POLL_MS`, and `REGISTRY_RPC_RPS` can override these. The commitment applies to the readiness probe, slot fence, signatures, transactions and block ordering. A failed cycle waits at least 60 seconds. No program-account scans occur. Unchanged signature heads fetch no transactions.

The worker decodes the real stock IDL's `RecordAttested`, `RecordFlagged`, `ConfigInitialized`, and `ConfigUpdated` events only from authenticated self-CPI inner instructions. `source_commit` is lowercase hex text. The record and flag address is derived and checked from the actual seeds:

- Record: `["record", program, report_hash, attester]`
- Flag: `["flag", record, input_hash]`
- Config: `["config"]`

The indexer stages raw facts, applies bounded atomic pages in chain order, advances cursors last, and runs under a renewable database lease released on shutdown. Event config facts remain in the staged transaction; current readiness and fees are read from the actual chain accounts. Corpus counts are stored as PostgreSQL bigint to match the ABI's u64 within the public contract's safe-integer range.

`/v1/registry/totals` returns `feesAccruedLamports`: the attest fees paid to the launch wallet, summed over indexed records from `RecordAttested.fee_lamports`. No RPC call. Record pages load their stored report and flag inputs by hash and retain the `tare check <record>` command even if content is unavailable.


## Deployed-program sampling

A cache miss or stale entry returns `sampling` with `Retry-After: 2` and starts one background run. Requests for the same program share it. Poll the endpoint after the advertised delay; successful, not-program and no-transaction results are stored with sample time. Cached requests make zero mainnet calls. A cached sample without signaturesExamined is refreshed once because it predates the usable-transaction sampling policy. Failed runs surface an error on the next poll; the user can retry. There is no sampling timer.

`WEIGH_SAMPLE_SIZE` is the target number of **usable transactions**, not the number of signatures fetched. Sampling pages through at most ten pages of 100 signatures (1,000 distinct candidates), stopping when it has enough usable transactions or reaches the history/page bound. A usable transaction contributes at least one attributable invocation of the requested program. Merely naming the program as an account does not count. Failed signatures are excluded before any transaction fetch.

The sample's `signaturesExamined` reports distinct signatures returned and inspected across these pages, including failures and non-invoking transactions. It is always supplied by new samples; older recordings may omit it, meaning unknown. `transactions` reports only the selected usable transactions, never more than the configured target. A partial nonzero sample is `ready`; `no_transactions` means no usable invocation was found inside the scanned bound, not that no signature mentioned the address.

**Maximum per run: 1,016 RPC methods and 66 HTTP requests with batching.** This is six account/IDL/rent reads, ten signature pages, and at most 1,000 transaction reads in batches of 20. If one batch is refused (-32600) or returns a batch-wide version error (-32015), the run switches once to single reads: the worst bound becomes 1,036 methods and 1,017 HTTP requests. Most runs stop well before these maxima. No rate-limit or other provider error is retried. Pacing counts every method at 5 methods/second; there is no background polling.

Null transactions, missing metadata or logs, version refusals (-32015), and un-attributable calls do not fill the target, so sampling continues within the same page bound. v0 instruction indexes resolve static keys followed by loaded writable and readonly keys. Full raw replies, signature pages and the selected signatures are saved by the recorder for diagnosis.

Program-data bytes are never downloaded. `getAccountInfo` with `dataSlice: {offset: 0, length: 0}` returns empty data plus the full `space`; the two captured mainnet responses confirm this, including Whirlpools' 10 MiB allocation. For upgradeable programs, binary size is `space - 45` (the loader metadata), including reserved zero padding. Rent comes from current `getMinimumBalanceForRentExemption` queries for the program and program-data spaces. Other executable programs use their account's `space`. Account payload strings are decoded directly only when needed; no regex is run over base64 account data.

IDL reads are optional metadata. Legacy Anchor storage uses the derived `create_with_seed(PDA([], program), "anchor:idl", program)` account, the discriminator `sha256("internal:IdlAccount")[..8]`, authority, little-endian length, and compressed JSON. Both pre-0.30 JSON (`name` and computed `global:<snake_name>` discriminators) and 0.30+ JSON (`metadata.name` and explicit discriminators) are supported. If legacy metadata is absent or unusable, the canonical Program Metadata PDA is checked for directly embedded UTF-8 JSON, uncompressed or gzip/zlib-compressed. No external metadata URL is fetched. IDLs larger than the 1 MiB parsing cap, malformed accounts, bad compression, and non-IDL JSON all mean no IDL rather than a failed sample.

`hasIdl: true` means a valid supported IDL was read. `anchor: true` means either that IDL was read or an accepted invocation contained an Anchor-style `Instruction: Name` log whose snake-case name hashes to that invocation's eight-byte discriminator. This is observed Anchor-compatible behavior, not proof of source provenance. No observation means `anchor: false`, which does not prove the program was built without Anchor. Program names and instruction labels still come only from an IDL; without one they remain null and the UI uses discriminators.

Compute is read from matched runtime invoke/consumed frames, including inner and recursive invocations. Failed transactions and known failed inner invocations do not count. When logs are truncated, every attributable target invocation whose invoke and consumed lines appeared before the cutoff still contributes, even if its success line was cut off. Calls with no consumed line remain unmeasured. Untruncated malformed traces remain strict.

CU is inclusive of child CPIs; recursive calls also have their own rows, so summing recursive invocations can overlap execution cost. Median and p90 use nearest rank, `sorted[ceil(p * count) - 1]`, keeping integer CU. Share uses total accepted invocation CU.

The recorded Drift failure is explained in [the 50-signature analysis](docs/lead-runs/drift-weigh-analysis.md): 44 signatures failed, and six successful transactions used Drift only as a readonly account. None actually invoked it. The new paging searches beyond that first page instead of declaring the program unweighable.

## Record real weigh fixtures

The lead with a Helius key runs:

```powershell
npm.cmd run weigh:record -- <program-address>
```

This runs one bounded sample per supplied address, prints one JSON line per program, and writes `contract/fixtures/weigh/<address>.raw.json` and `<address>.sample.json`. The raw file contains accounts, rent result, signatures, raw transaction replies and skip counts, with no RPC URL or credential. Fixture mode prefers a recorded sample for that address; explicit scenarios still override it. Until recordings exist, fictional fixtures remain in use. Absent IDLs and non-program addresses are ordinary results. Real failures print a redacted message, no stack or URL, and set a non-zero exit code. Repack the contract after adding recordings.

Use `X-Fixture` or `?fixture=`: `default`, `ready`, `empty`, `waiting`, `anchor`, `non-anchor`, `not-program`, `no-transactions`, `sampling`, `with-records`, `report-unavailable`, `diverged`, or any exported error code. Fictional fixture addresses are exported as `fixturePrograms`. Real mode exposes the preview at `/fixture`.

## Install and publish the CLI

`API_PUBLIC_URL` is the public API origin (scheme, host and optional port). With it configured, `/v1/config` supplies `installLine`, for example:

```sh
curl -fsSL http://127.0.0.1:7816/install.sh | sh
```

Without a configured API origin, `installLine` is null. `/install.sh` uses `RELEASES_BASE_URL` when set, otherwise `API_PUBLIC_URL/releases`; without either it returns the `CONFIGURATION_REQUIRED` envelope. The same routes work in fixture and chain modes. The browser can use `api.config().installLine` and hide the install action while it is null.

Package an existing binary from `backend/` (this does not build or modify the toolchain):

```powershell
npm.cmd run release:package -- toolchain/target/release/tare.exe x86_64-pc-windows-msvc 0.3.0
```

The packager creates `releases/tare-0.3.0-x86_64-pc-windows-msvc.tar.gz`, its `.sha256`, and `releases/latest`. The archive contains only `tare.exe` for Windows or `tare` elsewhere. Run it once per target with the same version. Supported triples are `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin` and `x86_64-pc-windows-msvc`. Linux binaries must run on the deployment's intended glibc baseline; packaging does not cross-compile or verify that baseline.

For a local check, set `MODE=fixture`, `HOST=127.0.0.1`, `PORT=7816`, `API_PUBLIC_URL=http://127.0.0.1:7816`, and leave `RELEASES_BASE_URL` blank. Run `npm.cmd run build` and `npm.cmd run fixture`, then run the install line above in Git Bash. The final line should be `tare 0.3.0`. Stop this separate server with Ctrl+C after checking it.

Publish in either of two ways:

- Keep the packaged files in `backend/releases/` beside the API, configure `API_PUBLIC_URL`, and leave `RELEASES_BASE_URL` unset. They are ignored by git but explicitly included by `.railwayignore`. When uploading with the Railway CLI, use `railway up --no-gitignore` so git's release exclusion does not remove them. The files must exist before the service upload; a git-only deployment must put them there during its build.
- Upload the same archives, `.sha256` files and `latest` to an external release directory (for example, a bucket or a GitHub release), and set `RELEASES_BASE_URL` to that directory's download URL. Keep `API_PUBLIC_URL` set for the install line. The API then needs no local release files. The external base must serve `latest` too; GitHub's download/latest path alone does not create that file.

The script selects the OS and architecture, fetches `latest` unless `TARE_VERSION` is set, verifies SHA-256, installs to `${TARE_INSTALL_DIR:-$HOME/.tare/bin}`, prints a PATH command if needed, and runs the installed binary's `--version`. `curl`, `tar`, and either `sha256sum` or `shasum` are required. Git Bash/MSYS on x86_64 Windows selects the MSVC archive. A missing archive exits nonzero and names its version and target.

Override the version or install directory through the script's environment:

```sh
curl -fsSL https://api.example.com/install.sh | TARE_VERSION=0.3.0 TARE_INSTALL_DIR="$HOME/.local/bin" sh
```

## Reports and CLI integration

Rust producers validate report/flag uploads against `contract/schema/UploadWeighing.json`, `UploadProofReport.json` and `UploadFlagInput.json`. `Weighing.json`, `ProofReport.json`, `Build.json`, `WeighingTotals.json` and `FlagInput.json` describe normalized public reads. The benchmark and compatibility schemas remain `BenchResults.json` and `Compat.json`. All schemas and OpenAPI are regenerated from Zod by the contract build.

Uploads require `application/json`, are capped at 1 MiB, and share a limit of 12 POSTs per client per minute per process. The validated request bytes are stored verbatim. The SHA-256 of those exact UTF-8 bytes is the returned hash. Repeated uploads reuse the same PostgreSQL `PinnedReport` row.

Content-hash reads have two representations. Explicit `Accept: application/json` selects the normalized public read schema: safe decimal strings become numeric arithmetic fields, while account lamports and rent epochs become strings. The typed client already sends this header. No header, `*/*`, or `application/octet-stream` returns the original bytes as `application/octet-stream`, preserving the CLI's hash check. Responses set `Vary: Accept`. A JSON view with an unsafe weight/count/slot fails with `REPORT_UNAVAILABLE` or `FLAG_INPUT_UNAVAILABLE`; its raw bytes remain available. Record pages embed the normalized view. Neither reading nor normalization rewrites stored content or its hash.

Without Pinata, uploaded bytes are written atomically under `.data/reports/` or `.data/flag-inputs/`; location is a relative API route. With a JWT, uploaded bytes are pinned and the **CID** is stored and returned as location. Reads use the configured gateway and verify the SHA-256 and document schema. Gateway-only configuration supports public reads without granting upload credentials. Reads are bounded to 1 MiB and 15 seconds.

`tare attest` and `tare flag` use these uploads, then sign their own chain instructions. An upload does not create an attestation. `tare check` reads by hash. `GET /v1/config` supplies `recordUrlTemplate`; replace `{record}` after attestation. No Pinata key belongs in the CLI.

## Configuration and Railway

`.env` contains only local setup and is ignored. `.env.example` lists every supported variable.

| Variable | Meaning |
| --- | --- |
| `MODE` | `fixture` by default; `chain` uses PostgreSQL and real services |
| `DATABASE_URL` | Required in chain mode; Railway `${{Postgres.DATABASE_URL}}` |
| `HOST`, `PORT` | Local loopback / 7810 defaults; Railway injected PORT and dual-stack host |
| `CORS_ORIGINS` | Exact browser origins; defaults to localhost and 127.0.0.1 on 7800 and 7801 |
| `CLUSTER`, `RPC_URL`, `SOLANA_RPC_URL` | Registry cluster and endpoint; RPC_URL takes precedence |
| `REGISTRY_COMMITMENT` | confirmed locally; finalized on other clusters |
| `INDEXER_POLL_MS`, `REGISTRY_RPC_RPS` | Local defaults 2000/50; other-cluster defaults 15000/5 |
| `REGISTRY_PROGRAM_ID`, `REGISTRY_IDL_PATH` | Registry identity and IDL path |
| `HELIUS_RPC_URL` | Required only when real sampling is requested |
| `WEIGH_SAMPLE_SIZE` | Usable-transaction target; default 50, maximum 100 |
| `WEIGH_CACHE_SECONDS` | Default 900; replaces the draft WEIGH_CACHE_AGE_SECONDS name |
| `PINATA_JWT`, `PINATA_GATEWAY` | Optional pinning key and public read gateway |
| `SITE_PUBLIC_URL` | Optional until the site's URL exists |
| `API_PUBLIC_URL` | Public API origin; sets the install line and default release base |
| `RELEASES_BASE_URL` | Optional external release directory; defaults to API_PUBLIC_URL/releases |
| `TOKEN_MINT`, `TOKEN_VENUE` | Blank until launch; API returns null |

Railway service root is `backend/`, build `npm run build`, API start `npm start`, worker start `npm run start:indexer`. Each role receives its own PORT. Internal PostgreSQL is contacted only at start/runtime. Persist `.data/` on a volume when using local content storage; Pinata needs no content volume because CID metadata is in PostgreSQL.

`.railwayignore` excludes the Rust toolchain, corpus source, Cargo files and local secrets, while including `bench/results.json`, `bench/compat.json`, only `registry/target/idl/tare_registry.json` from the registry tree, and packaged `releases/` when present. HTTP keep-alive is 65 seconds with a 66-second headers timeout. No deployment is performed by this round.
