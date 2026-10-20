# Tare toolchain

Tare compiles an unchanged Anchor workspace twice, optimizes the linked Tare ELF, and executes both artifacts in native LiteSVM. This workspace contains the `tare` CLI, the replacement `anchor-lang` implementation and macros, replacement `anchor-spl`, `tare-opt`, and `tare-prove`. [The measured matrix](../bench/compat.json) records exercised behavior; an implemented API is not a claim of complete Anchor conformance. [The full program report](../bench/round-six.md) and [unmeasured-input audit](../bench/gaps/unmeasured.json) distinguish measured coverage, stock failures, replacement limitations and missing corpus fixtures.

## Run

```powershell
cargo build --manifest-path toolchain/Cargo.toml -p tare
$env:TARE_BIN = (Resolve-Path toolchain/target/debug/tare.exe).Path
node bench/run.mjs basic-0 basic-1 basic-4
```

Inside a program workspace:

```sh
tare build --host
tare weigh --host
tare prove --host
```

Each accepts `--manifest-path path/to/program/Cargo.toml` and `--features feature1,feature2`. Build and replay use the same selected Cargo features, which are recorded with the corpus. Select a program manifest in a multi-program workspace. Without `--host`, the CLI starts `TARE_IMAGE` (default `tare-toolchain:<CLI version>`) and invokes the same code in the container. Publishing and selecting an immutable image reference belong to the operator. `TARE_HOME` identifies the directory containing the replacement crates; development defaults to this checkout.

Outputs in the user's workspace are `target/deploy/<program>.so` (stock), `target/idl/<program>.json` (stock IDL), `target/tare/<program>.so` (optimized Tare), the pre-optimization `target/tare/<program>.linked.so`, and `target/tare/{build,dependency-graph,optimization,corpus,execution,weigh,proof}.json`. `weigh.json` and `proof.json` follow the published contract schemas, with `optimization` and `recordedCorpus` facts retained and the contract 0.7.1 `replay` payload. Per-program `<program>.build.json`, `.corpus.json`, `.execution.json`, `.weigh.json` and `.proof.json` copies preserve each member of a multi-program workspace; the unprefixed paths retain the latest selected program. Unreached instruction weights are null with zero inputs and contribute nothing to totals.

On this Windows host, Cargo's Schannel network path sometimes fails; `CARGO_NET_OFFLINE=true` uses the populated cache. Some stock IDL builds compile native dev dependencies and require core Perl modules omitted by Git for Windows. `python toolchain/scripts/setup-windows-perl.py` downloads a checksum-pinned portable Strawberry Perl archive into ignored `toolchain/host/`; set `OPENSSL_SRC_PERL` to the printed executable for that invocation. This does not change global Cargo, TLS, Perl, or PATH configuration.

Some production sources contain custom serialization that the stock IDL compiler cannot describe. `--idl` accepts an explicit IDL, validates its schema, announces the input and pins it in the replay bundle. The supplied file's source verification is a separate corpus-preparation step. It is never chosen automatically. The Squads verifier checks the repository's SDK IDL against instruction arguments, ordered account fields, types and errors at the pinned source commit; its provenance is retained beside the IDL. This path does not claim that the stock compiler generated those bytes.
