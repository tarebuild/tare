# Tare round six: final in-place results

Updated by round eight with Tare 0.7.0. One full in-place run covers **238 entries: 189 measured, 49 explicitly not built**. All **257,892** proof transactions match. No previously measured program is lost.

Instruction-median totals: **6,295,676 gross -> 4,459,739 net CU** (1,835,937 removed). ELF totals: **38,786,032 -> 30,970,216 bytes** (7,815,816 removed). Totals sum instruction medians; unreached instructions remain null. Successful samples: 30,722.

## Production programs

All three production programs have smaller ELFs than stock and strictly lower CU on every reached instruction.

| Program | Gross CU | Net CU | Stock bytes | Tare bytes | Reached instructions | Matched transactions |
|---|---:|---:|---:|---:|---:|---:|
| Squads v4 | 307,612 | 278,716 | 791,768 | 777,488 | 34/36 | 485 |
| Whirlpools | 184,638 | 173,399 | 1,556,816 | 1,445,224 | 20/65 | 217 |
| Marinade | 72,737 | 65,473 | 964,616 | 782,112 | 6/29 | 67 |

Whirlpools: `close_position` **8,504 -> 8,380**, `collect_fees` **10,199 -> 9,991**, `swap` **26,173 -> 25,842 CU**. Its full 217-input proof matches. Squads full 485-input proof and Marinade full 67-input proof also match.

## Measured causes and changes

- [Squads symbols](transcripts/round-eight-squads-size.txt): instruction wrappers and repeated SDK construction/serialization code dominate the size gap. SDK account exits and descriptor construction now share out-of-line implementations.
- [Whirlpools trace](transcripts/round-eight-whirlpool-trace.txt): boxed metadata access created redundant SDK descriptor caches, and small close contexts copied account/results between loader and wrapper boundaries. Boxed metadata now forwards the existing view. Contexts with at most six leaf fields, a boxed account and a close constraint, with no init/zero/realloc, inline their loads and checks.
- The checks and error order stay unchanged. Broad field-load inlining exceeded size/stack limits and was reverted. A direct output-slot trial increased CU and was reverted. No rejected trial is in the final compiler.

## Remaining size failures

No reached instruction exceeds stock CU. The ten size exceptions remain measured and keep the general gate nonzero.

| Program | Excess bytes | Cause |
|---|---:|---|
| `v1_2_2-tests-bench-programs-bench` | 360,208 | Repeated generated initialization and validation code remains in .text. |
| `v0_32_3-tests-bench-programs-bench` | 185,888 | Repeated generated initialization and validation code remains in .text. |
| `v0_31_1-tests-bench-programs-bench` | 185,376 | Repeated generated initialization and validation code remains in .text. |
| `v1_2_2-tests-misc-programs-misc-optional` | 77,376 | Repeated generated initialization and validation code remains in .text. |
| `v1_2_2-tests-misc-programs-misc` | 65,688 | Repeated generated initialization and validation code remains in .text. |
| `v0_32_3-tests-misc-programs-misc-optional` | 45,584 | Repeated generated initialization and validation code remains in .text. |
| `v0_31_1-tests-misc-programs-misc-optional` | 45,072 | Repeated generated initialization and validation code remains in .text. |
| `v0_32_3-tests-misc-programs-misc` | 22,144 | Repeated generated initialization and validation code remains in .text. |
| `v0_31_1-tests-misc-programs-misc` | 21,632 | Repeated generated initialization and validation code remains in .text. |
| `v1_2_2-tests-optional-programs-optional` | 240 | SDK exit sharing shifts this small optional-account binary 240 bytes above stock; no CU failure. |

## Checks and run

The six pre-run canaries (basic-1, new-idl 0.32.3, registry, Squads, Whirlpools, Marinade) pass their size, CU and proof checks. The full run uses the real `bench/corpus` checkouts with one shared `toolchain/target/sbf-cache/<version>/repro-v1` per version. There are no workspace copies, per-run homes or per-run target trees. Native IDL builds use the fixed short `toolchain/target/i/<version>` path.

All 378 final stock/Tare ELF hashes and sizes match their rows. All eight corpus checkouts have no tracked changes. Results, compatibility, weighings and proofs pass their published schemas. Registry money-path tests: 17/17 match. Native proofs do not claim Linux/validator execution.

```text
Current-stock gate: FAIL: 179/189 measured programs pass; 10 size failures; 0 instruction CU failures.
Coverage: 238/238 manifest results; 49 explicit not-built; 0 coverage issues; 0 proof failures.
v0_31_1-tests-misc-programs-misc-optional: ELF exceeds stock by 45072 bytes
v0_31_1-tests-misc-programs-misc: ELF exceeds stock by 21632 bytes
v0_31_1-tests-bench-programs-bench: ELF exceeds stock by 185376 bytes
v0_32_3-tests-misc-programs-misc-optional: ELF exceeds stock by 45584 bytes
v0_32_3-tests-misc-programs-misc: ELF exceeds stock by 22144 bytes
v0_32_3-tests-bench-programs-bench: ELF exceeds stock by 185888 bytes
v1_2_2-tests-optional-programs-optional: ELF exceeds stock by 240 bytes
v1_2_2-tests-misc-programs-misc-optional: ELF exceeds stock by 77376 bytes
v1_2_2-tests-misc-programs-misc: ELF exceeds stock by 65688 bytes
v1_2_2-tests-bench-programs-bench: ELF exceeds stock by 360208 bytes
Report: bench\gaps\round-six\gate.json
```

Disk free at round start: **349,481,385,984 bytes (349.48 GB)**. Full-run start: **339,753,725,952 bytes (339.75 GB)**; full-run end before cleanup: **309,802,074,112 bytes (309.80 GB)**. Every 20-program checkpoint stays above 40 GB. Post-cleanup space is recorded below.

[Results](results.json) | [Compatibility](compat.json) | [Full run and gate](transcripts/round-eight.txt). Temporary diagnostics, candidate JSON and new build files are removed after verification; retained cache paths are preserved.
Post-cleanup disk free: **340,518,858,752 bytes (340.52 GB)**. Temporary diagnostic JSON, candidate results, the cleanup inventory and new generated build files are removed.

## Round ten: Agave 4.4 admission

No current ELF is refused. All 429 existing primary artifacts (214 stock, 215 Tare, including 51 artifacts from explicit not-built rows) pass 1,287 checks: actual deployment, test-validator preload verification, and program-cache load with JIT. The 378 artifacts belonging to measured rows still match their recorded hashes. [Before](loader-check/before.json) and [after](loader-check/after.json) contain per-file phases and hashes; no ELF changes between them.

The native Agave cache reproduces `DelayVisibility` at slot zero and `Loaded` at slot one for the valid basic-0 stock and Tare ELFs. The lead's failing test ran at slot zero. This explains `UnsupportedProgramId` without an ELF rejection; the Linux preflight now waits for ProgramData deployment slot + 1 before running tests. The lead must rerun the Linux suite to confirm the timing fix there.

The CLI now requires the exact admission check for both artifacts during build and prove. A malformed-ELF control is rejected. No corpus compilation or benchmark measurement is repeated; Tare stays 0.7.0, and the attached record remains valid. The compiler and bytecode stage are unchanged. Existing totals and compatibility facts are preserved.
Round-ten disk free: 315,342,536,704 bytes at start; 315,591,528,448 bytes after cleanup. No SBF rebuilds.
