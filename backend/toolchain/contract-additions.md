# Contract boundary

The installed services contract is 0.7.1. Its accepted `replay` field contains the complete compressed corpus and remains in report uploads; it is no longer a pending extension.

Every unsigned 64-bit integer is emitted as a JSON number through 9,007,199,254,740,991 and an exact decimal string above that value. Readers accept either form. This applies recursively to uploaded proof/flag documents and to the decoded replay bundle: account `lamports` and `rentEpoch`; transaction `slot` and `preSlot`; validator `computeUnits`; generated `seed`; instruction weights, totals, sizes, rent, comparison counts, corpus counts and trace counters when they exceed the safe range. Hashes cover the exact canonical upload bytes after this conversion. No float round trip is used.

The benchmark row retains `record: string | null`, with optional `recordCluster: string` and `recordUrl: string` when a record is attached. These are accepted by contract 0.7.1. `bench/attach-record.mjs` writes the address and cluster together; an optional RPC check verifies program/build/corpus identity. A remeasurement retains them only when both build hashes and the corpus hash are unchanged.

A deliberately defective local rehearsal flag uses the ordinary flag-input shape. Its label and actual extra log line identify the rehearsal. The local build metadata records `rehearsalDefect: "dispatch-log"`. No extra API field is required to record the rehearsal.

`build.json` is local compiler metadata, outside the service contract. It carries `reproducibility`, `features`, and optional `rehearsalDefect`. A rehearsal build cannot be attested. The CLI verifies a record's normal source and build hashes before producing a separate defective build for a flag rehearsal.

The replay payload is gzip plus base64 with a SHA-256 of the uncompressed JSON. It stores a relative manifest path, Cargo features, the generated corpus, external program ELFs, recorded test inputs and current-state mainnet snapshots. `check` reuses those exact inputs and never resamples RPC. Decoding is bounded to 256 MiB; API upload remains bounded to its current 1 MiB limit.

`ReplayBundle.suppliedIdl` pins an explicitly supplied IDL. The production Squads entry uses it for its source-verified SDK IDL because the stock IDL compiler cannot describe its custom serialization. Ordinary builds omit it. This is inside the opaque compressed `replay` field and requires no top-level service-schema addition.

## Round five build environment

`build.json` and weighing/proof reports add `buildEnvironment` with exactly one of these shapes:

```json
{"kind":"host","targetTriple":"x86_64-pc-windows-msvc"}
```

```json
{"kind":"image","image":"tare-toolchain:0.5.0"}
```

The image value is the selected `TARE_IMAGE` tag or digest. `check` compares this field before rebuilding and names both environments in one refusal line. Reports predating the field retain hash-based checking; their environment is not guessed.

`accountAbi` records `{"mode":"raw"|"sdk","selection":"..."}` in local build metadata and reports. Both modes compile Tare's own Anchor implementation. Rust type checking selects SDK account descriptors only when a dependency boundary requires that type; ordinary code uses raw descriptors.

`recordCluster` is an optional string on a measured benchmark row, paired with its existing `record` address string. `recordUrl` is an optional absolute HTTP(S) page URL for that record. The cluster value is a non-secret label of 1 to 64 ASCII letters, digits, dots, underscores or hyphens. No schema files outside the toolchain are changed here.

Contract 0.7.1 already defines `BuildEnvironment` and `AccountAbi` with the shapes above. `Optimization` accepts the added `indirectCallSites` array (`pc`, numeric `targets`, `resolved`, optional `reason`), plus `unresolvedIndirectCalls` and `retainedForIndirectCalls` counts.

Raw ABI selection is a compiler decision, not a user-facing mode. A source observation that requires the SDK layout selects that representation; otherwise successful type checking retains the compact representation. The local `account-abi-diagnostic.txt` preserves a typed boundary requiring SDK when encountered. No failed source construct is converted to stock Anchor.

## Round six

No additional service fields are required. `accountAbi.mode` remains `raw` or `sdk`; `selection` names the reason and appends `compact SDK account views` when the compiler uses SDK-only internal views. The existing field accepts this text. The image build environment records `tare-toolchain:0.6.0` when that image is selected. Contract 0.7.1 remains the validation target.
