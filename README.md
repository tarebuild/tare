# Tare

Weigh less.

Tare is a drop-in build tool for Anchor programs. `tare build` reads an existing Anchor workspace without changing a line and produces the same program at a fraction of the compute units and binary size. `tare prove` runs the stock build and the Tare build side by side to show they behave the same, and `tare attest` writes the result to an on-chain registry where anyone can rerun it.

Site: [tare.cx](https://tare.cx). API: [api.tare.cx](https://api.tare.cx). Registry program on mainnet: `7viqDjx5k8ffnfnZiBv9dMhxzVhcGSYLMb5bKdDP1XtP`.

## Install

```sh
curl -fsSL https://api.tare.cx/install.sh | sh
```

Prebuilt binaries cover macOS and glibc Linux on x86_64 and aarch64, and Windows on x86_64. To build from source:

```sh
cargo build --release --manifest-path backend/toolchain/Cargo.toml -p tare
```

## Commands

| Command | What it does | Writes |
|---|---|---|
| `tare build` | Builds the same workspace from the same source | `target/tare/<program>.so` |
| `tare weigh` | Gross, tare and net compute units for every instruction | `target/tare/weigh.json` |
| `tare prove` | Runs the stock build and the Tare build on the same transactions and compares them | `target/tare/proof.json` |
| `tare attest` | Records the build, its weighing and its proof in the registry | a record on chain |
| `tare check <record>` | Rebuilds and reruns a record from the record alone | |
| `tare flag <record> --input <file>` | Files an input on which the two builds differ | |

Every instruction gets three weights. Gross is what it burns built with stock Anchor, net is what it burns built with Tare, and tare is the difference between them.

After every transaction the proof compares account bytes, lamports, logs, return data and errors. It replays three sets of transactions: the program's own tests, instruction sequences generated from the IDL, and transactions sampled from mainnet for the program's address. Account layouts, discriminators, the IDL, error codes, events and logs stay exactly as stock Anchor makes them. Anchor 0.31, 0.32 and 1.x are supported.

## Benchmark

`backend/bench/` builds, weighs and proves pinned programs from the Anchor repository at 0.31.1, 0.32.3 and 1.2.2, five production repositories (Squads v4, Whirlpools and Marinade among them) and the registry program itself. On toolchain 0.7.0, 189 of 238 programs are measured and proven over 257,892 transactions: 6,295,676 gross compute units come out at 4,459,739 net, and 38,786,032 bytes of binary at 30,970,216. The other 49 are listed in `compat.json` with the construct that stops them. Every row is in `backend/bench/results.json`.

## Repository

| Path | What it holds |
|---|---|
| `backend/toolchain/` | The Rust workspace: the `tare` CLI, `tare-lang` and its macros, `tare-opt` (the bytecode optimizer), `tare-prove` (the differential replay) and `tare-spl` |
| `backend/registry/` | The `tare_registry` Anchor program, its tests and deploy scripts |
| `backend/src/` | The API, the registry indexer, the weigh service and the crank, on Node, Hono and Prisma over PostgreSQL |
| `backend/contract/` | `@tare/contract`: Zod schemas, a typed client, OpenAPI and JSON Schema shared by the API and the site |
| `backend/bench/` | The benchmark harness, its manifest and its results |
| `backend/install/` | The installer the API serves at `/install.sh` |
| `web/` | The site at tare.cx, on Next.js |
| `brand/logo/` | The mark, wordmark, lockups and icons |

## Run locally

```sh
cd backend
npm ci
npm run build
npm run fixture
```

The fixture server answers on `127.0.0.1:7810` with recorded data and needs no database or RPC. `backend/README.md` covers the chain mode, the indexer, every route and every variable. The site runs from `web/` with `npm ci` and `npm run dev`.

## License

Apache-2.0. See `backend/toolchain/LICENSE`.
