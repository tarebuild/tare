# Benchmark record attachment

`node bench/attach-record.mjs --program ID --record ADDRESS --cluster localnet` attaches a lead-observed attestation to an existing matched measured benchmark row. It is a file-only operation and sends no transaction. With `--url RPC_URL`, it first reads the registry Record and checks owner, discriminator, program address, stock/Tare hashes, source repository/commit, toolchain version, corpus hash/counts and gross/net totals. RPC credentials are never written to the result. `--registry-program ADDRESS` overrides the checked ABI's registry address. `--results FILE` selects a noncanonical result file.

The existing `record` field is the address string. Contract 0.7.1 accepts optional `recordCluster: string` (a non-secret label such as localnet, 1 to 64 ASCII letters/digits/dot/underscore/hyphen) and `recordUrl: string` (an HTTP(S) page URL). Supply `--record-url URL` to store the page URL. No contract files are modified by this helper.

A later remeasurement must retain an attachment only if both build hashes and the corpus hash still match; otherwise clear it and attest the new proof. The helper checks that the result file did not change during an optional RPC lookup, then replaces it atomically. Existing other rows and measured figures are preserved.
