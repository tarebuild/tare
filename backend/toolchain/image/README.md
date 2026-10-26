# Pinned Linux image

Build from **backend/**:

```sh
docker build --platform linux/amd64 -f toolchain/image/Dockerfile -t tare-toolchain:0.7.0 .
docker run --rm tare-toolchain:0.7.0 --version
```

The Dockerfile-specific ignore file restricts this context to the toolchain and registry, excluding targets, caches and Git metadata. The image pins Rust 1.99.0 by manifest digest, Agave 4.4.0 and platform-tools 1.57 by published SHA-256, and Debian packages by snapshot. `locks.json` records those source pins. Both stock and Tare builds use the same platform-tools, SBF v0 and workspace profile.

The image contains the CLI, replacement crate sources, ordinary registry source, Python and JSON Schema validation. Node/Yarn and upstream Anchor CLIs are bootstrapped into the run volume by `bench/linux.sh`. `TARE_HOME=/opt/tare` identifies the implementation; `TARE_IN_CONTAINER=1` avoids nested containers. Registry source is available at `/opt/tare-registry` and copied into the run volume when needed.

Use a fresh volume containing only `bench/` scripts/manifest and `contract/schema/`. Do not mount a Windows corpus checkout into the Linux suite. The [benchmark README](../../bench/README.md#one-linux-entry-point) gives the complete two-command volume initialization and run. `sh bench/linux.sh <program IDs...>` fetches pristine Linux checkouts, builds tools in separate clones, runs the paired builds/proofs, records stock transactions, tests the Tare build and imports results. It is safe to rerun a subset.

No validator ports are published. The suite's internal validator can use 8899 and its dynamic port range without occupying host ports. The recorder uses internal 7812. The standalone registry deployment handoff uses the lead's explicitly supplied RPC, normally http://127.0.0.1:7840.

`TARE_IMAGE` selects an image for normal CLI builds; the default is the local `tare-toolchain:0.7.0` tag. Publishing and immutable digest selection belong to the lead. This Windows round does not build or run Docker, WSL or a validator, and does not claim cross-platform byte reproducibility before the Linux run.

The build context includes the ordinary registry sources and local fixture key, not built ELFs or a production program key. The lead supplies production credentials separately. The local fixture is explicitly public and disposable.

The benchmark validator in Agave 4.4 requires `io_uring`. Run the trusted test image with `--security-opt seccomp=unconfined` (and no published ports), as shown in `bench/lead-run.md`. `linux.sh` diagnoses the observed io_uring assertion and exits unsuccessfully when a setup, recording or test step fails; its final table reports recorded transactions and stock/Tare test outcomes.

The CLI uses native Rust 1.99.0 by default to extract the stock IDL, independent of a source workspace's toolchain override. This handles Whirlpools' upstream 1.86.0 pin, which predates the span API used by Anchor 0.32.1. An explicit `RUSTUP_TOOLCHAIN` overrides the native IDL selection. Both SBF builds retain platform-tools 1.57.

The CLI records the selected image reference in `buildEnvironment`. Checks across a host/image boundary explain the environment difference before compiling. The compiler keeps the stock SBF-v0 ELF form and dedicated Rent syscall; the native 4.4 loader checks are under `bench/history/round-five/loader-check/`.
