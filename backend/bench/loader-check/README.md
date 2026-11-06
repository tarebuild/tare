# Agave 4.4 loader admission

This checker uses Agave **4.4.0** and its **solana-sbpf 0.24.0** loader. The feature set matches the recorded test-validator: all SVM features enabled except `disable_sbpf_v0_v1_v2_deployment`; both v0 execution-disable and reenable features are active. The execution budget follows `raise_cpi_nesting_limit_to_8`.

Each ELF runs through the actual `deploy_program` deployment environment, test-validator preload verification, and `ProgramCacheEntry::load` plus x86-64 JIT compilation. Failure exits nonzero with the phase and loader error. `--stdin` reads newline-delimited paths for a whole-corpus audit.

```sh
cargo build --locked --release --manifest-path bench/loader-check/Cargo.toml --target-dir toolchain/target/native-versions
toolchain/target/native-versions/release/tare-loader-check bench/corpus/anchor/examples/tutorial/basic-0/target/deploy/basic_0.so bench/corpus/anchor/examples/tutorial/basic-0/target/tare/basic_0.so
```

Upstream disables its JIT module on Windows. `vendor/solana-sbpf` is the published 0.24.0 source with only that module's Windows cfg exclusion removed in `lib.rs`, `elf.rs`, `program.rs`, and `vm.rs`. Its existing Windows VirtualAlloc/VirtualProtect backend allows the unchanged JIT compiler to run. Generated native code is **never executed** by this checker. On Linux the source follows the upstream JIT path. The interpreter and the differential harness are unchanged.

The output also demonstrates visibility with the actual program cache: a valid upgradeable ELF deployed at slot zero is `DelayVisibility` in slot zero and `Loaded` in slot one. The loader returns `UnsupportedProgramId` for `DelayVisibility` as well as failed verification. The suite preflight waits for the ProgramData deployment slot plus one before running any user transaction.

The round-ten scan covers 429 existing primary ELFs: 214 stock and 215 Tare, including files belonging to explicit not-built benchmark rows. All pass all three admission phases. [Audit data](before.json) preserves per-file results. No program ELF is rebuilt or changed for this fix.
