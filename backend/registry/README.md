# Tare registry

This is an ordinary Anchor 1.2.2 workspace. The stock Anchor source and IDL define its ABI; `tare build --host` substitutes the language implementation without rewriting this workspace. The checked local program ID is `6ri1ztKNUeHc2rwReL8p8thSTa1m48d6NRTzrwa9g85k`. The explicit `localnet-keypair.json` is a public, disposable local-validator fixture. Never use it on mainnet or to hold funds. Its matching copy under ignored `target/deploy/` is used by local build tooling. Production uses a separate private program keypair.

## Accounts and authority

- `Config`, PDA `[b"config"]`: `admin: Pubkey`, `attest_fee_lamports: u64`. It occupies 48 bytes including the discriminator.
- The fee recipient: `Config.fee_recipient`, the token's launch wallet. Every successful attestation transfers the current configured fee to it through the System Program, in the same instruction; `attest` refuses any other account (`FeeRecipientMismatch`). The admin changes the recipient and the fee with `set_config`.
- `Record`, PDA `[b"record", program, report_hash, attester]`: all build, source, corpus and report fields, followed by the attester and slot. Different reports or attesters have distinct records. An exact repeat fails account initialization and does not charge another attestation fee. Its maximum allocation is 1,128 bytes.
- `Flag`, PDA `[b"flag", record, input_hash]`: the existing record, input hash and location, flagger and slot. The same input can be flagged once per record; a missing or foreign record is rejected. Its maximum allocation is 628 bytes.

Only the registry's current upgrade authority can call `init_config(admin, attest_fee_lamports)`. The supplied `admin` may be a different public key; it does not need to sign initialization. Anchor verifies that the executable registry program refers to the supplied ProgramData account and that its upgrade authority signed. A fixed or permissionless first caller is not trusted. Once initialized, only `Config.admin` can call `set_config`, including transferring admin authority. An immutable deployment with no upgrade authority must initialize before relinquishing that authority.

`source_commit` is stored as 20 or 32 bytes and emitted as the same bytes in lowercase hex, matching the service's commit schema. The other length limits are 256 UTF-8 bytes for the source repository, 64 for the toolchain version, and 512 for report/input locations; strings must be nonempty. Counts, totals and fee values are u64. The registry stores an attester's claim; the CLI proves and pins its content before submitting it.

Every successful instruction emits an authenticated `emit_cpi!` event. Record and flag events contain every stored account field; Config events contain the stored fields; `RecordAttested` also names the fee recipient and the fee paid. The stock IDL is `target/idl/tare_registry.json`. `abi.json` records instruction/account discriminators, account order, seeds and allocation sizes for CLI integration.

## Build and native checks

From `backend/`:

```sh
cargo fetch --locked --manifest-path registry/Cargo.toml
tare build --host --manifest-path registry/programs/tare-registry/Cargo.toml
cargo run --locked --manifest-path registry/tests/Cargo.toml --target-dir toolchain/target
```

The native LiteSVM integration executes the exact stock and Tare ELFs over identical transactions. It checks account bytes, lamports, errors, return data, logs except runtime compute lines, and every self-CPI event payload. It additionally asserts upgrade/admin authorization, admin transfer, exact account-rent and configured-fee debits, fees accumulating in the launch wallet, a refused attest to any other recipient, distinct attestation addresses, failed duplicate atomicity, bad seeds and commits, and flagging only existing records. Its measured transcript is `target/native-tests.json`; this is a native SVM check, not an executed validator test. It also emits `target/native-replay.json`, a ReplayBundle with the exact 17 signed test transactions and empty generated corpus. `tare weigh --host --manifest-path registry/programs/tare-registry/Cargo.toml --replay registry/target/native-replay.json` and the corresponding `tare prove` replay those real program tests. The native replay preserves Clock and the upgradeable ProgramData header while each side loads its own ELF; it reproduces all 17 stock outcomes.

The fixture loads each ELF through the upgradeable loader, sets the ProgramData authority to the deterministic test keypair with seed `[12; 32]`, and sets slot 42. It passes seed 15 as the separate initial admin, then transfers admin to seed 13. Other test-only seeds are payer 11, attester 14, second attester 15 and flagger 16. Set `TARE_REGISTRY_SOURCE` to an alternate copied registry root when running the same harness in the pinned Linux image; it defaults to `registry` relative to the working directory. All inputs and output artifacts resolve beneath that root. No fixture key is a funded network wallet. ProgramData's ELF bytes naturally differ between builds and are not compared as application state; all transaction account changes are compared.

## Deployment and initialization

Install the registry's pinned Node client dependency (`npm ci --prefix registry`) and provide a funded deployer/upgrade-authority keypair. No validator, airdrop or mainnet transaction is started by these scripts. On a clean checkout, select a program keypair once and synchronize the ordinary source ID before building:

```sh
solana-keygen new --no-bip39-passphrase --outfile registry/target/deploy/tare_registry-keypair.json
node registry/scripts/set-id.mjs registry/target/deploy/tare_registry-keypair.json
tare build --host --manifest-path registry/programs/tare-registry/Cargo.toml
```

`set-id.mjs` is the single configuration command: it updates the ordinary `declare_id!` literal, `Anchor.toml`, and the ABI snapshot from the supplied program keypair. It does not edit compiled ELFs or IDLs. Build again after changing the ID. `tare build` itself never invokes this helper or edits source/manifests.

With a separately running validator/RPC, the lead runs:

```sh
node registry/scripts/deploy.mjs --url http://127.0.0.1:7840 --keypair /absolute/path/deployer.json --fee 1000
```

The URL defaults to `http://127.0.0.1:7840`; `--keypair` and `--fee` are always required. The fee shown is only an explicit example, not a program constant. The default program keypair is the public `localnet-keypair.json` fixture. Optional `--program-keypair FILE` and `--binary FILE` select different local artifacts; always pass a separate private program keypair for any nonlocal deployment. The default binary is the Tare build, `target/tare/tare_registry.so`.

The script verifies that the program keypair matches the stock IDL, skips deployment when on-chain ELF bytes already match, otherwise runs `solana program deploy --use-rpc` in WSL with a fixed buffer keypair and a compute-unit price, then initializes Config if absent with `--fee-recipient`. If Config exists with a different fee or recipient, the supplied current admin changes it; an already matching Config is left alone. It verifies Config afterward and writes `deployments/<cluster>.json`. This makes repeated runs idempotent without weakening on-chain initialization. The deployment script is syntax-checked here; network execution remains the lead's step.

## Event field names

- `ConfigInitialized` and `ConfigUpdated`: `admin`, `fee_recipient`, `attest_fee_lamports`.
- `RecordAttested`: `program`, `stock_build_hash`, `tare_build_hash`, `source_repository`, `source_commit`, `toolchain_version`, `corpus_hash`, `tests_count`, `generated_count`, `mainnet_count`, `gross`, `net`, `report_hash`, `report_location`, `attester`, `slot`.
- `RecordFlagged`: `record`, `input_hash`, `input_location`, `flagger`, `slot`.

`source_commit` is a bounded byte vector in the Record and lowercase hex text in the event. The other event field names and encodings match the stored account fields. Full stock IDL discriminators and types remain the source for the indexer.

The final native round-three artifacts measure 281,992 stock ELF bytes and 261,072 Tare bytes. The four instruction medians total 84,919 gross and 66,967 net CU; the 17-transaction money-path corpus matches. These totals are also in `bench/results.json`.
