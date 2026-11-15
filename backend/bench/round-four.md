# Tare round four

Tare 0.4.0; 2026-11-14T20:29:03.817Z. This report reads [results.json](results.json) and its archived artifacts. Totals sum successful per-instruction medians; they are not transaction-sequence totals.

238 of 238 manifest programs are recorded: 189 measured and 49 not built. 189 proofs match across 257,892 compared transactions. Corpus counts: 257,875 generated, 17 recorded tests, 0 mainnet.

Compute: 6,295,676 gross / 4,775,892 net / 1,519,784 tare CU from 30,722 successful samples. ELF bytes: 38,786,032 stock / 40,750,696 Tare. Rent lamports: 197,199,143,320 stock / 207,179,636,440 Tare.

## Production programs

| Program | Status | Gross CU | Net CU | Successful instructions | Compared inputs |
|---|---|---:|---:|---:|---:|
| production-squads-v4 | matched | 307,612 | 272,114 | 34/36 | 485 |
| production-whirlpools | matched | 184,638 | 186,932 | 20/65 | 217 |
| production-openbook-v2 | Anchor 0.29.0 | ? | ? | ? | ? |
| production-marinade | matched | 72,737 | 64,928 | 6/29 | 67 |
| production-drift-v2 | Anchor 0.29.0 | ? | ? | ? | ? |

Program addresses are checked through `weighing.program.address`. These native source builds do not claim equality with deployed mainnet binaries. Directed production scenarios retain explicit config snapshots and real System/SPL initialization transactions.

## Remaining measured gaps

35 measured programs fail the strictly-smaller ELF requirement. 23 instruction medians in 10 programs exceed stock. Every difference remains recorded; matching observable behavior does not waive a performance regression.

| Program | Stock bytes | Tare bytes | Increase |
|---|---:|---:|---:|
| v0_31_1-tests-realloc-programs-realloc | 210,376 | 232,008 | 21,632 |
| v0_31_1-tests-optional-programs-optional | 235,360 | 271,728 | 36,368 |
| v0_31_1-tests-idl-programs-idl | 255,656 | 266,184 | 10,528 |
| v0_31_1-tests-spl-token-proxy-programs-token-proxy | 300,744 | 336,568 | 35,824 |
| v0_31_1-tests-escrow-programs-escrow | 245,312 | 272,584 | 27,272 |
| v0_31_1-tests-misc-programs-misc-optional | 767,912 | 1,048,552 | 280,640 |
| v0_31_1-tests-misc-programs-misc | 716,656 | 1,002,568 | 285,912 |
| v0_31_1-tests-bench-programs-bench | 1,066,792 | 1,882,480 | 815,688 |
| v0_32_3-tests-realloc-programs-realloc | 209,840 | 232,008 | 22,168 |
| v0_32_3-tests-optional-programs-optional | 234,832 | 271,728 | 36,896 |
| v0_32_3-tests-idl-programs-idl | 255,128 | 266,184 | 11,056 |
| v0_32_3-tests-spl-token-proxy-programs-token-proxy | 300,464 | 336,800 | 36,336 |
| v0_32_3-tests-escrow-programs-escrow | 244,848 | 272,632 | 27,784 |
| v0_32_3-tests-idl-programs-new-idl | 283,536 | 308,912 | 25,376 |
| v0_32_3-tests-misc-programs-misc-optional | 767,632 | 1,048,784 | 281,152 |
| v0_32_3-tests-misc-programs-misc | 716,376 | 1,002,800 | 286,424 |
| v0_32_3-tests-bench-programs-bench | 1,066,280 | 1,882,480 | 816,200 |
| v1_2_2-tests-realloc-programs-realloc | 188,744 | 235,632 | 46,888 |
| v1_2_2-tests-optional-programs-optional | 186,048 | 220,744 | 34,696 |
| v1_2_2-tests-duplicate-mutable-accounts-programs-duplicate-mutable-accounts | 182,288 | 236,472 | 54,184 |
| v1_2_2-tests-declare-program-programs-declare-program | 134,176 | 136,256 | 2,080 |
| v1_2_2-tests-tictactoe-programs-tictactoe | 123,816 | 126,104 | 2,288 |
| v1_2_2-tests-cashiers-check-programs-cashiers-check | 152,184 | 158,072 | 5,888 |
| v1_2_2-tests-declare-program-programs-external | 172,560 | 179,904 | 7,344 |
| v1_2_2-tests-cpi-returns-programs-caller | 137,840 | 146,232 | 8,392 |
| v1_2_2-tests-spl-token-proxy-programs-token-proxy | 244,520 | 272,696 | 28,176 |
| v1_2_2-tests-escrow-programs-escrow | 195,656 | 216,760 | 21,104 |
| v1_2_2-tests-spl-token-wrapper-programs-token-wrapper | 236,232 | 236,392 | 160 |
| v1_2_2-tests-misc-programs-misc-optional | 695,384 | 1,022,872 | 327,488 |
| v1_2_2-tests-misc-programs-misc | 636,304 | 965,056 | 328,752 |
| v1_2_2-tests-bench-programs-bench | 833,576 | 1,839,400 | 1,005,824 |
| tare-registry | 281,992 | 288,272 | 6,280 |
| production-squads-v4 | 791,768 | 1,099,000 | 307,232 |
| production-whirlpools | 1,556,816 | 2,262,360 | 705,544 |
| production-marinade | 964,616 | 1,115,976 | 151,360 |

| Program | Instruction | Gross CU | Net CU | Increase | Successful samples |
|---|---|---:|---:|---:|---:|
| v0_31_1-tests-spl-token-proxy-programs-token-proxy | proxy_optional_transfer | 5,134 | 5,312 | 178 | 56 |
| v0_31_1-tests-spl-token-proxy-programs-token-proxy | proxy_transfer | 5,231 | 5,313 | 82 | 40 |
| v0_31_1-tests-escrow-programs-escrow | exchange | 15,861 | 16,701 | 840 | 4 |
| v0_31_1-tests-spl-token-wrapper-programs-token-wrapper | unwrap | 15,191 | 16,760 | 1,569 | 45 |
| v0_31_1-tests-spl-token-wrapper-programs-token-wrapper | wrap | 19,414 | 21,063 | 1,649 | 48 |
| v0_32_3-tests-cashiers-check-programs-cashiers-check | cancel_check | 6,871 | 6,915 | 44 | 8 |
| v0_32_3-tests-cashiers-check-programs-cashiers-check | cash_check | 6,883 | 6,917 | 34 | 8 |
| v0_32_3-tests-spl-token-proxy-programs-token-proxy | proxy_burn | 5,222 | 5,344 | 122 | 32 |
| v0_32_3-tests-spl-token-proxy-programs-token-proxy | proxy_mint_to | 5,138 | 5,258 | 120 | 32 |
| v0_32_3-tests-spl-token-proxy-programs-token-proxy | proxy_optional_transfer | 4,955 | 5,312 | 357 | 56 |
| v0_32_3-tests-spl-token-proxy-programs-token-proxy | proxy_transfer | 5,068 | 5,313 | 245 | 40 |
| v0_32_3-tests-escrow-programs-escrow | exchange | 15,356 | 16,702 | 1,346 | 4 |
| v0_32_3-tests-spl-token-wrapper-programs-token-wrapper | unwrap | 14,848 | 16,759 | 1,911 | 45 |
| v0_32_3-tests-spl-token-wrapper-programs-token-wrapper | wrap | 19,073 | 21,064 | 1,991 | 48 |
| v1_2_2-tests-misc-programs-misc-optional | test_init_many_associated_token_accounts | 119,134 | 119,567 | 433 | 24 |
| v1_2_2-tests-misc-programs-misc | test_init_many_associated_token_accounts | 119,131 | 119,565 | 434 | 24 |
| production-whirlpools | close_position | 8,504 | 9,430 | 926 | 4 |
| production-whirlpools | collect_fees | 10,199 | 12,201 | 2,002 | 4 |
| production-whirlpools | initialize_pool | 31,930 | 32,220 | 290 | 3 |
| production-whirlpools | set_fee_rate | 3,632 | 3,639 | 7 | 2 |
| production-whirlpools | set_protocol_fee_rate | 3,632 | 3,639 | 7 | 2 |
| production-whirlpools | swap | 26,173 | 27,986 | 1,813 | 3 |
| production-whirlpools | update_fees_and_rewards | 6,792 | 6,995 | 203 | 3 |

## Frozen old-corpus comparisons

176 native baseline reports match the current Tare ELF hash and their recorded native-report hash. 399 instruction medians regress on those unchanged old inputs. 0 baseline records do not yet match current artifacts and are excluded from this count.

| Program | Instruction | Prior Tare CU | Current Tare CU | Increase |
|---|---|---:|---:|---:|
| puppet | initialize | 1,914 | 1,925 | 11 |
| puppet-master | pull_strings | 2,209 | 2,920 | 711 |
| basic-4 | initialize | 6,684 | 6,752 | 68 |
| v1_2_2-tests-multiple-scripts-programs-example | initialize | 257 | 262 | 5 |
| events | initialize | 484 | 494 | 10 |
| events | test_event | 483 | 495 | 12 |
| events | test_event_cpi | 7,612 | 8,717 | 1,105 |
| errors | account_not_initialized_error | 227 | 257 | 30 |
| errors | account_owned_by_wrong_program_error | 227 | 257 | 30 |
| errors | has_one_error | 676 | 836 | 160 |
| errors | mut_error | 195 | 479 | 284 |
| errors | require_keys_neq | 206 | 219 | 13 |
| errors | require_keys_neq_default_error | 210 | 223 | 13 |
| errors | signer_error | 195 | 199 | 4 |
| v1_2_2-tests-safety-checks-programs-account-info | initialize | 146 | 338 | 192 |
| v1_2_2-tests-system-accounts-programs-system-accounts | initialize | 186 | 193 | 7 |
| v0_32_3-tests-safety-checks-programs-account-info | initialize | 153 | 343 | 190 |
| v1_2_2-tests-interface-account-programs-old | init | 1,968 | 2,004 | 36 |
| v1_2_2-tests-idl-programs-external | test_compilation | 186 | 206 | 20 |
| v1_2_2-tests-safety-checks-programs-ignore-non-accounts | initialize | 161 | 364 | 203 |
| v0_32_3-tests-safety-checks-programs-ignore-non-accounts | initialize | 178 | 370 | 192 |
| v0_32_3-examples-tutorial-basic-3-programs-puppet | initialize | 1,914 | 1,925 | 11 |
| v0_31_1-tests-safety-checks-programs-account-info | initialize | 153 | 343 | 190 |
| v1_2_2-tests-test-instruction-validation-pass-args-count-programs-test-instruction-validation | missing_instruction_attr | 699 | 705 | 6 |
| v1_2_2-tests-test-instruction-validation-pass-args-count-programs-test-instruction-validation | no_params | 288 | 299 | 11 |
| v1_2_2-examples-tutorial-basic-3-programs-puppet | initialize | 1,907 | 1,983 | 76 |
| v1_2_2-examples-tutorial-basic-3-programs-puppet-master | pull_strings | 3,228 | 3,415 | 187 |
| v1_2_2-tests-idl-programs-docs | test_idl_doc_parse | 187 | 207 | 20 |
| v0_32_3-examples-tutorial-basic-3-programs-puppet-master | pull_strings | 2,209 | 2,920 | 711 |
| v0_32_3-tests-idl-programs-docs | test_idl_doc_parse | 203 | 212 | 9 |
| v1_2_2-examples-tutorial-basic-2-programs-basic-2 | increment | 356 | 363 | 7 |
| v1_2_2-tests-events-programs-events | initialize | 483 | 494 | 11 |
| v1_2_2-tests-events-programs-events | test_event | 480 | 491 | 11 |
| v1_2_2-tests-events-programs-events | test_event_cpi | 1,534 | 1,545 | 11 |
| v0_31_1-tests-safety-checks-programs-ignore-non-accounts | initialize | 178 | 370 | 192 |
| v0_31_1-tests-idl-programs-docs | test_idl_doc_parse | 203 | 212 | 9 |
| v0_32_3-tests-events-programs-events | initialize | 484 | 494 | 10 |
| v0_32_3-tests-events-programs-events | test_event | 483 | 495 | 12 |
| v0_32_3-tests-events-programs-events | test_event_cpi | 7,612 | 8,717 | 1,105 |
| v0_32_3-tests-composite-programs-composite | initialize | 766 | 816 | 50 |
| v0_32_3-tests-cpi-returns-programs-callee | return_struct | 335 | 347 | 12 |
| v0_32_3-tests-cpi-returns-programs-callee | return_u64 | 340 | 345 | 5 |
| v0_32_3-tests-cpi-returns-programs-callee | return_vec | 373 | 404 | 31 |
| v0_31_1-tests-composite-programs-composite | initialize | 766 | 816 | 50 |
| v0_31_1-tests-cpi-returns-programs-callee | return_struct | 335 | 347 | 12 |
| v0_31_1-tests-cpi-returns-programs-callee | return_u64 | 340 | 345 | 5 |
| v0_31_1-tests-cpi-returns-programs-callee | return_vec | 373 | 404 | 31 |
| v0_31_1-tests-misc-programs-init-if-needed | close | 483 | 607 | 124 |
| v0_31_1-tests-idl-programs-generics | generic | 1,166 | 1,459 | 293 |
| v0_31_1-tests-custom-discriminator-programs-custom-discriminator | array | 187 | 196 | 9 |
| v0_31_1-tests-custom-discriminator-programs-custom-discriminator | byte_str | 188 | 197 | 9 |
| v0_31_1-tests-custom-discriminator-programs-custom-discriminator | constant | 191 | 200 | 9 |
| v0_31_1-tests-custom-discriminator-programs-custom-discriminator | int | 184 | 194 | 10 |
| v1_2_2-tests-composite-programs-composite | initialize | 743 | 934 | 191 |
| v1_2_2-tests-test-instruction-validation-pass-type-programs-test-instruction-validation | missing_instruction_attr | 641 | 648 | 7 |
| v1_2_2-tests-test-instruction-validation-pass-type-programs-test-instruction-validation | no_params | 293 | 303 | 10 |
| v1_2_2-tests-test-instruction-validation-pass-type-programs-test-instruction-validation | type_mismatch | 633 | 645 | 12 |
| v1_2_2-tests-misc-programs-init-if-needed | close | 481 | 617 | 136 |
| v1_2_2-examples-tutorial-basic-4-programs-basic-4 | initialize | 6,673 | 6,720 | 47 |
| v1_2_2-tests-cpi-returns-programs-callee | initialize | 1,971 | 1,991 | 20 |
| v1_2_2-tests-cpi-returns-programs-callee | return_struct | 337 | 347 | 10 |
| v1_2_2-tests-cpi-returns-programs-callee | return_u64 | 340 | 346 | 6 |
| v1_2_2-tests-cpi-returns-programs-callee | return_u64_spoofed | 339 | 347 | 8 |
| v1_2_2-tests-cpi-returns-programs-callee | return_vec | 375 | 403 | 28 |
| v0_32_3-tests-misc-programs-init-if-needed | close | 483 | 607 | 124 |
| v0_32_3-examples-tutorial-basic-4-programs-basic-4 | initialize | 6,684 | 6,752 | 68 |
| v0_32_3-tests-idl-programs-generics | generic | 1,166 | 1,459 | 293 |
| v0_32_3-tests-custom-discriminator-programs-custom-discriminator | array | 187 | 196 | 9 |
| v0_32_3-tests-custom-discriminator-programs-custom-discriminator | byte_str | 188 | 197 | 9 |
| v0_32_3-tests-custom-discriminator-programs-custom-discriminator | constant | 191 | 200 | 9 |
| v0_32_3-tests-custom-discriminator-programs-custom-discriminator | int | 184 | 194 | 10 |
| v0_32_3-tests-misc-programs-remaining-accounts | test_remaining_accounts | 1,342 | 1,374 | 32 |
| v1_2_2-tests-custom-discriminator-programs-custom-discriminator | array | 187 | 197 | 10 |
| v1_2_2-tests-custom-discriminator-programs-custom-discriminator | byte_str | 188 | 198 | 10 |
| v1_2_2-tests-custom-discriminator-programs-custom-discriminator | constant | 191 | 201 | 10 |
| v1_2_2-tests-custom-discriminator-programs-custom-discriminator | int | 184 | 195 | 11 |
| v1_2_2-tests-misc-programs-remaining-accounts | test_remaining_accounts | 1,221 | 1,376 | 155 |
| v1_2_2-tests-idl-programs-generics | generic | 1,085 | 1,419 | 334 |
| v1_2_2-tests-misc-programs-lamports | overflow | 3,410 | 3,587 | 177 |
| v1_2_2-tests-misc-programs-lamports | transfer | 6,968 | 7,571 | 603 |
| v0_31_1-tests-misc-programs-remaining-accounts | test_remaining_accounts | 1,342 | 1,374 | 32 |
| v0_31_1-tests-realloc-programs-realloc | initialize | 3,815 | 3,949 | 134 |
| v0_31_1-tests-realloc-programs-realloc | realloc2 | 6,989 | 7,477 | 488 |
| v0_31_1-tests-cpi-returns-programs-caller | cpi_call_return_struct | 2,722 | 3,375 | 653 |
| v0_31_1-tests-cpi-returns-programs-caller | cpi_call_return_u64 | 2,722 | 3,374 | 652 |
| v0_31_1-tests-cpi-returns-programs-caller | cpi_call_return_vec | 2,882 | 3,535 | 653 |
| v0_31_1-tests-misc-programs-lamports | overflow | 3,549 | 3,614 | 65 |
| v0_31_1-tests-misc-programs-lamports | transfer | 6,977 | 7,786 | 809 |
| v0_31_1-tests-chat-programs-chat | create_chat_room | 905 | 1,019 | 114 |
| v0_31_1-tests-chat-programs-chat | create_user | 4,513 | 4,845 | 332 |
| v0_31_1-tests-chat-programs-chat | send_message | 3,102 | 3,180 | 78 |
| v0_31_1-tests-declare-program-programs-external | test_compilation_data_as_parameter_name | 180 | 196 | 16 |
| v0_32_3-tests-realloc-programs-realloc | initialize | 3,815 | 3,949 | 134 |
| v0_32_3-tests-realloc-programs-realloc | realloc2 | 6,989 | 7,477 | 488 |
| v0_32_3-tests-cpi-returns-programs-caller | cpi_call_return_struct | 2,720 | 3,373 | 653 |
| v0_32_3-tests-cpi-returns-programs-caller | cpi_call_return_u64 | 2,720 | 3,372 | 652 |
| v0_32_3-tests-cpi-returns-programs-caller | cpi_call_return_vec | 2,880 | 3,533 | 653 |
| v0_32_3-tests-misc-programs-lamports | overflow | 3,549 | 3,614 | 65 |
| v0_32_3-tests-misc-programs-lamports | transfer | 6,977 | 7,786 | 809 |
| v0_32_3-tests-chat-programs-chat | create_chat_room | 905 | 1,019 | 114 |
| v0_32_3-tests-chat-programs-chat | create_user | 4,513 | 4,845 | 332 |
| v0_32_3-tests-chat-programs-chat | send_message | 3,102 | 3,180 | 78 |
| v0_32_3-tests-declare-program-programs-external | test_compilation_data_as_parameter_name | 180 | 196 | 16 |
| v1_2_2-tests-chat-programs-chat | create_chat_room | 934 | 1,067 | 133 |
| v1_2_2-tests-chat-programs-chat | create_user | 4,498 | 4,835 | 337 |
| v1_2_2-tests-chat-programs-chat | send_message | 3,090 | 3,180 | 90 |
| v1_2_2-examples-tutorial-basic-5-programs-basic-5 | create | 5,257 | 5,277 | 20 |
| v1_2_2-examples-tutorial-basic-5-programs-basic-5 | jump | 381 | 393 | 12 |
| v1_2_2-examples-tutorial-basic-5-programs-basic-5 | run | 383 | 395 | 12 |
| v1_2_2-examples-tutorial-basic-5-programs-basic-5 | walk | 384 | 394 | 10 |
| v1_2_2-tests-accountloader-realloc-programs-accountloader-realloc | initialize_legacy | 4,961 | 5,024 | 63 |
| v1_2_2-tests-accountloader-realloc-programs-accountloader-realloc | migrate | 5,836 | 6,199 | 363 |
| v1_2_2-tests-accountloader-realloc-programs-accountloader-realloc | read | 2,024 | 2,059 | 35 |
| v1_2_2-tests-accountloader-realloc-programs-accountloader-realloc | read_extra | 3,524 | 3,558 | 34 |
| v1_2_2-tests-accountloader-realloc-programs-accountloader-realloc | shrink | 4,294 | 4,655 | 361 |
| v1_2_2-tests-custom-coder-programs-native-system | advance_nonce_account | 250 | 738 | 488 |
| v1_2_2-tests-custom-coder-programs-native-system | allocate_with_seed | 498 | 767 | 269 |
| v1_2_2-tests-custom-coder-programs-native-system | assign | 167 | 169 | 2 |
| v1_2_2-tests-custom-coder-programs-native-system | assign_with_seed | 487 | 766 | 279 |
| v1_2_2-tests-custom-coder-programs-native-system | authorize_nonce_account | 226 | 501 | 275 |
| v1_2_2-tests-custom-coder-programs-native-system | create_account | 201 | 205 | 4 |
| v1_2_2-tests-custom-coder-programs-native-system | create_account_with_seed | 510 | 810 | 300 |
| v1_2_2-tests-custom-coder-programs-native-system | initialize_nonce_account | 328 | 532 | 204 |
| v1_2_2-tests-custom-coder-programs-native-system | transfer | 193 | 504 | 311 |
| v1_2_2-tests-custom-coder-programs-native-system | transfer_with_seed | 517 | 1,080 | 563 |
| v1_2_2-tests-custom-coder-programs-native-system | withdraw_nonce_account | 825 | 1,478 | 653 |
| v1_2_2-tests-optional-programs-optional | realloc | 3,768 | 4,580 | 812 |
| v1_2_2-tests-optional-programs-optional | update | 2,553 | 2,842 | 289 |
| v0_32_3-tests-custom-coder-programs-native-system | advance_nonce_account | 251 | 738 | 487 |
| v0_32_3-tests-custom-coder-programs-native-system | allocate_with_seed | 499 | 767 | 268 |
| v0_32_3-tests-custom-coder-programs-native-system | assign_with_seed | 490 | 768 | 278 |
| v0_32_3-tests-custom-coder-programs-native-system | authorize_nonce_account | 227 | 500 | 273 |
| v0_32_3-tests-custom-coder-programs-native-system | create_account_with_seed | 513 | 811 | 298 |
| v0_32_3-tests-custom-coder-programs-native-system | initialize_nonce_account | 329 | 531 | 202 |
| v0_32_3-tests-custom-coder-programs-native-system | transfer | 207 | 509 | 302 |
| v0_32_3-tests-custom-coder-programs-native-system | transfer_with_seed | 516 | 1,078 | 562 |
| v0_32_3-tests-custom-coder-programs-native-system | withdraw_nonce_account | 825 | 1,478 | 653 |
| v0_32_3-tests-optional-programs-optional | realloc | 3,747 | 4,645 | 898 |
| v0_32_3-tests-optional-programs-optional | update | 2,633 | 2,809 | 176 |
| v0_32_3-tests-zero-copy-programs-zero-copy | create_bar | 6,994 | 7,438 | 444 |
| v0_32_3-tests-zero-copy-programs-zero-copy | create_foo | 663 | 905 | 242 |
| v0_32_3-tests-zero-copy-programs-zero-copy | update_foo | 486 | 587 | 101 |
| v0_32_3-tests-zero-copy-programs-zero-copy | update_large_account | 500 | 715 | 215 |
| v0_32_3-tests-errors-programs-errors | account_not_initialized_error | 227 | 257 | 30 |
| v0_32_3-tests-errors-programs-errors | account_owned_by_wrong_program_error | 227 | 257 | 30 |
| v0_32_3-tests-errors-programs-errors | has_one_error | 676 | 836 | 160 |
| v0_32_3-tests-errors-programs-errors | mut_error | 195 | 479 | 284 |
| v0_32_3-tests-errors-programs-errors | require_keys_neq | 206 | 219 | 13 |
| v0_32_3-tests-errors-programs-errors | require_keys_neq_default_error | 210 | 223 | 13 |
| v0_32_3-tests-errors-programs-errors | signer_error | 195 | 199 | 4 |
| v1_2_2-tests-errors-programs-errors | account_not_initialized_error | 227 | 257 | 30 |
| v1_2_2-tests-errors-programs-errors | account_owned_by_wrong_program_error | 225 | 255 | 30 |
| v1_2_2-tests-errors-programs-errors | has_one_error | 676 | 864 | 188 |
| v1_2_2-tests-errors-programs-errors | mut_error | 193 | 477 | 284 |
| v1_2_2-tests-errors-programs-errors | require_keys_neq | 206 | 219 | 13 |
| v1_2_2-tests-errors-programs-errors | require_keys_neq_default_error | 210 | 223 | 13 |
| v1_2_2-tests-errors-programs-errors | signer_error | 193 | 197 | 4 |
| v1_2_2-tests-duplicate-mutable-accounts-programs-duplicate-mutable-accounts | allows_duplicate_readonly | 294 | 324 | 30 |
| v1_2_2-tests-duplicate-mutable-accounts-programs-duplicate-mutable-accounts | use_remaining_accounts | 882 | 912 | 30 |
| v1_2_2-tests-tictactoe-programs-tictactoe | initialize | 1,261 | 1,596 | 335 |
| v1_2_2-tests-declare-program-programs-external | init | 4,924 | 5,011 | 87 |
| v1_2_2-tests-declare-program-programs-external | test_compilation_data_as_parameter_name | 201 | 230 | 29 |
| v1_2_2-tests-declare-program-programs-external | test_compilation_defined_type_param | 196 | 203 | 7 |
| v1_2_2-tests-declare-program-programs-external | test_compilation_generic_types | 314 | 327 | 13 |
| v1_2_2-tests-declare-program-programs-external | test_compilation_initialize_with_token_2022 | 192 | 201 | 9 |
| v1_2_2-tests-declare-program-programs-external | test_compilation_packed_account | 223 | 260 | 37 |
| v1_2_2-tests-declare-program-programs-external | test_compilation_return_type | 313 | 324 | 11 |
| v0_31_1-tests-custom-coder-programs-native-system | advance_nonce_account | 251 | 738 | 487 |
| v0_31_1-tests-custom-coder-programs-native-system | allocate_with_seed | 499 | 767 | 268 |
| v0_31_1-tests-custom-coder-programs-native-system | assign_with_seed | 490 | 768 | 278 |
| v0_31_1-tests-custom-coder-programs-native-system | authorize_nonce_account | 227 | 500 | 273 |
| v0_31_1-tests-custom-coder-programs-native-system | create_account_with_seed | 513 | 811 | 298 |
| v0_31_1-tests-custom-coder-programs-native-system | initialize_nonce_account | 329 | 531 | 202 |
| v0_31_1-tests-custom-coder-programs-native-system | transfer | 207 | 509 | 302 |
| v0_31_1-tests-custom-coder-programs-native-system | transfer_with_seed | 516 | 1,078 | 562 |
| v0_31_1-tests-custom-coder-programs-native-system | withdraw_nonce_account | 825 | 1,478 | 653 |
| v0_31_1-tests-optional-programs-optional | realloc | 3,747 | 4,645 | 898 |
| v0_31_1-tests-optional-programs-optional | update | 2,633 | 2,809 | 176 |
| v0_31_1-tests-zero-copy-programs-zero-copy | create_bar | 6,994 | 7,438 | 444 |
| v0_31_1-tests-zero-copy-programs-zero-copy | create_foo | 663 | 905 | 242 |
| v0_31_1-tests-zero-copy-programs-zero-copy | update_foo | 486 | 587 | 101 |
| v0_31_1-tests-zero-copy-programs-zero-copy | update_large_account | 500 | 715 | 215 |
| v0_31_1-tests-spl-token-proxy-programs-token-proxy | proxy_burn | 3,674 | 4,114 | 440 |
| v0_31_1-tests-spl-token-proxy-programs-token-proxy | proxy_create_associated_token_account | 22,376 | 22,878 | 502 |
| v0_31_1-tests-spl-token-proxy-programs-token-proxy | proxy_create_mint | 10,751 | 10,792 | 41 |
| v0_31_1-tests-spl-token-proxy-programs-token-proxy | proxy_create_token_account | 8,480 | 9,120 | 640 |
| v0_31_1-tests-spl-token-proxy-programs-token-proxy | proxy_mint_to | 4,821 | 5,257 | 436 |
| v0_31_1-tests-spl-token-proxy-programs-token-proxy | proxy_optional_transfer | 4,597 | 5,312 | 715 |
| v0_31_1-tests-spl-token-proxy-programs-token-proxy | proxy_set_authority | 3,073 | 3,744 | 671 |
| v0_31_1-tests-spl-token-proxy-programs-token-proxy | proxy_transfer | 4,937 | 5,313 | 376 |
| v1_2_2-tests-cpi-returns-programs-caller | cpi_call_no_accounts | 1,597 | 1,615 | 18 |
| v1_2_2-tests-cpi-returns-programs-caller | cpi_call_return_struct | 3,340 | 3,440 | 100 |
| v1_2_2-tests-cpi-returns-programs-caller | cpi_call_return_u64 | 3,374 | 3,474 | 100 |
| v1_2_2-tests-cpi-returns-programs-caller | cpi_call_return_u64_same_program_spoofed | 6,030 | 6,171 | 141 |
| v1_2_2-tests-cpi-returns-programs-caller | cpi_call_return_u64_spoofed | 6,233 | 6,327 | 94 |
| v1_2_2-tests-cpi-returns-programs-caller | cpi_call_return_u64_spoofed_preserved | 6,084 | 6,178 | 94 |
| v1_2_2-tests-cpi-returns-programs-caller | cpi_call_return_vec | 3,501 | 3,606 | 105 |
| v1_2_2-tests-cpi-returns-programs-caller | return_vec | 324 | 334 | 10 |
| v1_2_2-tests-spl-token-proxy-programs-token-proxy | proxy_burn | 3,680 | 4,100 | 420 |
| v1_2_2-tests-spl-token-proxy-programs-token-proxy | proxy_create_associated_token_account | 22,347 | 23,143 | 796 |
| v1_2_2-tests-spl-token-proxy-programs-token-proxy | proxy_create_mint | 10,742 | 10,862 | 120 |
| v1_2_2-tests-spl-token-proxy-programs-token-proxy | proxy_create_token_account | 8,462 | 9,219 | 757 |
| v1_2_2-tests-spl-token-proxy-programs-token-proxy | proxy_mint_to | 4,829 | 5,245 | 416 |
| v1_2_2-tests-spl-token-proxy-programs-token-proxy | proxy_optional_transfer | 4,538 | 5,353 | 815 |
| v1_2_2-tests-spl-token-proxy-programs-token-proxy | proxy_set_authority | 3,041 | 3,632 | 591 |
| v1_2_2-tests-spl-token-proxy-programs-token-proxy | proxy_transfer | 5,133 | 5,641 | 508 |
| v1_2_2-tests-spl-token-wrapper-programs-token-wrapper | initialize | 33,511 | 36,290 | 2,779 |
| v1_2_2-tests-misc-programs-misc-optional | init_if_needed_checks_rent_exemption | 867 | 1,432 | 565 |
| v1_2_2-tests-misc-programs-misc-optional | initialize_no_rent_exempt | 280 | 464 | 184 |
| v1_2_2-tests-misc-programs-misc-optional | test_associated_token_with_token_program_constraint | 1,112 | 1,229 | 117 |
| v1_2_2-tests-misc-programs-misc-optional | test_empty_seeds_constraint | 1,836 | 2,013 | 177 |
| v1_2_2-tests-misc-programs-misc-optional | test_enforce_rent_exempt | 461 | 791 | 330 |
| v1_2_2-tests-misc-programs-misc-optional | test_init_associated_token | 19,615 | 20,291 | 676 |
| v1_2_2-tests-misc-programs-misc-optional | test_init_associated_token_if_needed | 21,507 | 22,389 | 882 |
| v1_2_2-tests-misc-programs-misc-optional | test_init_associated_token_if_needed_with_token_program | 1,881 | 2,016 | 135 |
| v1_2_2-tests-misc-programs-misc-optional | test_init_if_needed_checks_owner | 3,875 | 4,069 | 194 |
| v1_2_2-tests-misc-programs-misc-optional | test_init_many_associated_token_accounts | 119,998 | 122,567 | 2,569 |
| v1_2_2-tests-misc-programs-misc-optional | test_init_mint_if_needed | 1,158 | 1,447 | 289 |
| v1_2_2-tests-misc-programs-misc-optional | test_init_mint_if_needed_with_token_program | 1,125 | 1,654 | 529 |
| v1_2_2-tests-misc-programs-misc-optional | test_init_mint_with_token_program | 902 | 935 | 33 |
| v1_2_2-tests-misc-programs-misc-optional | test_init_token | 5,381 | 5,951 | 570 |
| v1_2_2-tests-misc-programs-misc-optional | test_init_token_if_needed_with_token_program | 1,540 | 1,761 | 221 |
| v1_2_2-tests-misc-programs-misc-optional | test_init_token_with_token_program | 1,344 | 1,428 | 84 |
| v1_2_2-tests-misc-programs-misc-optional | test_instruction_constraint | 1,990 | 2,305 | 315 |
| v1_2_2-tests-misc-programs-misc-optional | test_mint_constraint | 643 | 867 | 224 |
| v1_2_2-tests-misc-programs-misc-optional | test_mint_miss_mint_auth_constraint | 524 | 566 | 42 |
| v1_2_2-tests-misc-programs-misc-optional | test_mint_only_auth_constraint | 625 | 855 | 230 |
| v1_2_2-tests-misc-programs-misc-optional | test_mint_only_one_auth_constraint | 503 | 553 | 50 |
| v1_2_2-tests-misc-programs-misc-optional | test_mint_only_token_program_constraint | 505 | 555 | 50 |
| v1_2_2-tests-misc-programs-misc-optional | test_no_rent_exempt | 292 | 466 | 174 |
| v1_2_2-tests-misc-programs-misc-optional | test_only_token_program_constraint | 551 | 553 | 2 |
| v1_2_2-tests-misc-programs-misc-optional | test_owner | 408 | 544 | 136 |
| v1_2_2-tests-misc-programs-misc-optional | test_pda_init_zero_copy | 5,415 | 5,459 | 44 |
| v1_2_2-tests-misc-programs-misc-optional | test_pda_mut_zero_copy | 2,242 | 2,495 | 253 |
| v1_2_2-tests-misc-programs-misc-optional | test_program_id_constraint | 3,538 | 3,860 | 322 |
| v1_2_2-tests-misc-programs-misc-optional | test_program_id_constraint_find_pda | 3,526 | 3,847 | 321 |
| v1_2_2-tests-misc-programs-misc-optional | test_simulate | 1,408 | 1,417 | 9 |
| v1_2_2-tests-misc-programs-misc | init_if_needed_checks_rent_exemption | 1,922 | 2,621 | 699 |
| v1_2_2-tests-misc-programs-misc | initialize_no_rent_exempt | 165 | 383 | 218 |
| v1_2_2-tests-misc-programs-misc | test_associated_constraint | 4,067 | 4,286 | 219 |
| v1_2_2-tests-misc-programs-misc | test_boxed_owner_constraint | 422 | 648 | 226 |
| v1_2_2-tests-misc-programs-misc | test_empty_seeds_constraint | 3,217 | 3,429 | 212 |
| v1_2_2-tests-misc-programs-misc | test_enforce_rent_exempt | 338 | 712 | 374 |
| v1_2_2-tests-misc-programs-misc | test_executable | 196 | 481 | 285 |
| v1_2_2-tests-misc-programs-misc | test_init_associated_token | 19,410 | 20,174 | 764 |
| v1_2_2-tests-misc-programs-misc | test_init_associated_token_if_needed | 21,216 | 22,242 | 1,026 |
| v1_2_2-tests-misc-programs-misc | test_init_if_needed_checks_owner | 8,320 | 8,522 | 202 |
| v1_2_2-tests-misc-programs-misc | test_init_many_associated_token_accounts | 119,998 | 122,565 | 2,567 |
| v1_2_2-tests-misc-programs-misc | test_init_token | 5,187 | 5,844 | 657 |
| v1_2_2-tests-misc-programs-misc | test_no_rent_exempt | 194 | 383 | 189 |
| v1_2_2-tests-misc-programs-misc | test_pda_init_zero_copy | 3,606 | 3,797 | 191 |
| v1_2_2-tests-misc-programs-misc | test_pda_mut_zero_copy | 2,048 | 2,395 | 347 |
| v1_2_2-tests-misc-programs-misc | test_program_id_constraint | 3,306 | 3,762 | 456 |
| v1_2_2-tests-misc-programs-misc | test_program_id_constraint_find_pda | 3,282 | 3,736 | 454 |
| v1_2_2-tests-misc-programs-misc | test_simulate | 1,410 | 1,417 | 7 |
| v0_32_3-tests-spl-token-proxy-programs-token-proxy | proxy_burn | 3,673 | 4,113 | 440 |
| v0_32_3-tests-spl-token-proxy-programs-token-proxy | proxy_create_associated_token_account | 22,376 | 22,878 | 502 |
| v0_32_3-tests-spl-token-proxy-programs-token-proxy | proxy_create_mint | 10,752 | 10,793 | 41 |
| v0_32_3-tests-spl-token-proxy-programs-token-proxy | proxy_create_token_account | 8,482 | 9,122 | 640 |
| v0_32_3-tests-spl-token-proxy-programs-token-proxy | proxy_mint_to | 4,822 | 5,258 | 436 |
| v0_32_3-tests-spl-token-proxy-programs-token-proxy | proxy_optional_transfer | 4,597 | 5,312 | 715 |
| v0_32_3-tests-spl-token-proxy-programs-token-proxy | proxy_set_authority | 3,075 | 3,746 | 671 |
| v0_32_3-tests-spl-token-proxy-programs-token-proxy | proxy_transfer | 4,937 | 5,313 | 376 |
| v0_32_3-tests-spl-token-wrapper-programs-token-wrapper | initialize | 33,618 | 35,688 | 2,070 |
| v0_32_3-tests-misc-programs-misc-optional | init_if_needed_checks_rent_exemption | 865 | 1,432 | 567 |
| v0_32_3-tests-misc-programs-misc-optional | initialize_no_rent_exempt | 281 | 464 | 183 |
| v0_32_3-tests-misc-programs-misc-optional | test_associated_token_with_token_program_constraint | 1,102 | 1,194 | 92 |
| v0_32_3-tests-misc-programs-misc-optional | test_empty_seeds_constraint | 1,860 | 2,036 | 176 |
| v0_32_3-tests-misc-programs-misc-optional | test_enforce_rent_exempt | 462 | 791 | 329 |
| v0_32_3-tests-misc-programs-misc-optional | test_init_associated_token | 19,713 | 20,132 | 419 |
| v0_32_3-tests-misc-programs-misc-optional | test_init_associated_token_if_needed | 21,555 | 22,233 | 678 |
| v0_32_3-tests-misc-programs-misc-optional | test_init_associated_token_if_needed_with_token_program | 1,746 | 1,942 | 196 |
| v0_32_3-tests-misc-programs-misc-optional | test_init_if_needed_checks_owner | 3,895 | 4,097 | 202 |
| v0_32_3-tests-misc-programs-misc-optional | test_init_many_associated_token_accounts | 120,465 | 121,692 | 1,227 |
| v0_32_3-tests-misc-programs-misc-optional | test_init_mint_if_needed | 1,152 | 1,426 | 274 |
| v0_32_3-tests-misc-programs-misc-optional | test_init_mint_if_needed_with_token_program | 1,120 | 1,637 | 517 |
| v0_32_3-tests-misc-programs-misc-optional | test_init_mint_with_token_program | 903 | 927 | 24 |
| v0_32_3-tests-misc-programs-misc-optional | test_init_token | 5,470 | 5,925 | 455 |
| v0_32_3-tests-misc-programs-misc-optional | test_init_token_if_needed | 1,496 | 1,538 | 42 |
| v0_32_3-tests-misc-programs-misc-optional | test_init_token_if_needed_with_token_program | 1,479 | 1,763 | 284 |
| v0_32_3-tests-misc-programs-misc-optional | test_init_token_with_token_program | 1,360 | 1,440 | 80 |
| v0_32_3-tests-misc-programs-misc-optional | test_instruction_constraint | 2,037 | 2,349 | 312 |
| v0_32_3-tests-misc-programs-misc-optional | test_mint_constraint | 643 | 865 | 222 |
| v0_32_3-tests-misc-programs-misc-optional | test_mint_miss_mint_auth_constraint | 525 | 566 | 41 |
| v0_32_3-tests-misc-programs-misc-optional | test_mint_only_auth_constraint | 626 | 853 | 227 |
| v0_32_3-tests-misc-programs-misc-optional | test_mint_only_one_auth_constraint | 504 | 555 | 51 |
| v0_32_3-tests-misc-programs-misc-optional | test_mint_only_token_program_constraint | 506 | 557 | 51 |
| v0_32_3-tests-misc-programs-misc-optional | test_no_rent_exempt | 293 | 466 | 173 |
| v0_32_3-tests-misc-programs-misc-optional | test_only_token_program_constraint | 528 | 553 | 25 |
| v0_32_3-tests-misc-programs-misc-optional | test_owner | 409 | 543 | 134 |
| v0_32_3-tests-misc-programs-misc-optional | test_pda_init_zero_copy | 5,423 | 5,448 | 25 |
| v0_32_3-tests-misc-programs-misc-optional | test_pda_mut_zero_copy | 2,269 | 2,497 | 228 |
| v0_32_3-tests-misc-programs-misc-optional | test_program_id_constraint | 3,621 | 3,942 | 321 |
| v0_32_3-tests-misc-programs-misc-optional | test_program_id_constraint_find_pda | 3,569 | 3,895 | 326 |
| v0_32_3-tests-misc-programs-misc-optional | test_simulate | 1,409 | 1,417 | 8 |
| v0_32_3-tests-misc-programs-misc | init_if_needed_checks_rent_exemption | 1,924 | 2,641 | 717 |
| v0_32_3-tests-misc-programs-misc | initialize_no_rent_exempt | 165 | 382 | 217 |
| v0_32_3-tests-misc-programs-misc | test_associated_constraint | 4,224 | 4,410 | 186 |
| v0_32_3-tests-misc-programs-misc | test_boxed_owner_constraint | 430 | 647 | 217 |
| v0_32_3-tests-misc-programs-misc | test_empty_seeds_constraint | 3,240 | 3,452 | 212 |
| v0_32_3-tests-misc-programs-misc | test_enforce_rent_exempt | 337 | 711 | 374 |
| v0_32_3-tests-misc-programs-misc | test_executable | 196 | 480 | 284 |
| v0_32_3-tests-misc-programs-misc | test_init_associated_token | 19,509 | 20,018 | 509 |
| v0_32_3-tests-misc-programs-misc | test_init_associated_token_if_needed | 21,335 | 22,091 | 756 |
| v0_32_3-tests-misc-programs-misc | test_init_if_needed_checks_owner | 8,344 | 8,550 | 206 |
| v0_32_3-tests-misc-programs-misc | test_init_many_associated_token_accounts | 120,464 | 121,690 | 1,226 |
| v0_32_3-tests-misc-programs-misc | test_init_token | 5,271 | 5,821 | 550 |
| v0_32_3-tests-misc-programs-misc | test_no_rent_exempt | 194 | 382 | 188 |
| v0_32_3-tests-misc-programs-misc | test_pda_init_zero_copy | 3,609 | 3,786 | 177 |
| v0_32_3-tests-misc-programs-misc | test_pda_mut_zero_copy | 2,107 | 2,423 | 316 |
| v0_32_3-tests-misc-programs-misc | test_program_id_constraint | 3,388 | 3,842 | 454 |
| v0_32_3-tests-misc-programs-misc | test_program_id_constraint_find_pda | 3,328 | 3,781 | 453 |
| v0_32_3-tests-misc-programs-misc | test_simulate | 1,410 | 1,417 | 7 |
| v1_2_2-tests-bench-programs-bench | account_empty1 | 232 | 262 | 30 |
| v1_2_2-tests-bench-programs-bench | account_empty_init1 | 1,930 | 1,981 | 51 |
| v1_2_2-tests-bench-programs-bench | account_info1 | 165 | 383 | 218 |
| v1_2_2-tests-bench-programs-bench | account_info2 | 189 | 634 | 445 |
| v1_2_2-tests-bench-programs-bench | account_info4 | 591 | 1,218 | 627 |
| v1_2_2-tests-bench-programs-bench | account_info8 | 992 | 2,618 | 1,626 |
| v1_2_2-tests-bench-programs-bench | interface1 | 224 | 231 | 7 |
| v1_2_2-tests-bench-programs-bench | program1 | 218 | 226 | 8 |
| v1_2_2-tests-bench-programs-bench | program2 | 247 | 255 | 8 |
| v1_2_2-tests-bench-programs-bench | signer1 | 206 | 210 | 4 |
| v1_2_2-tests-bench-programs-bench | signer2 | 238 | 244 | 6 |
| v1_2_2-tests-bench-programs-bench | unchecked_account1 | 208 | 209 | 1 |
| v0_31_1-tests-spl-token-wrapper-programs-token-wrapper | initialize | 33,612 | 35,682 | 2,070 |
| v0_31_1-tests-misc-programs-misc-optional | init_if_needed_checks_rent_exemption | 865 | 1,432 | 567 |
| v0_31_1-tests-misc-programs-misc-optional | initialize_no_rent_exempt | 281 | 464 | 183 |
| v0_31_1-tests-misc-programs-misc-optional | test_associated_token_with_token_program_constraint | 1,102 | 1,194 | 92 |
| v0_31_1-tests-misc-programs-misc-optional | test_empty_seeds_constraint | 1,860 | 2,036 | 176 |
| v0_31_1-tests-misc-programs-misc-optional | test_enforce_rent_exempt | 462 | 791 | 329 |
| v0_31_1-tests-misc-programs-misc-optional | test_init_associated_token | 19,713 | 20,132 | 419 |
| v0_31_1-tests-misc-programs-misc-optional | test_init_associated_token_if_needed | 21,555 | 22,233 | 678 |
| v0_31_1-tests-misc-programs-misc-optional | test_init_associated_token_if_needed_with_token_program | 1,746 | 1,942 | 196 |
| v0_31_1-tests-misc-programs-misc-optional | test_init_if_needed_checks_owner | 3,895 | 4,097 | 202 |
| v0_31_1-tests-misc-programs-misc-optional | test_init_many_associated_token_accounts | 120,464 | 121,691 | 1,227 |
| v0_31_1-tests-misc-programs-misc-optional | test_init_mint_if_needed | 1,152 | 1,426 | 274 |
| v0_31_1-tests-misc-programs-misc-optional | test_init_mint_if_needed_with_token_program | 1,120 | 1,637 | 517 |
| v0_31_1-tests-misc-programs-misc-optional | test_init_mint_with_token_program | 903 | 927 | 24 |
| v0_31_1-tests-misc-programs-misc-optional | test_init_token | 5,468 | 5,923 | 455 |
| v0_31_1-tests-misc-programs-misc-optional | test_init_token_if_needed | 1,496 | 1,538 | 42 |
| v0_31_1-tests-misc-programs-misc-optional | test_init_token_if_needed_with_token_program | 1,479 | 1,763 | 284 |
| v0_31_1-tests-misc-programs-misc-optional | test_init_token_with_token_program | 1,360 | 1,440 | 80 |
| v0_31_1-tests-misc-programs-misc-optional | test_instruction_constraint | 2,037 | 2,349 | 312 |
| v0_31_1-tests-misc-programs-misc-optional | test_mint_constraint | 643 | 865 | 222 |
| v0_31_1-tests-misc-programs-misc-optional | test_mint_miss_mint_auth_constraint | 525 | 566 | 41 |
| v0_31_1-tests-misc-programs-misc-optional | test_mint_only_auth_constraint | 626 | 853 | 227 |
| v0_31_1-tests-misc-programs-misc-optional | test_mint_only_one_auth_constraint | 504 | 555 | 51 |
| v0_31_1-tests-misc-programs-misc-optional | test_mint_only_token_program_constraint | 506 | 557 | 51 |
| v0_31_1-tests-misc-programs-misc-optional | test_no_rent_exempt | 293 | 466 | 173 |
| v0_31_1-tests-misc-programs-misc-optional | test_only_token_program_constraint | 528 | 553 | 25 |
| v0_31_1-tests-misc-programs-misc-optional | test_owner | 409 | 543 | 134 |
| v0_31_1-tests-misc-programs-misc-optional | test_pda_init_zero_copy | 5,423 | 5,448 | 25 |
| v0_31_1-tests-misc-programs-misc-optional | test_pda_mut_zero_copy | 2,269 | 2,497 | 228 |
| v0_31_1-tests-misc-programs-misc-optional | test_program_id_constraint | 3,621 | 3,942 | 321 |
| v0_31_1-tests-misc-programs-misc-optional | test_program_id_constraint_find_pda | 3,569 | 3,895 | 326 |
| v0_31_1-tests-misc-programs-misc-optional | test_simulate | 1,409 | 1,417 | 8 |
| v0_31_1-tests-misc-programs-misc | init_if_needed_checks_rent_exemption | 1,924 | 2,641 | 717 |
| v0_31_1-tests-misc-programs-misc | initialize_no_rent_exempt | 165 | 382 | 217 |
| v0_31_1-tests-misc-programs-misc | test_associated_constraint | 4,224 | 4,410 | 186 |
| v0_31_1-tests-misc-programs-misc | test_boxed_owner_constraint | 430 | 647 | 217 |
| v0_31_1-tests-misc-programs-misc | test_empty_seeds_constraint | 3,240 | 3,452 | 212 |
| v0_31_1-tests-misc-programs-misc | test_enforce_rent_exempt | 337 | 711 | 374 |
| v0_31_1-tests-misc-programs-misc | test_executable | 196 | 480 | 284 |
| v0_31_1-tests-misc-programs-misc | test_init_associated_token | 19,509 | 20,018 | 509 |
| v0_31_1-tests-misc-programs-misc | test_init_associated_token_if_needed | 21,335 | 22,091 | 756 |
| v0_31_1-tests-misc-programs-misc | test_init_if_needed_checks_owner | 8,344 | 8,550 | 206 |
| v0_31_1-tests-misc-programs-misc | test_init_many_associated_token_accounts | 120,463 | 121,689 | 1,226 |
| v0_31_1-tests-misc-programs-misc | test_init_token | 5,269 | 5,819 | 550 |
| v0_31_1-tests-misc-programs-misc | test_no_rent_exempt | 194 | 382 | 188 |
| v0_31_1-tests-misc-programs-misc | test_pda_init_zero_copy | 3,609 | 3,786 | 177 |
| v0_31_1-tests-misc-programs-misc | test_pda_mut_zero_copy | 2,107 | 2,423 | 316 |
| v0_31_1-tests-misc-programs-misc | test_program_id_constraint | 3,388 | 3,842 | 454 |
| v0_31_1-tests-misc-programs-misc | test_program_id_constraint_find_pda | 3,328 | 3,781 | 453 |
| v0_31_1-tests-misc-programs-misc | test_simulate | 1,410 | 1,417 | 7 |
| v0_31_1-tests-bench-programs-bench | account_empty1 | 232 | 261 | 29 |
| v0_31_1-tests-bench-programs-bench | account_empty_init1 | 3,075 | 3,105 | 30 |
| v0_31_1-tests-bench-programs-bench | account_info1 | 165 | 382 | 217 |
| v0_31_1-tests-bench-programs-bench | account_info2 | 189 | 638 | 449 |
| v0_31_1-tests-bench-programs-bench | account_info4 | 591 | 1,221 | 630 |
| v0_31_1-tests-bench-programs-bench | account_info8 | 992 | 2,618 | 1,626 |
| v0_31_1-tests-bench-programs-bench | interface1 | 224 | 230 | 6 |
| v0_31_1-tests-bench-programs-bench | program1 | 218 | 225 | 7 |
| v0_31_1-tests-bench-programs-bench | program2 | 247 | 254 | 7 |
| v0_31_1-tests-bench-programs-bench | signer1 | 206 | 209 | 3 |
| v0_31_1-tests-bench-programs-bench | signer2 | 238 | 243 | 5 |
| v0_31_1-tests-bench-programs-bench | unchecked_account1 | 204 | 206 | 2 |
| v0_32_3-tests-bench-programs-bench | account_empty1 | 232 | 261 | 29 |
| v0_32_3-tests-bench-programs-bench | account_empty_init1 | 3,075 | 3,105 | 30 |
| v0_32_3-tests-bench-programs-bench | account_info1 | 165 | 382 | 217 |
| v0_32_3-tests-bench-programs-bench | account_info2 | 189 | 638 | 449 |
| v0_32_3-tests-bench-programs-bench | account_info4 | 591 | 1,221 | 630 |
| v0_32_3-tests-bench-programs-bench | account_info8 | 992 | 2,618 | 1,626 |
| v0_32_3-tests-bench-programs-bench | interface1 | 224 | 230 | 6 |
| v0_32_3-tests-bench-programs-bench | program1 | 218 | 225 | 7 |
| v0_32_3-tests-bench-programs-bench | program2 | 247 | 254 | 7 |
| v0_32_3-tests-bench-programs-bench | signer1 | 206 | 209 | 3 |
| v0_32_3-tests-bench-programs-bench | signer2 | 238 | 243 | 5 |
| v0_32_3-tests-bench-programs-bench | unchecked_account1 | 204 | 206 | 2 |

## Instructions without successful inputs

136 instruction rows remain unmeasured. Categories: adaptive_fee_scenario_missing=8, additional_token_program_scenario_missing=14, dynamic_tick_array_scenario_missing=1, fixed_production_authority_unavailable=1, generated_state_missing=35, locked_position_scenario_missing=2, position_bundle_scenario_missing=5, position_range_scenario_missing=1, production_admin_signature=2, protocol_fee_scenario_missing=1, reward_vault_scenario_missing=5, second_pool_scenario_missing=2, source_constraints_impossible=3, token_badge_scenario_missing=3, unconditional_source_error=1, unreachable_by_source=51, upstream_source_idl_mismatch=1.

The source-resolved misc scenarios retain the entire original corpus and add client-derived account relationships, explicit signer/payer roles, rent-plus-debit funding, and real SPL mint setup. The targeted controls show 39 former gaps gaining successful inputs; the final rows above remain the measurement authority. The three fixed-bump `test_used_identifiers` rows require an on-curve bump4 address under fixed argument constraints; their exact source and native derivation evidence classify them as source-impossible. All failure inputs remain in the proof corpus.

[Per-instruction reasons and source evidence](round-four-unmeasured.json) distinguish source failures, upstream IDL mismatches, and unresolved generated state. [Generator control evidence](gaps/round-four/scenario-fix/summary.json) retains the old self-close defect and corrected comparisons.

## Not built

| Program | Anchor | Construct or observed failure |
|---|---|---|
| v0_31_1-tests-custom-discriminator-programs-ambiguous-discriminator | 0.31.1 | stock IDL validation; [transcript](transcripts/v0_31_1-tests-custom-discriminator-programs-ambiguous-discriminator.txt) |
| v0_31_1-tests-bpf-upgradeable-state-programs-bpf-upgradeable-state | 0.31.1 | generated corpus account/type resolution; [transcript](transcripts/v0_31_1-tests-bpf-upgradeable-state-programs-bpf-upgradeable-state.txt) |
| v0_31_1-tests-lazy-account-programs-lazy-account | 0.31.1 | LazyAccount; [transcript](transcripts/v0_31_1-tests-lazy-account-programs-lazy-account.txt) |
| v0_31_1-tests-declare-program-programs-declare-program | 0.31.1 | SBF stack frame > 4096 bytes; [transcript](transcripts/v0_31_1-tests-declare-program-programs-declare-program.txt) |
| v0_31_1-tests-tictactoe-programs-tictactoe | 0.31.1 | error[E0277]: the trait bound `anchor_lang::Error: From<ErrorCode>` is not satisfied; [transcript](transcripts/v0_31_1-tests-tictactoe-programs-tictactoe.txt) |
| v0_31_1-tests-pda-derivation-programs-pda-derivation | 0.31.1 | generated corpus account/type resolution; [transcript](transcripts/v0_31_1-tests-pda-derivation-programs-pda-derivation.txt) |
| v0_31_1-tests-multisig-programs-multisig | 0.31.1 | Pipeline incomplete: error: generated account resolver cannot sign sequence 'seed-1413567045-0' input 'change_threshold:short-instruction' (instructions: change_threshold; unresolved signer accounts: 84DbJDDHyFw9q8aRY7BP65k6oFB22Jr2xxcjvk1ReeYw): not enough signers; see transcripts/v0_31_1-tests-multisig-programs-multisig.txt.; [transcript](transcripts/v0_31_1-tests-multisig-programs-multisig.txt) |
| v0_31_1-tests-spl-token-extensions-programs-token-extensions | 0.31.1 | extensions::metadata_pointer::authority; [transcript](transcripts/v0_31_1-tests-spl-token-extensions-programs-token-extensions.txt) |
| v0_31_1-tests-idl-programs-new-idl | 0.31.1 | stock IDL validation; [transcript](transcripts/v0_31_1-tests-idl-programs-new-idl.txt) |
| v0_31_1-tests-swap-programs-swap | 0.31.1 | upstream dependency resolution; [transcript](transcripts/v0_31_1-tests-swap-programs-swap.txt) |
| v0_31_1-tests-lockup-programs-lockup | 0.31.1 | error[E0432]: unresolved import `anchor_lang::accounts::state`; [transcript](transcripts/v0_31_1-tests-lockup-programs-lockup.txt) |
| v0_31_1-tests-ido-pool-programs-ido-pool | 0.31.1 | generated corpus account/type resolution; [transcript](transcripts/v0_31_1-tests-ido-pool-programs-ido-pool.txt) |
| v0_31_1-tests-cfo-programs-cfo | 0.31.1 | upstream dependency resolution; [transcript](transcripts/v0_31_1-tests-cfo-programs-cfo.txt) |
| v0_31_1-tests-lockup-programs-registry | 0.31.1 | error[E0432]: unresolved import `anchor_lang::accounts::state`; [transcript](transcripts/v0_31_1-tests-lockup-programs-registry.txt) |
| v0_31_1-tests-auction-house-programs-auction-house | 0.31.1 | generated corpus account/type resolution; [transcript](transcripts/v0_31_1-tests-auction-house-programs-auction-house.txt) |
| v0_32_3-tests-custom-discriminator-programs-ambiguous-discriminator | 0.32.3 | stock IDL validation; [transcript](transcripts/v0_32_3-tests-custom-discriminator-programs-ambiguous-discriminator.txt) |
| v0_32_3-tests-bpf-upgradeable-state-programs-bpf-upgradeable-state | 0.32.3 | generated corpus account/type resolution; [transcript](transcripts/v0_32_3-tests-bpf-upgradeable-state-programs-bpf-upgradeable-state.txt) |
| v0_32_3-tests-lazy-account-programs-lazy-account | 0.32.3 | LazyAccount; [transcript](transcripts/v0_32_3-tests-lazy-account-programs-lazy-account.txt) |
| v0_32_3-tests-declare-program-programs-declare-program | 0.32.3 | SBF stack frame > 4096 bytes; [transcript](transcripts/v0_32_3-tests-declare-program-programs-declare-program.txt) |
| v0_32_3-tests-tictactoe-programs-tictactoe | 0.32.3 | error[E0277]: the trait bound `anchor_lang::Error: From<ErrorCode>` is not satisfied; [transcript](transcripts/v0_32_3-tests-tictactoe-programs-tictactoe.txt) |
| v0_32_3-tests-pda-derivation-programs-pda-derivation | 0.32.3 | generated corpus account/type resolution; [transcript](transcripts/v0_32_3-tests-pda-derivation-programs-pda-derivation.txt) |
| v0_32_3-tests-multisig-programs-multisig | 0.32.3 | Pipeline incomplete: error: generated account resolver cannot sign sequence 'seed-1413567045-0' input 'change_threshold:short-instruction' (instructions: change_threshold; unresolved signer accounts: 84DbJDDHyFw9q8aRY7BP65k6oFB22Jr2xxcjvk1ReeYw): not enough signers; see transcripts/v0_32_3-tests-multisig-programs-multisig.txt.; [transcript](transcripts/v0_32_3-tests-multisig-programs-multisig.txt) |
| v0_32_3-tests-spl-token-extensions-programs-token-extensions | 0.32.3 | extensions::metadata_pointer::authority; [transcript](transcripts/v0_32_3-tests-spl-token-extensions-programs-token-extensions.txt) |
| v0_32_3-tests-swap-programs-swap | 0.32.3 | upstream dependency resolution; [transcript](transcripts/v0_32_3-tests-swap-programs-swap.txt) |
| v0_32_3-tests-lockup-programs-lockup | 0.32.3 | error[E0432]: unresolved import `anchor_lang::accounts::state`; [transcript](transcripts/v0_32_3-tests-lockup-programs-lockup.txt) |
| v0_32_3-tests-ido-pool-programs-ido-pool | 0.32.3 | generated corpus account/type resolution; [transcript](transcripts/v0_32_3-tests-ido-pool-programs-ido-pool.txt) |
| v0_32_3-tests-cfo-programs-cfo | 0.32.3 | upstream dependency resolution; [transcript](transcripts/v0_32_3-tests-cfo-programs-cfo.txt) |
| v0_32_3-tests-lockup-programs-registry | 0.32.3 | error[E0432]: unresolved import `anchor_lang::accounts::state`; [transcript](transcripts/v0_32_3-tests-lockup-programs-registry.txt) |
| v0_32_3-tests-auction-house-programs-auction-house | 0.32.3 | generated corpus account/type resolution; [transcript](transcripts/v0_32_3-tests-auction-house-programs-auction-house.txt) |
| v1_2_2-tests-errors-programs-multiple-errors | 1.2.2 | stock IDL validation; [transcript](transcripts/v1_2_2-tests-errors-programs-multiple-errors.txt) |
| v1_2_2-tests-custom-discriminator-programs-zeroed-discriminator | 1.2.2 | stock all-zero discriminator rejection; [transcript](transcripts/v1_2_2-tests-custom-discriminator-programs-zeroed-discriminator.txt) |
| v1_2_2-tests-custom-discriminator-programs-ambiguous-discriminator | 1.2.2 | stock IDL validation; [transcript](transcripts/v1_2_2-tests-custom-discriminator-programs-ambiguous-discriminator.txt) |
| v1_2_2-tests-zero-copy-programs-zero-cpi | 1.2.2 | upstream dependency resolution; [transcript](transcripts/v1_2_2-tests-zero-copy-programs-zero-cpi.txt) |
| v1_2_2-tests-test-instruction-validation-fail-type-programs-test-instruction-validation | 1.2.2 | error[E0308]: mismatched types; [transcript](transcripts/v1_2_2-tests-test-instruction-validation-fail-type-programs-test-instruction-validation.txt) |
| v1_2_2-tests-test-instruction-validation-fail-args-count-programs-test-instruction-validation | 1.2.2 | stock instruction validation; [transcript](transcripts/v1_2_2-tests-test-instruction-validation-fail-args-count-programs-test-instruction-validation.txt) |
| v1_2_2-tests-interface-account-programs-interface-account | 1.2.2 | generated corpus account/type resolution; [transcript](transcripts/v1_2_2-tests-interface-account-programs-interface-account.txt) |
| v1_2_2-tests-bpf-upgradeable-state-programs-bpf-upgradeable-state | 1.2.2 | generated corpus account/type resolution; [transcript](transcripts/v1_2_2-tests-bpf-upgradeable-state-programs-bpf-upgradeable-state.txt) |
| v1_2_2-tests-lazy-account-programs-lazy-account | 1.2.2 | LazyAccount; [transcript](transcripts/v1_2_2-tests-lazy-account-programs-lazy-account.txt) |
| v1_2_2-tests-zero-copy-programs-zero-copy | 1.2.2 | upstream dependency resolution; [transcript](transcripts/v1_2_2-tests-zero-copy-programs-zero-copy.txt) |
| v1_2_2-tests-pda-derivation-programs-pda-derivation | 1.2.2 | generated corpus account/type resolution; [transcript](transcripts/v1_2_2-tests-pda-derivation-programs-pda-derivation.txt) |
| v1_2_2-tests-multisig-programs-multisig | 1.2.2 | Pipeline incomplete: error: generated account resolver cannot sign sequence 'seed-1413567045-0' input 'change_threshold:short-instruction' (instructions: change_threshold; unresolved signer accounts: 84DbJDDHyFw9q8aRY7BP65k6oFB22Jr2xxcjvk1ReeYw): not enough signers; see transcripts/v1_2_2-tests-multisig-programs-multisig.txt.; [transcript](transcripts/v1_2_2-tests-multisig-programs-multisig.txt) |
| v1_2_2-tests-idl-programs-idl | 1.2.2 | error[E0433]: cannot find `feature` in `solana_program`; [transcript](transcripts/v1_2_2-tests-idl-programs-idl.txt) |
| v1_2_2-tests-lockup-programs-lockup | 1.2.2 | error[E0432]: unresolved import `anchor_lang::accounts::state`; [transcript](transcripts/v1_2_2-tests-lockup-programs-lockup.txt) |
| v1_2_2-tests-spl-token-extensions-programs-token-extensions | 1.2.2 | extensions::metadata_pointer::authority; [transcript](transcripts/v1_2_2-tests-spl-token-extensions-programs-token-extensions.txt) |
| v1_2_2-tests-ido-pool-programs-ido-pool | 1.2.2 | generated corpus account/type resolution; [transcript](transcripts/v1_2_2-tests-ido-pool-programs-ido-pool.txt) |
| v1_2_2-tests-lockup-programs-registry | 1.2.2 | error[E0432]: unresolved import `anchor_lang::accounts::state`; [transcript](transcripts/v1_2_2-tests-lockup-programs-registry.txt) |
| v1_2_2-tests-auction-house-programs-auction-house | 1.2.2 | generated corpus account/type resolution; [transcript](transcripts/v1_2_2-tests-auction-house-programs-auction-house.txt) |
| production-openbook-v2 | 0.29.0 | Anchor 0.29.0; [transcript](transcripts/production-openbook-v2.txt) |
| production-drift-v2 | 0.29.0 | Anchor 0.29.0; [transcript](transcripts/production-drift-v2.txt) |

## Evidence and lead runs

Stock-relative regressions above use the current corpus. Frozen old-corpus baseline comparisons are separate evidence and must not be confused with those values; see the round-four baseline reports under `history/round-four/final-0.4.0-baseline/`.

[Exact report facts and artifact checks](round-four.json) retain current hashes and all exceptions. [Production provenance](history/round-four/final-0.4-production/provenance.json) identifies the frozen compiler and source trees. [Native flow evidence](rehearsal/chain-verification-0.4.json) and [reproducibility evidence](repro) remain separate from benchmark totals.

Docker, a validator, and public-chain transactions are not run on this Windows host. The lead runs the exact image, validator, registry, and API commands in [lead-run.md](lead-run.md). Recorded validator/mainnet counts are reported only when present in the final files.
