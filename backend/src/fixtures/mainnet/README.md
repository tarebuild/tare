# Mainnet boundary captures

These are eight individual read-only requests to the public Solana mainnet endpoint, captured on 2026-11-14. There was no sampling run, polling loop, retry loop, transaction submission, or RPC key. Each JSON file includes the JSON request and response, capture time and HTTP status.

- Squads v4: sliced program header, zero-byte program-data slice with full space, one signature, and its legacy IDL account.
- Whirlpools: sliced program header, zero-byte program-data slice with full space, one signature, and that transaction.
- The Whirlpools transaction references its program through a v0 loaded readonly address. The grouping test checks the observed SwapV2 discriminator and runtime CU line.
- The zero-byte program-data responses preserve spaces of 1,470,461 bytes for Squads and 10,485,760 bytes for Whirlpools.
- Squads' legacy IDL starts with 184662bf3a907b9e: sha256("internal:IdlAccount")[..8].

These captures test response handling. They are not Tare-versus-stock benchmarks and are never used as benchmark figures.

Source definitions: [Agave account encoding](https://github.com/anza-xyz/agave/blob/master/account-decoder/src/lib.rs), [legacy Anchor IDL account](https://github.com/coral-xyz/anchor/blob/v0.32.1/lang/src/idl.rs), [current Anchor Program Metadata decoding](https://github.com/otter-sec/anchor/blob/master/ts/packages/anchor/src/idl.ts).
