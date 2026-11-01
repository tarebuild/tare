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
