# Stock-suite transaction recordings

`record-tests.sh` runs the stock upstream suite unchanged. A process-local Node transport hook redirects only HTTP requests addressed to the local validator from port 8899 to the recording proxy on 7812. The proxy forwards to the original validator, serializes transaction submissions, records their signed bytes, and forwards the original JSON-RPC response. No source, manifest, IDL, or transaction bytes are rewritten by the recorder. Anchor may temporarily rewrite generated IDL addresses for tests; the runner restores the original IDL afterward.

Run inside the isolated Linux container described in `bench/README.md`, with no published ports:

```sh
sh bench/linux.sh basic-0 basic-1 basic-4
```

Use the fresh-volume initialization commands in the benchmark README first. The entry point fetches source and installs tools before recording; it does not receive Windows checkouts.

Stock ELFs and stock IDLs must already exist from the paired build. The test dependencies come from the pinned Anchor repository. The recorder creates an ephemeral local wallet and uses Anchor's stock validator genesis loading and fixture lifecycle. A new validator genesis starts a new recorded sequence. The runner verifies a unique recorder session on `/_tare/health`, so a stale process on port 7812 cannot be mistaken for this run. It records stock Mocha results and capture failures in `stock-tests.json`; each Anchor suite and its validator descendants belong to a dedicated process group that the runner terminates on completion or interruption. The wrapper keeps the SBPF-v0 deployment feature setting consistent with this round's build notes. It starts no validator or Docker process on this Windows host.

Output is `bench/corpus-data/<program-id>/tests.json`, with one file per corpus manifest ID. Each file contains the target address, exact stock ELF hash, source commit, captured external executable ELFs, and ordered sequences. Each transaction retains `transactionBase64`, signature, stage, pre-state and post-state account snapshots, snapshot slot, execution slot, validator logs/error/return data/compute units, and whether the execution-bank Clock was captured exactly. Address lookup table accounts and loaded addresses are included. Loader-v3 program accounts resolve to their captured ProgramData ELF. Other non-ELF executable formats stop capture explicitly. Account integers above JavaScript's safe range are preserved as decimal strings, including rent epochs.

The Clock account is observed through the validator's account subscription and matched by execution slot. A missing execution-bank Clock remains an explicit incomplete-reproduction result. It is not estimated from wall time or from another slot. Preflight-rejected sends are recorded too; replay simulates those inputs without committing transaction fees. Landed failures replay as transactions. External account changes between frames, including airdrops and clock progression, are applied identically to both replay states. Each build's program-produced state persists between frames.

`run_recorded(stock, tare, corpus)` reports compiler equivalence and stock-versus-validator reproduction separately. Reproduction compares account results, logs, RPC-visible return data, errors, metered units when supplied, and exact-clock coverage; its denominator includes failures. An incomplete capture is refused. Capture and replay stock ELF hashes are both reported, with a `sameStockBinary` flag. Cross-host source paths can change stock ELF bytes and logs; the native replay retains and reports those reproduction differences. The CLI validates the recorded source commit against its build record before importing. A compiler match does not claim that the validator outcome was reproduced: those reproduction failures remain visible in their separate counts.

Standalone replay after copying the recording back to this host:

```powershell
cargo run --manifest-path toolchain/Cargo.toml -p tare-prove --example replay_recording -- bench/corpus-data/basic-1/tests.json bench/corpus/anchor/examples/tutorial/basic-1/target/deploy/basic_1.so bench/corpus/anchor/examples/tutorial/basic-1/target/tare/basic_1.so bench/corpus-data/basic-1/replay.json
```

Exit 1 means stock/Tare diverged; exit 2 means the native stock replay did not reproduce the validator. This round checks the importer natively with three genuinely executed and signed SVM transactions (including one failure) (`recording_smoke`), including preserved fee-payer state. That is an importer check, not an upstream validator recording; no upstream test count is fabricated.

`merge_recorded(&mut generated_report, recorded_run)` appends every recorded comparison and returns component corpus hashes plus the independent validator-reproduction report. It recomputes instruction medians from the actual successful sample values, retaining test and generated input counts separately. Recorded instruction samples require exactly one transaction instruction targeting the program and one observed top-level runtime compute line; multi-instruction transactions remain proof inputs without attributed instruction cost. Full instruction bytes are retained for IDL discriminator matching. The combined corpus hash commits to the exact portable replay bundle, including its generated seed, recorded inputs, snapshots and external program bytes.

The recorder resolves web3, bs58 and toml from the installed, linked Anchor client first, then from its pinned peer packages under `bench/tools/recorder`. Setup installs and verifies this resolution before the suite runs. The tutorial workspace need not declare the recorder's transitive dependencies.

The installer builds the pinned Anchor 0.31.1, 0.32.3, and 1.2.2 CLIs separately under `bench/tools/anchor-<version>/`. The runners verify each selected entry's own checkout commit and choose its matching CLI and TypeScript client. Version-specific Yarn link folders keep the 0.32 and 1.x packages from replacing one another. Anchor 1.2.2 uses `test --validator legacy` to select the Agave validator explicitly. The 1.2.2 CLI is compiled with its `solana-v4` feature to match the installed Agave cohort.

For a cross-version recorded subset after the corresponding paired builds:

```sh
sh bench/setup-upstream-tests.sh 0.31.1 0.32.3 1.2.2
sh bench/record-tests.sh basic-0 basic-1 basic-4 \
  v0_32_3-examples-tutorial-basic-0-programs-basic-0 \
  v1_2_2-examples-tutorial-basic-0-programs-basic-0
```

`setup-upstream-tests.sh` without versions installs all three manifest cohorts. `record-tests.sh` and `run-upstream-tests.sh` without IDs select the entire manifest and group entries by checkout plus workspace; choose a subset when unsupported programs in a larger workspace have no build outputs. Each output file retains its unique manifest ID, including version prefixes.

Each transaction contains an `input` projection in the services' `ReplayInput` shape: `kind`, `label`, signed `transactionBase64`, non-null pre-state `accounts` (address, owner, lamports, executable, dataBase64), and pre-state `slot`. The separate rich snapshots retain absent-account markers, rent epochs, execution slot, and validator evidence. The importer checks the projection against those snapshots before replay. Lamport values beyond JavaScript's exact integer range remain decimal strings in both representations; contract 0.7.1 accepts both exact decimal strings and safe JSON numbers.


The validator wrapper records only its own child process identities. Each suite waits for its RPC ports to be free; cleanup stops only those recorded processes and waits again. A preload records the actual validator version, feature activation accounts, expected program IDs, loader owners, and ELF headers/bytes before the client sends transactions. These diagnostics distinguish program preloading and runtime feature failures from compiler behavior. Native synthetic checks do not replace the lead's real Linux suite run.
