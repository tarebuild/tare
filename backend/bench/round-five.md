# Tare round five

Tare 0.5.0; report source [results.json](results.json). All CU totals sum measured per-instruction medians, not transaction-sequence costs.

Recorded programs: 238/238; measured 189; not built 49. Matched proofs: 189/189, comparing 257,892 transactions.

Current corpus: gross 6,295,676 / net 4,433,845 / tare 1,861,831 CU over 30,722 successful samples. Stock/Tare ELF bytes: 38,786,032 / 35,997,528. Test/generated/mainnet inputs: 17 / 257,875 / 0.

## Current-stock acceptance gate

**Gate FAILED.** Every measured program must have a matched proof and a Tare ELF no larger than its current stock build. Every reached instruction requires net <= gross; production instructions require net < gross. All 238 manifest programs need an explicit result.

172/189 measured programs pass; 17 size failures and 7 instruction-CU failures remain. 49 programs are explicitly not built.

Hash-verified old-input evidence covers 178 programs and 220,531 compared transactions. For those programs only: round-three net 3,808,167; candidate net 3,739,418; current-stock gross 5,469,202 CU. This historical comparison is informational only and never changes current-stock acceptance.

## Production and registry

| Program | Status | Current gross/net CU | Round-three net / exact current net | Stock/Tare bytes | Round-three bytes | Successful methods | Proof inputs | Tests passed/ran |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| tare-registry | matched | 84,919/66,087 | 66,967 / 66,087 | 281,992/281,152 | 261,072 | 4/4 | 17 | 17/17 |
| production-squads-v4 | matched | 307,612/272,053 | outside round three / not replayed | 791,768/1,100,376 | outside round three | 34/36 | 485 | 0/0 |
| production-whirlpools | matched | 184,638/186,847 | outside round three / not replayed | 1,556,816/2,263,656 | outside round three | 20/65 | 217 | 0/0 |
| production-openbook-v2 | Anchor 0.29.0 | not measured | not measured | not measured | not measured | not measured | 0 | 0/0 |
| production-marinade | matched | 72,737/64,878 | outside round three / not replayed | 964,616/1,115,168 | outside round three | 6/29 | 67 | 0/0 |
| production-drift-v2 | Anchor 0.29.0 | not measured | not measured | not measured | not measured | not measured | 0 | 0/0 |

Program addresses come from the current source/IDL and `weighing.program.address`; production source builds are not evidence that their ELF hashes equal currently deployed mainnet programs. Zero validator test counts mean no verified matching suite result, not a passing suite.

## Remaining program gaps

Every remaining failed program appears below, ordered by its largest CU increase. Code families describe source-selected paths; static sections and symbols do not establish an exact causal CU split.

| Program | Maximum CU increase | Size increase | Cause and evidence |
|---|---:|---:|---|
| production-whirlpools | 2,003 | 706,840 | Largest instruction increase +2003 CU. SDK account materialization is selected: representation observation size_of<Self> at math\bn.rs. Source-selected families: account close, balance transfer and owner/size updates; typed account validation and snapshot serialization/exit; account-view extraction and account-info handling; PDA seeds and bump checks. File delta +706840 B is accounted for by the measured sections (.text +745760 B, .data.rel.ro +6304 B, .dynsym +24 B, .dynstr +11 B). Optimizer reports 70 unresolved indirect calls and 6511 additional instruction starts retained conservatively. |
| v1_2_2-tests-bench-programs-bench | 0 | 1,014,728 | Largest instruction increase +0 CU. raw account ABI is selected: raw representation type checked; source has no detected layout observations. File delta +1014728 B is accounted for by the measured sections (.text +1043608 B). Optimizer reports 6 unresolved indirect calls and 3329 additional instruction starts retained conservatively. |
| v0_32_3-tests-bench-programs-bench | 0 | 857,232 | Largest instruction increase +0 CU. raw account ABI is selected: raw representation type checked; source has no detected layout observations. File delta +857232 B is accounted for by the measured sections (.text +884408 B, .data.rel.ro +512 B, .dynsym +24 B, .dynstr +8 B). Optimizer reports 7 unresolved indirect calls and 3482 additional instruction starts retained conservatively. |
| v0_31_1-tests-bench-programs-bench | 0 | 856,720 | Largest instruction increase +0 CU. raw account ABI is selected: raw representation type checked; source has no detected layout observations. File delta +856720 B is accounted for by the measured sections (.text +883896 B, .data.rel.ro +512 B, .dynsym +24 B, .dynstr +8 B). Optimizer reports 7 unresolved indirect calls and 3482 additional instruction starts retained conservatively. |
| production-squads-v4 | 0 | 308,608 | Largest instruction increase +0 CU. SDK account materialization is selected: representation observation size_of<T> at utils\small_vec.rs. File delta +308608 B is accounted for by the measured sections (.text +349392 B, .data.rel.ro +4704 B, .dynsym +48 B, .dynstr +31 B). Optimizer reports 59 unresolved indirect calls and 11447 additional instruction starts retained conservatively. |
| v1_2_2-tests-misc-programs-misc-optional | 0 | 249,664 | Largest instruction increase +0 CU. raw account ABI is selected: raw representation type checked; source has no detected layout observations. File delta +249664 B is accounted for by the measured sections (.text +273736 B, .data.rel.ro +656 B, headersAndPadding +4 B). Optimizer reports 18 unresolved indirect calls and 3686 additional instruction starts retained conservatively. |
| v0_32_3-tests-misc-programs-misc-optional | 0 | 226,056 | Largest instruction increase +0 CU. raw account ABI is selected: raw representation type checked; source has no detected layout observations. File delta +226056 B is accounted for by the measured sections (.text +246936 B, .data.rel.ro +2280 B, .dynsym +24 B, .dynstr +8 B). Optimizer reports 19 unresolved indirect calls and 4388 additional instruction starts retained conservatively. |
| v0_31_1-tests-misc-programs-misc-optional | 0 | 225,544 | Largest instruction increase +0 CU. raw account ABI is selected: raw representation type checked; source has no detected layout observations. File delta +225544 B is accounted for by the measured sections (.text +246424 B, .data.rel.ro +2280 B, .dynsym +24 B, .dynstr +8 B). Optimizer reports 19 unresolved indirect calls and 4388 additional instruction starts retained conservatively. |
| v1_2_2-tests-misc-programs-misc | 0 | 224,008 | Largest instruction increase +0 CU. raw account ABI is selected: raw representation type checked; source has no detected layout observations. File delta +224008 B is accounted for by the measured sections (.text +247896 B, .data.rel.ro +544 B). Optimizer reports 14 unresolved indirect calls and 3693 additional instruction starts retained conservatively. |
| v0_32_3-tests-misc-programs-misc | 0 | 185,392 | Largest instruction increase +0 CU. raw account ABI is selected: raw representation type checked; source has no detected layout observations. File delta +185392 B is accounted for by the measured sections (.text +205952 B, .data.rel.ro +2208 B, .dynsym +24 B, .dynstr +8 B). Optimizer reports 15 unresolved indirect calls and 4389 additional instruction starts retained conservatively. |
| v0_31_1-tests-misc-programs-misc | 0 | 184,880 | Largest instruction increase +0 CU. raw account ABI is selected: raw representation type checked; source has no detected layout observations. File delta +184880 B is accounted for by the measured sections (.text +205440 B, .data.rel.ro +2208 B, .dynsym +24 B, .dynstr +8 B). Optimizer reports 15 unresolved indirect calls and 4389 additional instruction starts retained conservatively. |
| production-marinade | 0 | 150,552 | Largest instruction increase +0 CU. SDK account materialization is selected: representation observation size_of<StakeState> at checks.rs. File delta +150552 B is accounted for by the measured sections (.text +196896 B, .data.rel.ro +6344 B, .dynsym +48 B, .dynstr +31 B). Optimizer reports 26 unresolved indirect calls and 7326 additional instruction starts retained conservatively. |
| v1_2_2-tests-realloc-programs-realloc | 0 | 18,296 | Largest instruction increase +0 CU. raw account ABI is selected: raw representation type checked; source has no detected layout observations. File delta +18296 B is accounted for by the measured sections (.text +32312 B, .data.rel.ro +824 B, headersAndPadding +3 B). Optimizer reports 10 unresolved indirect calls and 4387 additional instruction starts retained conservatively. |
| v1_2_2-tests-optional-programs-optional | 0 | 9,336 | Largest instruction increase +0 CU. raw account ABI is selected: raw representation type checked; source has no detected layout observations. File delta +9336 B is accounted for by the measured sections (.text +22304 B, .data.rel.ro +1168 B). Optimizer reports 6 unresolved indirect calls and 4397 additional instruction starts retained conservatively. |
| v0_32_3-tests-optional-programs-optional | 0 | 2,696 | Largest instruction increase +0 CU. raw account ABI is selected: raw representation type checked; source has no detected layout observations. File delta +2696 B is accounted for by the measured sections (.text +15608 B, .data.rel.ro +2024 B, .dynsym +24 B, .dynstr +8 B). Optimizer reports 7 unresolved indirect calls and 4543 additional instruction starts retained conservatively. |
| v0_31_1-tests-optional-programs-optional | 0 | 2,168 | Largest instruction increase +0 CU. raw account ABI is selected: raw representation type checked; source has no detected layout observations. File delta +2168 B is accounted for by the measured sections (.text +15080 B, .data.rel.ro +2024 B, .dynsym +24 B, .dynstr +8 B). Optimizer reports 7 unresolved indirect calls and 4543 additional instruction starts retained conservatively. |
| v1_2_2-tests-duplicate-mutable-accounts-programs-duplicate-mutable-accounts | 0 | 1,648 | Largest instruction increase +0 CU. raw account ABI is selected: raw representation type checked; source has no detected layout observations. File delta +1648 B is accounted for by the measured sections (.text +20392 B, headersAndPadding +5 B). Optimizer reports 3 unresolved indirect calls and 3367 additional instruction starts retained conservatively. |

### Size increases, largest first

| Program | Stock bytes | Current bytes | Increase |
|---|---:|---:|---:|
| v1_2_2-tests-bench-programs-bench | 833,576 | 1,848,304 | 1,014,728 |
| v0_32_3-tests-bench-programs-bench | 1,066,280 | 1,923,512 | 857,232 |
| v0_31_1-tests-bench-programs-bench | 1,066,792 | 1,923,512 | 856,720 |
| production-whirlpools | 1,556,816 | 2,263,656 | 706,840 |
| production-squads-v4 | 791,768 | 1,100,376 | 308,608 |
| v1_2_2-tests-misc-programs-misc-optional | 695,384 | 945,048 | 249,664 |
| v0_32_3-tests-misc-programs-misc-optional | 767,632 | 993,688 | 226,056 |
| v0_31_1-tests-misc-programs-misc-optional | 767,912 | 993,456 | 225,544 |
| v1_2_2-tests-misc-programs-misc | 636,304 | 860,312 | 224,008 |
| v0_32_3-tests-misc-programs-misc | 716,376 | 901,768 | 185,392 |
| v0_31_1-tests-misc-programs-misc | 716,656 | 901,536 | 184,880 |
| production-marinade | 964,616 | 1,115,168 | 150,552 |
| v1_2_2-tests-realloc-programs-realloc | 188,744 | 207,040 | 18,296 |
| v1_2_2-tests-optional-programs-optional | 186,048 | 195,384 | 9,336 |
| v0_32_3-tests-optional-programs-optional | 234,832 | 237,528 | 2,696 |
| v0_31_1-tests-optional-programs-optional | 235,360 | 237,528 | 2,168 |
| v1_2_2-tests-duplicate-mutable-accounts-programs-duplicate-mutable-accounts | 182,288 | 183,936 | 1,648 |

### Current-stock instruction failures, largest first

| Program | Instruction | Stock CU | Current CU | Increase | Samples | Source family |
|---|---|---:|---:|---:|---:|---|
| production-whirlpools | collect_fees | 10,199 | 12,202 | 2,003 | 4 | typed account validation and snapshot serialization/exit; declared relationship/address constraints |
| production-whirlpools | swap | 26,173 | 27,978 | 1,805 | 3 | typed account validation and snapshot serialization/exit; account-view extraction and account-info handling; PDA seeds and bump checks; declared relationship/address constraints |
| production-whirlpools | close_position | 8,504 | 9,433 | 929 | 4 | account close, balance transfer and owner/size updates; typed account validation and snapshot serialization/exit; account-view extraction and account-info handling; PDA seeds and bump checks; declared relationship/address constraints |
| production-whirlpools | initialize_pool | 31,930 | 32,187 | 257 | 3 | System initialization CPI and Rent validation; typed account validation and snapshot serialization/exit; sysvar reads and runtime syscall path; PDA seeds and bump checks; declared relationship/address constraints |
| production-whirlpools | update_fees_and_rewards | 6,792 | 7,000 | 208 | 3 | typed account validation and snapshot serialization/exit; account-view extraction and account-info handling; declared relationship/address constraints |
| production-whirlpools | set_fee_rate | 3,632 | 3,640 | 8 | 2 | typed account validation and snapshot serialization/exit; declared relationship/address constraints |
| production-whirlpools | set_protocol_fee_rate | 3,632 | 3,640 | 8 | 2 | typed account validation and snapshot serialization/exit; declared relationship/address constraints |

## Current binaries larger than stock

| Program | Stock bytes | Tare bytes | Increase |
|---|---:|---:|---:|
| v1_2_2-tests-bench-programs-bench | 833,576 | 1,848,304 | 1,014,728 |
| v0_32_3-tests-bench-programs-bench | 1,066,280 | 1,923,512 | 857,232 |
| v0_31_1-tests-bench-programs-bench | 1,066,792 | 1,923,512 | 856,720 |
| production-whirlpools | 1,556,816 | 2,263,656 | 706,840 |
| production-squads-v4 | 791,768 | 1,100,376 | 308,608 |
| v1_2_2-tests-misc-programs-misc-optional | 695,384 | 945,048 | 249,664 |
| v0_32_3-tests-misc-programs-misc-optional | 767,632 | 993,688 | 226,056 |
| v0_31_1-tests-misc-programs-misc-optional | 767,912 | 993,456 | 225,544 |
| v1_2_2-tests-misc-programs-misc | 636,304 | 860,312 | 224,008 |
| v0_32_3-tests-misc-programs-misc | 716,376 | 901,768 | 185,392 |
| v0_31_1-tests-misc-programs-misc | 716,656 | 901,536 | 184,880 |
| production-marinade | 964,616 | 1,115,168 | 150,552 |
| v1_2_2-tests-realloc-programs-realloc | 188,744 | 207,040 | 18,296 |
| v1_2_2-tests-optional-programs-optional | 186,048 | 195,384 | 9,336 |
| v0_32_3-tests-optional-programs-optional | 234,832 | 237,528 | 2,696 |
| v0_31_1-tests-optional-programs-optional | 235,360 | 237,528 | 2,168 |
| v1_2_2-tests-duplicate-mutable-accounts-programs-duplicate-mutable-accounts | 182,288 | 183,936 | 1,648 |

## Current-corpus medians above stock

These use the current generated/recorded corpus, not the frozen round-three gate inputs.

| Program | Instruction | Gross CU | Net CU | Increase | Samples |
|---|---|---:|---:|---:|---:|
| production-whirlpools | collect_fees | 10,199 | 12,202 | 2,003 | 4 |
| production-whirlpools | swap | 26,173 | 27,978 | 1,805 | 3 |
| production-whirlpools | close_position | 8,504 | 9,433 | 929 | 4 |
| production-whirlpools | initialize_pool | 31,930 | 32,187 | 257 | 3 |
| production-whirlpools | update_fees_and_rewards | 6,792 | 7,000 | 208 | 3 |
| production-whirlpools | set_fee_rate | 3,632 | 3,640 | 8 | 2 |
| production-whirlpools | set_protocol_fee_rate | 3,632 | 3,640 | 8 | 2 |

## Instructions without a successful input

136 current instruction rows remain unmeasured. adaptive_fee_scenario_missing: 8, additional_token_program_scenario_missing: 14, dynamic_tick_array_scenario_missing: 1, fixed_production_authority_unavailable: 1, generated_state_missing: 35, locked_position_scenario_missing: 2, position_bundle_scenario_missing: 5, position_range_scenario_missing: 1, production_admin_signature: 2, protocol_fee_scenario_missing: 1, reward_vault_scenario_missing: 5, second_pool_scenario_missing: 2, source_constraints_impossible: 3, token_badge_scenario_missing: 3, unconditional_source_error: 1, unreachable_by_source: 51, upstream_source_idl_mismatch: 1.

| Program | Instruction | Category | Source reason |
|---|---|---|---|
| errors | hello | unreachable_by_source | The handler directly returns Err for every input. |
| errors | hello_next | unreachable_by_source | The handler directly returns Err for every input. |
| errors | hello_no_msg | unreachable_by_source | The handler directly returns Err for every input. |
| errors | require_eq | unreachable_by_source | The handler compares fixed constants with require_eq!(5241, 124124124, ...), which always fails. |
| errors | require_eq_default_error | unreachable_by_source | The handler compares fixed constants with require_eq!(5241, 124124124, ...), which always fails. |
| errors | require_gt | unreachable_by_source | The handler compares fixed constants with require_gt!(5, 10, ...), which always fails. |
| errors | require_gt_default_error | unreachable_by_source | The handler compares fixed constants with require_gt!(10, 10, ...), which always fails. |
| errors | require_gte | unreachable_by_source | The handler compares fixed constants with require_gte!(5, 10, ...), which always fails. |
| errors | require_gte_default_error | unreachable_by_source | The handler compares fixed constants with require_gte!(5, 10, ...), which always fails. |
| errors | require_neq | unreachable_by_source | The handler compares fixed constants with require_neq!(500, 500, ...), which always fails. |
| errors | require_neq_default_error | unreachable_by_source | The handler compares fixed constants with require_neq!(500, 500, ...), which always fails. |
| errors | test_err | unreachable_by_source | The handler directly returns Err for every input. |
| errors | test_program_error | unreachable_by_source | The handler directly returns Err for every input. |
| errors | test_program_error_with_source | unreachable_by_source | The handler directly returns Err for every input. |
| errors | test_require | unreachable_by_source | The handler unconditionally executes require!(false, ...). |
| errors | try_into_integer | unreachable_by_source | The handler converts u64::MAX to its smaller signed return type and always returns the conversion error. |
| v0_31_1-tests-pyth-programs-pyth | initialize | generated_state_missing | The generated corpus has not produced a successful input for this handler. Its recorded baseline fails with InstructionError(0, ProgramFailedToComplete). |
| v0_31_1-tests-pyth-programs-pyth | set_price | generated_state_missing | The generated corpus has not produced a successful input for this handler. Its recorded baseline fails with InstructionError(0, ProgramFailedToComplete). |
| v0_31_1-tests-spl-transfer-hook-programs-transfer-hook | execute | generated_state_missing | The generated corpus has not produced a successful input for this handler. Its recorded baseline fails with InstructionError(0, Custom(2021)). |
| v0_31_1-tests-spl-transfer-hook-programs-transfer-hook | initialize | generated_state_missing | The generated corpus has not produced a successful input for this handler. Its recorded baseline fails with InstructionError(0, Custom(2022)). |
| v0_31_1-tests-idl-programs-idl | cause_error | unreachable_by_source | The handler directly returns Err for every input. |
| v0_31_1-tests-misc-programs-misc | test_used_identifiers | source_constraints_impossible | The constraints fix all four one-byte seeds and explicit bumps. Bump4 produces an on-curve address, so no fourth account can satisfy the required PDA constraint. |
| v0_32_3-tests-pyth-programs-pyth | initialize | generated_state_missing | The generated corpus has not produced a successful input for this handler. Its recorded baseline fails with InstructionError(0, ProgramFailedToComplete). |
| v0_32_3-tests-pyth-programs-pyth | set_price | generated_state_missing | The generated corpus has not produced a successful input for this handler. Its recorded baseline fails with InstructionError(0, ProgramFailedToComplete). |
| v0_32_3-tests-errors-programs-errors | hello | unreachable_by_source | The handler directly returns Err for every input. |
| v0_32_3-tests-errors-programs-errors | hello_next | unreachable_by_source | The handler directly returns Err for every input. |
| v0_32_3-tests-errors-programs-errors | hello_no_msg | unreachable_by_source | The handler directly returns Err for every input. |
| v0_32_3-tests-errors-programs-errors | require_eq | unreachable_by_source | The handler compares fixed constants with require_eq!(5241, 124124124, ...), which always fails. |
| v0_32_3-tests-errors-programs-errors | require_eq_default_error | unreachable_by_source | The handler compares fixed constants with require_eq!(5241, 124124124, ...), which always fails. |
| v0_32_3-tests-errors-programs-errors | require_gt | unreachable_by_source | The handler compares fixed constants with require_gt!(5, 10, ...), which always fails. |
| v0_32_3-tests-errors-programs-errors | require_gt_default_error | unreachable_by_source | The handler compares fixed constants with require_gt!(10, 10, ...), which always fails. |
| v0_32_3-tests-errors-programs-errors | require_gte | unreachable_by_source | The handler compares fixed constants with require_gte!(5, 10, ...), which always fails. |
| v0_32_3-tests-errors-programs-errors | require_gte_default_error | unreachable_by_source | The handler compares fixed constants with require_gte!(5, 10, ...), which always fails. |
| v0_32_3-tests-errors-programs-errors | require_neq | unreachable_by_source | The handler compares fixed constants with require_neq!(500, 500, ...), which always fails. |
| v0_32_3-tests-errors-programs-errors | require_neq_default_error | unreachable_by_source | The handler compares fixed constants with require_neq!(500, 500, ...), which always fails. |
| v0_32_3-tests-errors-programs-errors | test_err | unreachable_by_source | The handler directly returns Err for every input. |
| v0_32_3-tests-errors-programs-errors | test_program_error | unreachable_by_source | The handler directly returns Err for every input. |
| v0_32_3-tests-errors-programs-errors | test_program_error_with_source | unreachable_by_source | The handler directly returns Err for every input. |
| v0_32_3-tests-errors-programs-errors | test_require | unreachable_by_source | The handler unconditionally executes require!(false, ...). |
| v0_32_3-tests-errors-programs-errors | try_into_integer | unreachable_by_source | The handler converts u64::MAX to its smaller signed return type and always returns the conversion error. |
| v0_32_3-tests-spl-transfer-hook-programs-transfer-hook | execute | generated_state_missing | The generated corpus has not produced a successful input for this handler. Its recorded baseline fails with InstructionError(0, Custom(2021)). |
| v0_32_3-tests-spl-transfer-hook-programs-transfer-hook | initialize | generated_state_missing | The generated corpus has not produced a successful input for this handler. Its recorded baseline fails with InstructionError(0, Custom(2022)). |
| v0_32_3-tests-idl-programs-idl | cause_error | unreachable_by_source | The handler directly returns Err for every input. |
| v0_32_3-tests-misc-programs-misc | test_used_identifiers | source_constraints_impossible | The constraints fix all four one-byte seeds and explicit bumps. Bump4 produces an on-curve address, so no fourth account can satisfy the required PDA constraint. |
| v1_2_2-tests-pyth-programs-pyth | initialize | generated_state_missing | The generated corpus has not produced a successful input for this handler. Its recorded baseline fails with InstructionError(0, ProgramFailedToComplete). |
| v1_2_2-tests-pyth-programs-pyth | set_price | generated_state_missing | The generated corpus has not produced a successful input for this handler. Its recorded baseline fails with InstructionError(0, ProgramFailedToComplete). |
| v1_2_2-tests-errors-programs-errors | hello | unreachable_by_source | The handler directly returns Err for every input. |
| v1_2_2-tests-errors-programs-errors | hello_next | unreachable_by_source | The handler directly returns Err for every input. |
| v1_2_2-tests-errors-programs-errors | hello_no_msg | unreachable_by_source | The handler directly returns Err for every input. |
| v1_2_2-tests-errors-programs-errors | require_eq | unreachable_by_source | The handler compares fixed constants with require_eq!(5241, 124124124, ...), which always fails. |
| v1_2_2-tests-errors-programs-errors | require_eq_default_error | unreachable_by_source | The handler compares fixed constants with require_eq!(5241, 124124124, ...), which always fails. |
| v1_2_2-tests-errors-programs-errors | require_gt | unreachable_by_source | The handler compares fixed constants with require_gt!(5, 10, ...), which always fails. |
| v1_2_2-tests-errors-programs-errors | require_gt_default_error | unreachable_by_source | The handler compares fixed constants with require_gt!(10, 10, ...), which always fails. |
| v1_2_2-tests-errors-programs-errors | require_gte | unreachable_by_source | The handler compares fixed constants with require_gte!(5, 10, ...), which always fails. |
| v1_2_2-tests-errors-programs-errors | require_gte_default_error | unreachable_by_source | The handler compares fixed constants with require_gte!(5, 10, ...), which always fails. |
| v1_2_2-tests-errors-programs-errors | require_neq | unreachable_by_source | The handler compares fixed constants with require_neq!(500, 500, ...), which always fails. |
| v1_2_2-tests-errors-programs-errors | require_neq_default_error | unreachable_by_source | The handler compares fixed constants with require_neq!(500, 500, ...), which always fails. |
| v1_2_2-tests-errors-programs-errors | test_err | unreachable_by_source | The handler directly returns Err for every input. |
| v1_2_2-tests-errors-programs-errors | test_program_error | unreachable_by_source | The handler directly returns Err for every input. |
| v1_2_2-tests-errors-programs-errors | test_program_error_with_source | unreachable_by_source | The handler directly returns Err for every input. |
| v1_2_2-tests-errors-programs-errors | test_require | unreachable_by_source | The handler unconditionally executes require!(false, ...). |
| v1_2_2-tests-errors-programs-errors | try_into_integer | unreachable_by_source | The handler converts u64::MAX to its smaller signed return type and always returns the conversion error. |
| v1_2_2-tests-spl-transfer-hook-programs-transfer-hook | execute | generated_state_missing | The generated corpus has not produced a successful input for this handler. Its recorded baseline fails with InstructionError(0, Custom(2021)). |
| v1_2_2-tests-spl-transfer-hook-programs-transfer-hook | initialize | generated_state_missing | The generated corpus has not produced a successful input for this handler. Its recorded baseline fails with InstructionError(0, Custom(2022)). |
| v1_2_2-tests-declare-program-programs-declare-program | cpi_empty_composite | upstream_source_idl_mismatch | The caller uses instructions from its checked-in external IDL that the compiled external program source and its generated stock IDL do not implement. Stock dispatch returns 101. |
| v1_2_2-tests-misc-programs-misc | test_used_identifiers | source_constraints_impossible | The constraints fix all four one-byte seeds and explicit bumps. Bump4 produces an on-curve address, so no fourth account can satisfy the required PDA constraint. |
| production-squads-v4 | multisig_create | unreachable_by_source | unconditional source error |
| production-squads-v4 | program_config_init | fixed_production_authority_unavailable | fixed production initializer signature unavailable |
| production-whirlpools | close_bundled_position | position_bundle_scenario_missing | The current lifecycle creates an individual position; no position-bundle NFT, bitmap, or bundled-position lifecycle is generated. |
| production-whirlpools | close_position_with_token_extensions | additional_token_program_scenario_missing | The current successful lifecycle exercises classic SPL Token; this method needs its own token-extension, remaining-account, or metadata account scenario. |
| production-whirlpools | collect_fees_v2 | additional_token_program_scenario_missing | The current successful lifecycle exercises classic SPL Token; this method needs its own token-extension, remaining-account, or metadata account scenario. |
| production-whirlpools | collect_protocol_fees | protocol_fee_scenario_missing | The current lifecycle collects position fees; no dedicated protocol-fee withdrawal transaction is generated. |
| production-whirlpools | collect_protocol_fees_v2 | additional_token_program_scenario_missing | The current successful lifecycle exercises classic SPL Token; this method needs its own token-extension, remaining-account, or metadata account scenario. |
| production-whirlpools | collect_reward | reward_vault_scenario_missing | The current lifecycle has no funded reward mint/vault or initialized reward-emission schedule. |
| production-whirlpools | collect_reward_v2 | additional_token_program_scenario_missing | The current successful lifecycle exercises classic SPL Token; this method needs its own token-extension, remaining-account, or metadata account scenario. |
| production-whirlpools | decrease_liquidity_v2 | additional_token_program_scenario_missing | The current successful lifecycle exercises classic SPL Token; this method needs its own token-extension, remaining-account, or metadata account scenario. |
| production-whirlpools | delete_position_bundle | position_bundle_scenario_missing | The current lifecycle creates an individual position; no position-bundle NFT, bitmap, or bundled-position lifecycle is generated. |
| production-whirlpools | delete_token_badge | token_badge_scenario_missing | The current lifecycle does not enable the admin-controlled token-badge feature or create a badge mint lifecycle. |
| production-whirlpools | idl_include | unconditional_source_error | The source-only IDL inclusion handler always returns InvalidInstructionData. |
| production-whirlpools | increase_liquidity_by_token_amounts_v2 | additional_token_program_scenario_missing | The current successful lifecycle exercises classic SPL Token; this method needs its own token-extension, remaining-account, or metadata account scenario. |
| production-whirlpools | increase_liquidity_v2 | additional_token_program_scenario_missing | The current successful lifecycle exercises classic SPL Token; this method needs its own token-extension, remaining-account, or metadata account scenario. |
| production-whirlpools | initialize_adaptive_fee_tier | adaptive_fee_scenario_missing | The current lifecycle creates a fixed-fee pool; no adaptive-fee tier, oracle state, or delegated authority lifecycle is generated. |
| production-whirlpools | initialize_config | production_admin_signature | The mainnet feature requires a fixed production admin signature. The configured test authorities cannot authorize this instruction. |
| production-whirlpools | initialize_dynamic_tick_array | dynamic_tick_array_scenario_missing | The current lifecycle creates fixed tick arrays; the dynamic account allocation path has no directed input. |
| production-whirlpools | initialize_pool_v2 | additional_token_program_scenario_missing | The current successful lifecycle exercises classic SPL Token; this method needs its own token-extension, remaining-account, or metadata account scenario. |
| production-whirlpools | initialize_pool_with_adaptive_fee | adaptive_fee_scenario_missing | The current lifecycle creates a fixed-fee pool; no adaptive-fee tier, oracle state, or delegated authority lifecycle is generated. |
| production-whirlpools | initialize_position_bundle | position_bundle_scenario_missing | The current lifecycle creates an individual position; no position-bundle NFT, bitmap, or bundled-position lifecycle is generated. |
| production-whirlpools | initialize_position_bundle_with_metadata | position_bundle_scenario_missing | The current lifecycle creates an individual position; no position-bundle NFT, bitmap, or bundled-position lifecycle is generated. |
| production-whirlpools | initialize_reward | reward_vault_scenario_missing | The current lifecycle has no funded reward mint/vault or initialized reward-emission schedule. |
| production-whirlpools | initialize_reward_v2 | additional_token_program_scenario_missing | The current successful lifecycle exercises classic SPL Token; this method needs its own token-extension, remaining-account, or metadata account scenario. |
| production-whirlpools | initialize_token_badge | token_badge_scenario_missing | The current lifecycle does not enable the admin-controlled token-badge feature or create a badge mint lifecycle. |
| production-whirlpools | lock_position | locked_position_scenario_missing | The current lifecycle has no lock configuration or locked position-token owner transition. |
| production-whirlpools | open_bundled_position | position_bundle_scenario_missing | The current lifecycle creates an individual position; no position-bundle NFT, bitmap, or bundled-position lifecycle is generated. |
| production-whirlpools | open_position_with_metadata | additional_token_program_scenario_missing | The current successful lifecycle exercises classic SPL Token; this method needs its own token-extension, remaining-account, or metadata account scenario. |
| production-whirlpools | open_position_with_token_extensions | additional_token_program_scenario_missing | The current successful lifecycle exercises classic SPL Token; this method needs its own token-extension, remaining-account, or metadata account scenario. |
| production-whirlpools | reposition_liquidity_v2 | additional_token_program_scenario_missing | The current successful lifecycle exercises classic SPL Token; this method needs its own token-extension, remaining-account, or metadata account scenario. |
| production-whirlpools | reset_position_range | position_range_scenario_missing | The current lifecycle opens a fixed-range position; a range-reset transition is not generated. |
| production-whirlpools | set_adaptive_fee_constants | adaptive_fee_scenario_missing | The current lifecycle creates a fixed-fee pool; no adaptive-fee tier, oracle state, or delegated authority lifecycle is generated. |
| production-whirlpools | set_config_feature_flag | production_admin_signature | The mainnet feature requires a fixed production admin signature. The configured test authorities cannot authorize this instruction. |
| production-whirlpools | set_default_base_fee_rate | adaptive_fee_scenario_missing | The current lifecycle creates a fixed-fee pool; no adaptive-fee tier, oracle state, or delegated authority lifecycle is generated. |
| production-whirlpools | set_delegated_fee_authority | adaptive_fee_scenario_missing | The current lifecycle creates a fixed-fee pool; no adaptive-fee tier, oracle state, or delegated authority lifecycle is generated. |
| production-whirlpools | set_fee_rate_by_delegated_fee_authority | adaptive_fee_scenario_missing | The current lifecycle creates a fixed-fee pool; no adaptive-fee tier, oracle state, or delegated authority lifecycle is generated. |
| production-whirlpools | set_initialize_pool_authority | adaptive_fee_scenario_missing | The current lifecycle creates a fixed-fee pool; no adaptive-fee tier, oracle state, or delegated authority lifecycle is generated. |
| production-whirlpools | set_preset_adaptive_fee_constants | adaptive_fee_scenario_missing | The current lifecycle creates a fixed-fee pool; no adaptive-fee tier, oracle state, or delegated authority lifecycle is generated. |
| production-whirlpools | set_reward_authority | reward_vault_scenario_missing | The current lifecycle has no funded reward mint/vault or initialized reward-emission schedule. |
| production-whirlpools | set_reward_authority_by_super_authority | reward_vault_scenario_missing | The current lifecycle has no funded reward mint/vault or initialized reward-emission schedule. |
| production-whirlpools | set_reward_emissions | reward_vault_scenario_missing | The current lifecycle has no funded reward mint/vault or initialized reward-emission schedule. |
| production-whirlpools | set_reward_emissions_v2 | additional_token_program_scenario_missing | The current successful lifecycle exercises classic SPL Token; this method needs its own token-extension, remaining-account, or metadata account scenario. |
| production-whirlpools | set_token_badge_attribute | token_badge_scenario_missing | The current lifecycle does not enable the admin-controlled token-badge feature or create a badge mint lifecycle. |
| production-whirlpools | swap_v2 | additional_token_program_scenario_missing | The current successful lifecycle exercises classic SPL Token; this method needs its own token-extension, remaining-account, or metadata account scenario. |
| production-whirlpools | transfer_locked_position | locked_position_scenario_missing | The current lifecycle has no lock configuration or locked position-token owner transition. |
| production-whirlpools | two_hop_swap | second_pool_scenario_missing | The current trading lifecycle has one pool; this instruction needs a funded second pool and ordered route accounts. |
| production-whirlpools | two_hop_swap_v2 | second_pool_scenario_missing | The current trading lifecycle has one pool; this instruction needs a funded second pool and ordered route accounts. |
| production-marinade | add_liquidity | generated_state_missing | Needs a directed funded LP-token/liquidity withdrawal lifecycle after initialization. |
| production-marinade | add_validator | generated_state_missing | Needs a directed validator/stake-account lifecycle and, for epoch-sensitive paths, explicit clock/stake-history progression. |
| production-marinade | claim | generated_state_missing | Needs a delayed-unstake ticket lifecycle and epoch progression. |
| production-marinade | config_lp | generated_state_missing | The current initialized deposit lifecycle has no directed input for this state transition. |
| production-marinade | config_marinade | generated_state_missing | The current initialized deposit lifecycle has no directed input for this state transition. |
| production-marinade | create_canonical_stake | generated_state_missing | Needs a directed validator/stake-account lifecycle and, for epoch-sensitive paths, explicit clock/stake-history progression. |
| production-marinade | deactivate_stake | generated_state_missing | Needs a directed validator/stake-account lifecycle and, for epoch-sensitive paths, explicit clock/stake-history progression. |
| production-marinade | deposit_stake_account | generated_state_missing | Needs a directed validator/stake-account lifecycle and, for epoch-sensitive paths, explicit clock/stake-history progression. |
| production-marinade | emergency_unstake | generated_state_missing | Needs a directed validator/stake-account lifecycle and, for epoch-sensitive paths, explicit clock/stake-history progression. |
| production-marinade | finalize_delinquent_upgrade | generated_state_missing | Needs a directed validator/stake-account lifecycle and, for epoch-sensitive paths, explicit clock/stake-history progression. |
| production-marinade | liquid_unstake | generated_state_missing | Needs a directed validator/stake-account lifecycle and, for epoch-sensitive paths, explicit clock/stake-history progression. |
| production-marinade | merge_stakes | generated_state_missing | Needs a directed validator/stake-account lifecycle and, for epoch-sensitive paths, explicit clock/stake-history progression. |
| production-marinade | order_unstake | generated_state_missing | Needs a directed validator/stake-account lifecycle and, for epoch-sensitive paths, explicit clock/stake-history progression. |
| production-marinade | partial_unstake | generated_state_missing | Needs a directed validator/stake-account lifecycle and, for epoch-sensitive paths, explicit clock/stake-history progression. |
| production-marinade | realloc_stake_list | generated_state_missing | Needs a directed validator/stake-account lifecycle and, for epoch-sensitive paths, explicit clock/stake-history progression. |
| production-marinade | realloc_validator_list | generated_state_missing | Needs a directed validator/stake-account lifecycle and, for epoch-sensitive paths, explicit clock/stake-history progression. |
| production-marinade | remove_liquidity | generated_state_missing | Needs a directed funded LP-token/liquidity withdrawal lifecycle after initialization. |
| production-marinade | remove_validator | generated_state_missing | Needs a directed validator/stake-account lifecycle and, for epoch-sensitive paths, explicit clock/stake-history progression. |
| production-marinade | set_validator_score | generated_state_missing | Needs a directed validator/stake-account lifecycle and, for epoch-sensitive paths, explicit clock/stake-history progression. |
| production-marinade | stake_reserve | generated_state_missing | Needs a directed validator/stake-account lifecycle and, for epoch-sensitive paths, explicit clock/stake-history progression. |
| production-marinade | update_active | generated_state_missing | Needs a directed validator/stake-account lifecycle and, for epoch-sensitive paths, explicit clock/stake-history progression. |
| production-marinade | update_deactivated | generated_state_missing | Needs a directed validator/stake-account lifecycle and, for epoch-sensitive paths, explicit clock/stake-history progression. |
| production-marinade | withdraw_stake_account | generated_state_missing | Needs a directed validator/stake-account lifecycle and, for epoch-sensitive paths, explicit clock/stake-history progression. |

## Not built

| Program | Anchor | Construct or actual failure |
|---|---|---|
| v0_31_1-tests-custom-discriminator-programs-ambiguous-discriminator | 0.31.1 | stock IDL validation |
| v0_31_1-tests-bpf-upgradeable-state-programs-bpf-upgradeable-state | 0.31.1 | generated corpus account/type resolution |
| v0_31_1-tests-lazy-account-programs-lazy-account | 0.31.1 | LazyAccount |
| v0_31_1-tests-declare-program-programs-declare-program | 0.31.1 | SBF stack frame > 4096 bytes |
| v0_31_1-tests-tictactoe-programs-tictactoe | 0.31.1 | error[E0277]: the trait bound `anchor_lang::Error: From<ErrorCode>` is not satisfied   --> programs\tictactoe\src\lib.rs:96:47    &#124; 96 &#124;             return Err(ErrorCode::Illegalmove.into());    &#124;                                               ^^^^ the trait `From<ErrorCode>` is not implemented for `anchor_lang::Error`    &#124; help: the following other types implement trait `From<T>`   --> src\error.rs:294:0    &#124;    = note: `anchor_lang::Error` implements `From<ProgramError>`   ::: src\error.rs:349:0    &#124;    = note: `anchor_lang::Error` implements `From<anchor_lang::ErrorCode>`   ::: src\error.rs:605:0    &#124;    = note: `anchor_lang::Error` implements `From<std::io::Error>`   ::: src\error.rs:611:0    &#124; |
| v0_31_1-tests-pda-derivation-programs-pda-derivation | 0.31.1 | generated corpus account/type resolution |
| v0_31_1-tests-multisig-programs-multisig | 0.31.1 | Pipeline incomplete: error: generated account resolver cannot sign sequence 'seed-1413567045-0' input 'change_threshold:short-instruction' (instructions: change_threshold; unresolved signer accounts: 84DbJDDHyFw9q8aRY7BP65k6oFB22Jr2xxcjvk1ReeYw): not enough signers; see transcripts/v0_31_1-tests-multisig-programs-multisig.txt. |
| v0_31_1-tests-spl-token-extensions-programs-token-extensions | 0.31.1 | extensions::metadata_pointer::authority |
| v0_31_1-tests-idl-programs-new-idl | 0.31.1 | stock IDL validation |
| v0_31_1-tests-swap-programs-swap | 0.31.1 | upstream dependency resolution |
| v0_31_1-tests-lockup-programs-lockup | 0.31.1 | error[E0432]: unresolved import `anchor_lang::accounts::state`  --> programs\lockup\src\lib.rs:4:28   &#124; 4 &#124; use anchor_lang::accounts::state::ProgramState;   &#124;                            ^^^^^ could not find `state` in `accounts`  error: cannot find attribute `interface` in this scope    --> programs\lockup\src\lib.rs:539:3     &#124; 539 &#124; #[interface]     &#124;   ^^^^^^^^^  error: cannot find attribute `state` in this scope   --> programs\lockup\src\lib.rs:18:7    &#124; 18 &#124;     #[state]    &#124;       ^^^^^ |
| v0_31_1-tests-ido-pool-programs-ido-pool | 0.31.1 | generated corpus account/type resolution |
| v0_31_1-tests-cfo-programs-cfo | 0.31.1 | upstream dependency resolution |
| v0_31_1-tests-lockup-programs-registry | 0.31.1 | error[E0432]: unresolved import `anchor_lang::accounts::state`  --> programs\lockup\src\lib.rs:4:28   &#124; 4 &#124; use anchor_lang::accounts::state::ProgramState;   &#124;                            ^^^^^ could not find `state` in `accounts`  error: cannot find attribute `interface` in this scope    --> programs\lockup\src\lib.rs:539:3     &#124; 539 &#124; #[interface]     &#124;   ^^^^^^^^^  error: cannot find attribute `state` in this scope   --> programs\lockup\src\lib.rs:18:7    &#124; 18 &#124;     #[state]    &#124;       ^^^^^ |
| v0_31_1-tests-auction-house-programs-auction-house | 0.31.1 | generated corpus account/type resolution |
| v0_32_3-tests-custom-discriminator-programs-ambiguous-discriminator | 0.32.3 | stock IDL validation |
| v0_32_3-tests-bpf-upgradeable-state-programs-bpf-upgradeable-state | 0.32.3 | generated corpus account/type resolution |
| v0_32_3-tests-lazy-account-programs-lazy-account | 0.32.3 | LazyAccount |
| v0_32_3-tests-declare-program-programs-declare-program | 0.32.3 | SBF stack frame > 4096 bytes |
| v0_32_3-tests-tictactoe-programs-tictactoe | 0.32.3 | error[E0277]: the trait bound `anchor_lang::Error: From<ErrorCode>` is not satisfied   --> programs\tictactoe\src\lib.rs:96:47    &#124; 96 &#124;             return Err(ErrorCode::Illegalmove.into());    &#124;                                               ^^^^ the trait `From<ErrorCode>` is not implemented for `anchor_lang::Error`    &#124; help: the following other types implement trait `From<T>`   --> src\error.rs:294:0    &#124;    = note: `anchor_lang::Error` implements `From<ProgramError>`   ::: src\error.rs:349:0    &#124;    = note: `anchor_lang::Error` implements `From<anchor_lang::ErrorCode>`   ::: src\error.rs:605:0    &#124;    = note: `anchor_lang::Error` implements `From<std::io::Error>`   ::: src\error.rs:611:0    &#124; |
| v0_32_3-tests-pda-derivation-programs-pda-derivation | 0.32.3 | generated corpus account/type resolution |
| v0_32_3-tests-multisig-programs-multisig | 0.32.3 | Pipeline incomplete: error: generated account resolver cannot sign sequence 'seed-1413567045-0' input 'change_threshold:short-instruction' (instructions: change_threshold; unresolved signer accounts: 84DbJDDHyFw9q8aRY7BP65k6oFB22Jr2xxcjvk1ReeYw): not enough signers; see transcripts/v0_32_3-tests-multisig-programs-multisig.txt. |
| v0_32_3-tests-spl-token-extensions-programs-token-extensions | 0.32.3 | extensions::metadata_pointer::authority |
| v0_32_3-tests-swap-programs-swap | 0.32.3 | upstream dependency resolution |
| v0_32_3-tests-lockup-programs-lockup | 0.32.3 | error[E0432]: unresolved import `anchor_lang::accounts::state`  --> programs\lockup\src\lib.rs:4:28   &#124; 4 &#124; use anchor_lang::accounts::state::ProgramState;   &#124;                            ^^^^^ could not find `state` in `accounts`  error: cannot find attribute `interface` in this scope    --> programs\lockup\src\lib.rs:539:3     &#124; 539 &#124; #[interface]     &#124;   ^^^^^^^^^  error: cannot find attribute `state` in this scope   --> programs\lockup\src\lib.rs:18:7    &#124; 18 &#124;     #[state]    &#124;       ^^^^^ |
| v0_32_3-tests-ido-pool-programs-ido-pool | 0.32.3 | generated corpus account/type resolution |
| v0_32_3-tests-cfo-programs-cfo | 0.32.3 | upstream dependency resolution |
| v0_32_3-tests-lockup-programs-registry | 0.32.3 | error[E0432]: unresolved import `anchor_lang::accounts::state`  --> programs\lockup\src\lib.rs:4:28   &#124; 4 &#124; use anchor_lang::accounts::state::ProgramState;   &#124;                            ^^^^^ could not find `state` in `accounts`  error: cannot find attribute `interface` in this scope    --> programs\lockup\src\lib.rs:539:3     &#124; 539 &#124; #[interface]     &#124;   ^^^^^^^^^  error: cannot find attribute `state` in this scope   --> programs\lockup\src\lib.rs:18:7    &#124; 18 &#124;     #[state]    &#124;       ^^^^^ |
| v0_32_3-tests-auction-house-programs-auction-house | 0.32.3 | generated corpus account/type resolution |
| v1_2_2-tests-errors-programs-multiple-errors | 1.2.2 | stock IDL validation |
| v1_2_2-tests-custom-discriminator-programs-zeroed-discriminator | 1.2.2 | stock all-zero discriminator rejection |
| v1_2_2-tests-custom-discriminator-programs-ambiguous-discriminator | 1.2.2 | stock IDL validation |
| v1_2_2-tests-zero-copy-programs-zero-cpi | 1.2.2 | upstream dependency resolution |
| v1_2_2-tests-test-instruction-validation-fail-type-programs-test-instruction-validation | 1.2.2 | error[E0308]: mismatched types   --> programs\test-instruction-validation\src\lib.rs:28:15    &#124;  7 &#124; #[program]    &#124; ---------- in this attribute macro expansion ... 28 &#124;         data: u64, // Handler parameter is u64    &#124;               ^^^ expected fn pointer, found fn item    &#124;    = note: expected fn pointer `for<'a> fn(&'a u64)`                  found fn item `for<'a> fn(&'a u8) {TypeMismatch::<'_>::__anchor_validate_ix_arg_type_0}`    = note: this error originates in the attribute macro `program` (in Nightly builds, run with -Z macro-backtrace for more info)  For more information about this error, try `rustc --explain E0308`. error: could not compile `test-instruction-validation` (lib) due to 1 previous error error: cargo-build-sbf exited with exit code: 1 |
| v1_2_2-tests-test-instruction-validation-fail-args-count-programs-test-instruction-validation | 1.2.2 | stock instruction validation |
| v1_2_2-tests-interface-account-programs-interface-account | 1.2.2 | generated corpus account/type resolution |
| v1_2_2-tests-bpf-upgradeable-state-programs-bpf-upgradeable-state | 1.2.2 | generated corpus account/type resolution |
| v1_2_2-tests-lazy-account-programs-lazy-account | 1.2.2 | LazyAccount |
| v1_2_2-tests-zero-copy-programs-zero-copy | 1.2.2 | upstream dependency resolution |
| v1_2_2-tests-pda-derivation-programs-pda-derivation | 1.2.2 | generated corpus account/type resolution |
| v1_2_2-tests-multisig-programs-multisig | 1.2.2 | Pipeline incomplete: error: generated account resolver cannot sign sequence 'seed-1413567045-0' input 'change_threshold:short-instruction' (instructions: change_threshold; unresolved signer accounts: 84DbJDDHyFw9q8aRY7BP65k6oFB22Jr2xxcjvk1ReeYw): not enough signers; see transcripts/v1_2_2-tests-multisig-programs-multisig.txt. |
| v1_2_2-tests-idl-programs-idl | 1.2.2 | error[E0433]: cannot find `feature` in `solana_program`    --> programs\idl\src\lib.rs:490:53     &#124; 490 &#124;     pub struct Feature(anchor_lang::solana_program::feature::Feature);     &#124;                                                     ^^^^^^^ could not find `feature` in `solana_program`  error[E0433]: cannot find `feature` in `solana_program`    --> programs\idl\src\lib.rs:501:50     &#124; 501 &#124;             Ok(Self(anchor_lang::solana_program::feature::Feature {     &#124;                                                  ^^^^^^^ could not find `feature` in `solana_program`  error[E0433]: cannot find `feature` in `solana_program`    --> programs\idl\src\lib.rs:509:47     &#124; 509 &#124;             Self(anchor_lang::solana_program::feature::Feature {     &#124;                                               ^^^^^^^ could not find `feature` in `solana_program` |
| v1_2_2-tests-lockup-programs-lockup | 1.2.2 | error[E0432]: unresolved import `anchor_lang::accounts::state`  --> programs\lockup\src\lib.rs:4:28   &#124; 4 &#124; use anchor_lang::accounts::state::ProgramState;   &#124;                            ^^^^^ could not find `state` in `accounts`  error: cannot find attribute `interface` in this scope    --> programs\lockup\src\lib.rs:539:3     &#124; 539 &#124; #[interface]     &#124;   ^^^^^^^^^  error: cannot find attribute `state` in this scope   --> programs\lockup\src\lib.rs:18:7    &#124; 18 &#124;     #[state]    &#124;       ^^^^^ |
| v1_2_2-tests-spl-token-extensions-programs-token-extensions | 1.2.2 | extensions::metadata_pointer::authority |
| v1_2_2-tests-ido-pool-programs-ido-pool | 1.2.2 | generated corpus account/type resolution |
| v1_2_2-tests-lockup-programs-registry | 1.2.2 | error[E0432]: unresolved import `anchor_lang::accounts::state`  --> programs\lockup\src\lib.rs:4:28   &#124; 4 &#124; use anchor_lang::accounts::state::ProgramState;   &#124;                            ^^^^^ could not find `state` in `accounts`  error: cannot find attribute `interface` in this scope    --> programs\lockup\src\lib.rs:539:3     &#124; 539 &#124; #[interface]     &#124;   ^^^^^^^^^  error: cannot find attribute `state` in this scope   --> programs\lockup\src\lib.rs:18:7    &#124; 18 &#124;     #[state]    &#124;       ^^^^^ |
| v1_2_2-tests-auction-house-programs-auction-house | 1.2.2 | generated corpus account/type resolution |
| production-openbook-v2 | 0.29.0 | Anchor 0.29.0 |
| production-drift-v2 | 0.29.0 | Anchor 0.29.0 |

## Evidence and remaining checks

[Exact report facts](round-five.json), [gate](history/round-five/gate.json), [per-program cause evidence](gaps/round-five/gap-causes.json) and [unmeasured source reasons](round-five-unmeasured.json) retain the measured facts. No divergent input is filtered. Historical round-three differences are optional information, never the acceptance threshold.

The Windows run does not execute Docker or a Linux validator. The lead's fresh-volume, registry and CLI commands are in [lead-run.md](lead-run.md). Native loader checks and synthetic RPC tests do not substitute for the real validator suite.
