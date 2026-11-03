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
