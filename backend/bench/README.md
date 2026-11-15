# Tare benchmark

`manifest.json` names the pinned Anchor 0.31.1, 0.32.3 and 1.2.2 repository programs, five pinned production repositories, and the product's own `tare-registry`. `results.json` and `compat.json` contain observed results only. Intermediate runs and the previous round live under `history/`; live per-program, per-command terminal captures live under `transcripts/`.

## Native Windows run

```powershell
./bench/fetch.ps1
cargo build --release --manifest-path toolchain/Cargo.toml -p tare
node bench/run.mjs basic-0 basic-1 basic-4
```

Omit IDs to select the whole manifest. `--tare <path>` or `TARE_BIN` selects a built CLI. `TARE_BENCH_RESULTS` selects a separate result file; place parallel-worker files under `bench/history/<run>/`. `node bench/merge-results.mjs <files...>` merges explicit partitions, recomputes totals and validates schemas. Existing rows are retained during subset runs. `TARE_BENCH_MEASURE_ONLY=1` reruns weighing/proof inputs for previously measured rows without recompiling. It verifies both ELF hashes against that result file first, and preserves failed-build rows. Use it when only corpus generation changed; the normal runner always performs paired builds.

The runner builds the source in place with command-line dependency patches, weighs both ELFs and runs the differential proof. Cargo manifests and program source are never rewritten. Original Cargo.lock bytes are restored. Unimplemented constructs and failing upstream builds remain explicit `not_built` entries. A proof divergence is a failing run and stays in its report.

The own registry entry additionally runs its native money-path tests, then uses their exact signed transactions and account snapshots as a recorded corpus. Its test count is imported only when the tested ELF hash matches. Handwritten references in `pinocchio/` stay outside corpus totals.

The complete image, validator, registry and CLI handoff is in [lead-run.md](lead-run.md).

## One Linux entry point

Build the image from **backend/** (the context contains only the whitelisted toolchain and registry files):

```sh
docker build --platform linux/amd64 -f toolchain/image/Dockerfile -t tare-toolchain:0.6.0 .
```

Initialize a fresh Docker volume with only the benchmark scripts/manifest and contract schemas. Do not copy Windows corpus checkouts or build outputs:

```sh
docker volume create tare-bench

docker run --rm --entrypoint sh \
  -v "$PWD:/source:ro" -v tare-bench:/work tare-toolchain:0.6.0 -c '
    tar -C /source --exclude="bench/corpus" --exclude="bench/tools" \
      --exclude="bench/history" --exclude="bench/flows" --exclude="bench/artifacts" \
      --exclude="bench/gaps" --exclude="bench/transcripts" --exclude="*/target" \
      --exclude="bench/results.json" --exclude="bench/compat.json" \
      --exclude="bench/corpus-data" --exclude="bench/upstream-tests.json" \
      --exclude="bench/stock-tests.json" --exclude="bench/linux-summary.json" \
      --exclude="bench/record-attachments.json" \
      -cf - bench contract/schema | tar -C /work -xf -
  '

docker run --rm --entrypoint sh --security-opt seccomp=unconfined \
  -e TARE_ISOLATED_TEST_NETWORK=1 -v tare-bench:/work -w /work \
  tare-toolchain:0.6.0 sh bench/linux.sh basic-0 basic-1 basic-4 tare-registry
```

Replace the IDs with a wanted subset; omit them for every manifest entry. No ports are published. Anchor's unchanged suites can use their internal 8899 validator, and the recording proxy uses internal 7812. No validator runs on this Windows host. The trusted test container needs `io_uring`; the default Docker seccomp profile blocks it. This invocation relaxes seccomp only for that container, publishes no ports, and mounts no wallet or secrets. `linux.sh` probes the syscall and stops validator stages with an actionable diagnostic when the host still denies it. See [Linux run notes](linux-notes.md).

`linux.sh` bootstraps pinned Node/Yarn before fetch, fetches pristine checkouts inside the container, installs the matching Anchor CLI/client tools in **separate** clones under `tools/source/`, builds/weighs/proves, records the stock suite, runs the suite against Tare, then reruns the benchmark to import recordings and test counts. The first invocation archives any imported result files; subsequent subset runs retain results from that Linux volume. Its outputs are `results.json`, `compat.json`, `stock-tests.json`, `upstream-tests.json`, `linux-summary.json`, `corpus-data/` and `transcripts/`, including a retained setup transcript for each selected Anchor version.

Git ownership is configured per invoked checkout, not globally. Tool installation may change its own clone's Cargo.lock; it cannot change the corpus checkout's lockfile. Source checkouts use `core.autocrlf=false`. Every shell script is UTF-8 without BOM and uses LF.

The pipeline returns nonzero for retained build/proof/suite failures but still attempts later concerns and writes their evidence. A failed tools installation is recorded per Anchor version; other versions and build/proof stages continue. A failed recording workspace does not prevent later selected workspaces from being attempted. The end-of-run table lists recorded transactions, stock tests and Tare tests for every selected program; missing, zero-test and failed evidence yields a nonzero exit. Unsupported programs in a multi-program workspace can prevent that workspace's upstream suite from starting; those failures remain visible.

## Evidence and replay

`record-tests.sh` and `run-upstream-tests.sh` remain separate concerns and accept the same IDs. [recording/README.md](recording/README.md) defines capture/replay semantics and validator reproduction counts. A native replay match does not claim to reproduce the validator's outcome; those counts are reported separately.

Instruction weights are upper observed medians of successful attributed calls. Null weights mean no successful input reached the instruction and contribute nothing to totals. Whole-program state, lamports, logs, return data and errors are compared; only exact runtime compute lines are normalized. `gaps/` contains per-section/symbol size and CU evidence, including programs that remain larger than stock.

The current native CLI rehearsal records successful attest, a matching cross-folder rebuild/check, a positive flag with the explicit rehearsal defect, and refusal of the same input without the defect. Exact evidence is in `rehearsal/chain-verification.json` and `transcripts/round-four-native-*.txt`. The lead verified Windows Docker execution and real validator/API flows in round four; the earlier round-three path-dependent hashes remain archived.

The film feature is `v0_32_3-tests-idl-programs-new-idl` (13 measured instructions); its command transcripts and a Tare disassembly excerpt live in `transcripts/`.

The benchmark feed omits the bulky replay payload and validates directly against the shared BenchResults schema. Each full `artifacts/<id>/proof.json` and workspace proof retains the complete replay bundle. No measured weights, comparison counts, build hashes, optimization facts or recorded-corpus facts are omitted.

Contract 0.7.1 accepts `replay` and pins the full reproducible corpus. Exact wide-integer encoding and the optional record-cluster addition are documented in [../toolchain/contract-additions.md](../toolchain/contract-additions.md). Local diagnostic fields `optimization` and `recordedCorpus` are retained in benchmark reports.

## Generated state scenarios

The generator retains its whole-program sequences and adds independent scenarios for validation-only handlers and fresh initializers in larger programs. This prevents unrelated account families that reuse field names from overwriting one another's test state. Small programs with multiple possible endings receive additional ordered sequences that run each ending first after initialization; later attempts still exercise burned or closed account failures. Typed accounts with a reused field name are separated by their actual account type.

SPL authority and mint relationships come from the source's CPI account structs and account constraints. Signable mints, token accounts and associated token accounts are initialized through real setup transactions in the corpus. `fixtureProvenance` identifies those accounts and the remaining program-derived snapshots; setup transactions are compared but excluded from target instruction medians. Fixed literal PDA seeds used by signed CPIs are resolved against the current program. A unique compatible PDA definition can be shared across instructions or imported from the account owner's stock IDL. Unknown, ambiguous, argument-dependent external seeds and unresolved account-data seeds remain explicit gaps. Zero-constrained accounts start as genuinely zero-filled allocations. Constant-index writes through untyped account data determine the minimum fixture size and required writable meta.

`python bench/gaps/audit-unmeasured.py bench/results.json bench/gaps/unmeasured.json` writes a per-instruction explanation for every missing successful measurement, including source path and line, the observed baseline error when available, and direct source-level unconditional errors. Programs whose stock IDL declares no user instructions are listed separately. The audit never removes a failing input from a proof.

The [final artifact audit](gaps/final-audit.json) checks canonical rows, schemas, archived ELF/proof hashes and pinned checkout cleanliness. [Generator provenance](gaps/generator-provenance.json) distinguishes real SPL setup transactions, ordered initializer successes and directly seeded state. The [fixed-corpus performance aggregate](gaps/round-three/performance/summary-final.json) compares all available prior Tare baselines with the exact canonical candidate ELFs; the [performance notes](gaps/round-three/performance/README.md) retain regressions explicitly. [Remaining state families](gaps/coverage/remaining-state-families.json) explains the named generator gaps without excluding their failing inputs.

[Remaining measured gaps](gaps/remaining.md) summarizes unresolved successful-input coverage, fixed-corpus compute regressions, size exceptions, and explicit typed snapshots.

## Attach an attested build

After `tare attest` prints the real Record address, attach it to its matching benchmark row:

```sh
node bench/attach-record.mjs --program basic-1 --record RECORD_ADDRESS --cluster localnet --url http://127.0.0.1:7840
```

`--results FILE` selects another result file. `--url` verifies the Record owner and build/source/corpus/totals against the row; omit it only when importing an address already verified by the lead. `--registry-program ADDRESS` selects a registry other than `registry/abi.json`. The helper records `record` and the non-secret `recordCluster` label (`--record-url URL` also stores the page URL) and refuses a concurrent result-file change. `recordCluster` is the optional contract addition documented in [record-attachment.md](record-attachment.md). It never sends a transaction.

## Round four flow checks

`bench/repro/live-summary.json` records two-folder and fresh-cache native reproducibility checks. `bench/rehearsal/README.md` gives the positive local flag rehearsal; its `dispatch-log` defect is explicit, disabled by default and never eligible for attestation. Native RPC-adapter transcripts are labeled `round-four-native-*`; they are not validator runs.

Production source pins and source-verified IDL preparation live in `bench/production/`. The normal generated-corpus runner accepts manifest `cargoFeatures`, optional `suppliedIdl`, and directed `productionCorpus` profiles. A production address identifies the intended program, not an unverified claim that these local ELFs equal a current deployment.

After a real attestation, attach its address with:

```sh
node bench/attach-record.mjs --program v0_32_3-tests-idl-programs-new-idl --record RECORD --cluster localnet --url http://127.0.0.1:7840
```

The optional `recordCluster` and `recordUrl` fields are documented in `toolchain/contract-additions.md` for the services schema. Subsequent measurements preserve the attachment only if both build hashes and the corpus hash remain identical.
