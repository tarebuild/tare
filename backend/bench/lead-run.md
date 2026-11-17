# Lead verification commands

Run from `backend/` on Linux. This agent has not executed Docker or a validator. Native flow checks use a local LiteSVM RPC/report adapter; the lead's earlier validator/API evidence remains separate.

## Image and upstream corpus

```sh
docker build --platform linux/amd64 -f toolchain/image/Dockerfile -t tare-toolchain:0.6.0 .
docker volume create tare-bench

docker run --rm --entrypoint sh -v "$PWD:/source:ro" -v tare-bench:/work tare-toolchain:0.6.0 -c '
  tar -C /source --exclude="bench/corpus" --exclude="bench/tools" \
    --exclude="bench/history" --exclude="bench/flows" --exclude="bench/artifacts" \
    --exclude="bench/gaps" --exclude="bench/transcripts" --exclude="*/target" \
    --exclude="bench/results.json" --exclude="bench/compat.json" \
    --exclude="bench/corpus-data" --exclude="bench/upstream-tests.json" \
    --exclude="bench/stock-tests.json" --exclude="bench/linux-summary.json" --exclude="bench/record-attachments.json" \
    -cf - bench contract/schema | tar -C /work -xf -
'

docker run --rm --entrypoint sh --security-opt seccomp=unconfined -e TARE_ISOLATED_TEST_NETWORK=1 \
  -v tare-bench:/work -w /work tare-toolchain:0.6.0 \
  bench/linux.sh basic-0 basic-1 basic-4 tare-registry
```

Remove the final IDs to run every manifest entry. No ports are published. Native stock IDL extraction uses Rust 1.99.0 by default in the corrected CLI; an explicit `RUSTUP_TOOLCHAIN` overrides it. Both SBF builds still use platform-tools 1.57. The entry point fetches Linux checkouts itself, links the image's toolchain for the production corpus helper, builds tools in separate clones, records stock suites, runs Tare suites, and writes the resulting proofs, test counts, recordings and transcripts. Repeating with a subset retains the other rows in this Linux volume. Use a fresh volume to start another independent run. The trusted container needs `io_uring`; the seccomp option is required by the observed Agave startup failure. Keep wallet/secrets mounts out of this test container. See [Linux run notes](linux-notes.md).

## Registry deployment

Start a separate local validator yourself, using the SBF-v0 feature setting described in the deployment notes:

```sh
solana-test-validator --reset --ledger /tmp/tare-registry-ledger --rpc-port 7840 \
  --deactivate-feature B8JJXCy5amZyWG9r7EnUYLwzXSXTxG7GZ1qZ1qggo83g
```

In another terminal, supply a local funded deployer and choose the fee explicitly:

```sh
export TARE_RPC_URL=http://127.0.0.1:7840
export TARE_KEYPAIR=/absolute/path/local-deployer.json
export TARE_REGISTRY_PROGRAM_ID=6ri1ztKNUeHc2rwReL8p8thSTa1m48d6NRTzrwa9g85k
solana airdrop 20 --url "$TARE_RPC_URL" --keypair "$TARE_KEYPAIR"
cargo build --release --manifest-path toolchain/Cargo.toml -p tare
export PATH="$PWD/toolchain/target/release:$PATH"
tare build --host --manifest-path registry/programs/tare-registry/Cargo.toml
npm ci --prefix registry
node registry/scripts/deploy.mjs --url "$TARE_RPC_URL" --keypair "$TARE_KEYPAIR" --fee 1000
```

The fee is an example argument, not a constant in the program. Deployment defaults to the explicitly public disposable localnet program key; use a separately generated private program key for any real deployment. `node registry/scripts/set-id.mjs PROGRAM_KEYPAIR` is the one ID synchronization command and requires rebuilding afterward.

## API and end-to-end CLI

Use services contract 0.7.1 or later, which accepts `replay`, `recordCluster` and `recordUrl`. Start the API with its registry/RPC settings and expose its URL through `TARE_API_URL`. Exact u64 fields accept decimal strings as described in `toolchain/contract-additions.md`.

From a clean, committed source checkout:

```sh
export TARE_API_URL=http://127.0.0.1:7810
tare build
tare prove
tare attest --url "$TARE_RPC_URL" --keypair "$TARE_KEYPAIR"
# From backend/, attach the printed address after the attestation succeeds:
node bench/attach-record.mjs --program basic-1 --record RECORD_ADDRESS --cluster localnet --url "$TARE_RPC_URL"
tare check RECORD_ADDRESS --url "$TARE_RPC_URL"
tare flag RECORD_ADDRESS --input diverging-input.json --url "$TARE_RPC_URL" --keypair "$TARE_KEYPAIR"
```

Use the same pinned image for build and check. A flag is submitted only if the supplied input actually diverges. For a positive local rehearsal, first attest normally, then set `TARE_REHEARSAL_DEFECT=dispatch-log` and use `bench/rehearsal/new-idl-input.json`; see [the exact rehearsal steps](rehearsal/README.md). Capture these actual validator commands as attest/check/flag transcripts. A nondivergent input must be refused.

A bounded mainnet replay needs only a read-only RPC URL, not a keypair:

```sh
TARE_MAINNET_RPC_URL=https://api.mainnet-beta.solana.com tare prove --program PROGRAM_ADDRESS --samples 3
```

Use an ELF built from that program's source. The report pins the exact sampled signatures, signed transactions and current account snapshots; subsequent check replays that bundle without resampling.

## Round four native flow and validator repetition

Native LiteSVM verified normal attest, cross-folder check, positive defect flag, and normal non-divergence refusal. The native record/flag are test-adapter state, not addresses on your validator. For the real local validator use a new ordinary attestation produced by this compiler; the old round-three record commits to different pre-normalization artifact hashes.

```powershell
$env:TARE_API_URL = 'http://127.0.0.1:7810'
$env:TARE_RPC_URL = 'http://127.0.0.1:7840'
$env:TARE_KEYPAIR = 'C:/path/to/local-wallet.json'
$env:TARE_REGISTRY_PROGRAM_ID = '6ri1ztKNUeHc2rwReL8p8thSTa1m48d6NRTzrwa9g85k'
# In a clean pinned Anchor checkout's tests/idl workspace:
tare build --host --manifest-path programs/new-idl/Cargo.toml
tare weigh --host --manifest-path programs/new-idl/Cargo.toml
tare prove --host --manifest-path programs/new-idl/Cargo.toml
tare attest --host --manifest-path programs/new-idl/Cargo.toml
tare check RECORD --host --directory C:/path/to/another-clean-checkout
$env:TARE_REHEARSAL_DEFECT = 'dispatch-log'
tare flag RECORD --host --directory C:/path/to/another-clean-checkout --input C:/path/to/backend/bench/rehearsal/new-idl-input.json
Remove-Item Env:TARE_REHEARSAL_DEFECT
# Same input now refuses before upload/send (exit 1):
tare flag RECORD --host --directory C:/path/to/another-clean-checkout --input C:/path/to/backend/bench/rehearsal/new-idl-input.json
```

Capture default output for attest/check/flag in `bench/transcripts/`. The server must accept decimal strings for wide u64 fields. `recordCluster` remains an optional schema addition for benchmark attachments; details are in `toolchain/contract-additions.md`.

## Production programs

Use the same frozen image/runtime as the normal corpus. The CLI defaults to Rust 1.99.0 for native stock IDL extraction; Whirlpools pins Rust 1.86.0 for its own workspace, which cannot compile Anchor's newer IDL span API. Both SBF builds still use the same installed platform-tools.

```powershell
$env:TARE_BIN = (Resolve-Path toolchain/target/release/tare.exe).Path
node bench/production/prepare.mjs production-squads-v4
node bench/run.mjs production-squads-v4 production-whirlpools production-marinade
$env:TARE_MAINNET_RPC_URL = 'https://YOUR_READ_ONLY_RPC'
& $env:TARE_BIN prove --host --manifest-path bench/corpus/squads-v4/programs/squads_multisig_program/Cargo.toml --idl bench/production/controls/squads-v4/verified-idl.json --replay bench/corpus/squads-v4/target/tare/production-replay.json --program SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf --samples 3
& $env:TARE_BIN prove --host --manifest-path bench/corpus/whirlpools/programs/whirlpool/Cargo.toml --features mainnet --replay bench/corpus/whirlpools/target/tare/production-replay.json --program whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc --samples 3
```

The RPC URL needs read access only. Current-state samples retain signatures, signed transactions, account snapshots and external ELFs in the proof's replay bundle; they do not claim to reproduce the historical bank. Keep RPC credentials in the environment, not captured command transcripts. An attestation made before the 0.4.0 build is a different build: attest the new ordinary proof before the check/flag rehearsal.

## Round five Linux diagnostics

Recorder setup installs its three pinned peer packages into `bench/tools/recorder`, independently of the untouched tutorial manifests. It still resolves a linked Anchor client's own packages first. Before each suite, the runner checks that RPC ports are free. The validator wrapper records its own PID and Linux process start time; cleanup signals only the owned process group and those matching validator identities. It never kills a process merely because it owns a port.

A preload runs after Anchor starts the validator and before test code. It first records `getVersion`, the current slot, and the actual feature-account activation states for `disable_sbpf_v0_execution`, `reenable_sbpf_v0_execution` and the existing `disable_sbpf_v0_v1_v2_deployment` override. It then logs each configured program's actual loader owner, executable bit, ELF machine and flags, and checks the loaded bytes against the intended build. A missing/wrong preload fails the suite. This provides evidence for `Unsupported program id` without treating a loader error as a compiler equivalence result. The generated corpus gate is separate: `node bench/gate.mjs` compares current Tare against current stock, requires matched proofs and full manifest result coverage, and rejects larger ELFs or higher reached-instruction medians. Production instruction medians must be strictly lower. Optional `--against-round-three` reporting is informational.

## Round five checks

The measured-corpus gate is explicit:

```sh
node bench/gate.mjs --summary
```

Before validator suites, run the loader/JIT control against freshly built artifacts:

```sh
cargo run --release --manifest-path bench/loader-check/Cargo.toml -- bench/artifacts/basic-0/stock.so bench/artifacts/basic-0/tare.so bench/artifacts/basic-1/stock.so bench/artifacts/basic-1/tare.so
```

A fresh image test volume still starts with the volume-copy command above and runs:

```sh
docker run --rm --entrypoint sh --security-opt seccomp=unconfined -e TARE_ISOLATED_TEST_NETWORK=1 -v tare-bench:/work -w /work tare-toolchain:0.6.0 bench/linux.sh basic-0 basic-1 basic-4 tare-registry
```

The final table must show matching stock/Tare test counts and a zero command exit. The recorder now installs its own pinned peer dependencies, and the suite records installed program identities before executing tests. Cleanup tracks only validators started by this run and waits for their ports to close. Capture the preflight lines in `*-recorder.txt` / `*-upstream-tests.txt` and validator logs if a program still fails admission; the native header/loader tests are not a claim that the Linux validator suite passed.

## Round ten visibility check

Rebuild the 0.7.0 image to include the exact Agave 4.4.0 admission helper and the suite's ProgramData visibility wait, then rerun the four-program Linux subset. The preflight must print `program availability: effectiveSlot=1 observedSlot=1` (or a later observed slot) before the first test. No program ELF or attestation hash changes for this fix. The local native audit admits all 429 existing ELFs; the Linux suite confirms the readiness change on the validator.

```sh
docker build --platform linux/amd64 -f toolchain/image/Dockerfile -t tare-toolchain:0.7.0 .
# Refresh bench/recording and bench/loader-check in the existing test volume using the volume-copy command above.
docker run --rm --entrypoint sh --security-opt seccomp=unconfined -e TARE_ISOLATED_TEST_NETWORK=1 -v tare-bench:/work -w /work tare-toolchain:0.7.0 bench/linux.sh basic-0 basic-1 basic-4 tare-registry
```
