# Handwritten Pinocchio references

The workspace keeps the no-account basic-0 baseline and adds independent handwritten equivalents of the user instructions in Anchor basic-1 and basic-4. Their validation order, account layout, custom errors, logs, and state changes follow the pinned Anchor source. They use Pinocchio 0.9.3, pinocchio-system 0.3.0, borrowed account bytes, fixed stack account descriptors, and no allocator.

These references implement the user instructions; they do not implement Anchor's built-in IDL administration. The diagnostic corpus is explicitly scoped accordingly. Full Tare proofs continue to include those built-ins. They are excluded from the product benchmark totals.

Build both with the same toolchain and architecture as stock/Tare:

```powershell
$env:CARGO_NET_OFFLINE='true'
cargo build-sbf --manifest-path bench/pinocchio/basic-1/Cargo.toml --arch v0
cargo build-sbf --manifest-path bench/pinocchio/basic-4/Cargo.toml --arch v0
cargo build --manifest-path toolchain/Cargo.toml -p tare-prove --features trace --example reference_compare
```

Run `reference_compare` with stock ELF, Tare ELF, handwritten ELF, stock IDL, original source, and output directory. It reuses `tare-prove`'s ordered corpus and executor, freezes all three ELF bytes with SHA-256 hashes, and records actual executed PCs, opcodes, syscall arguments, and metered units. `source-lock.json` identifies the handwritten sources used for this run. `../gaps/analyze.py` reconciles every remaining CU with executed instructions and runtime charges.

The basic-4 comparison adds explicit overflow and unauthorized-signer cases; both must preserve the exact stock error and panic logs. The build script records the original source path with the host's separators so panic messages agree on Windows and Linux.
