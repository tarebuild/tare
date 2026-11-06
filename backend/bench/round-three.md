# Toolchain round three

Tare 0.3.0; recorded 2026-11-14T17:43:45.092Z. All figures come from results.json. Totals sum successful instruction medians, excluding null weights.

178 measured program reports, 178 matching proofs, 55 not built. Compute: 5,469,171 gross / 3,808,167 net. ELF size: 33,918,520 stock / 28,531,128 Tare bytes. Inputs: 220,514 generated, 17 recorded tests, 0 mainnet.

21 measured programs do not meet the strictly-smaller ELF requirement. [Exact section/symbol evidence](gaps/round-three/size/README.md) retains each exception. [Unmeasured instruction audit](gaps/unmeasured.json) separates empty IDLs, intentional errors and unresolved generated state.

The round does not fully meet its size/compute target: 21 programs remain larger than stock, and the fixed-input ledger retains small `misc-optional` instruction regressions. All measured behavioral proofs match.

## Measured

| Program | Anchor | Gross | Net | Stock bytes | Tare bytes | Successful samples | Proof |
|---|---|---:|---:|---:|---:|---:|---|
| basic-0 | 0.31.1 | 197 | 137 | 162,176 | 107,032 | 12 | matched |
| basic-1 | 0.31.1 | 5,185 | 2,019 | 182,144 | 126,656 | 48 | matched |
| basic-2 | 0.31.1 | 5,736 | 2,199 | 184,456 | 131,384 | 48 | matched |
| puppet | 0.31.1 | 5,181 | 2,007 | 181,800 | 126,200 | 48 | matched |
| puppet-master | 0.31.1 | 3,492 | 2,124 | 175,144 | 121,256 | 32 | matched |
| basic-4 | 0.31.1 | 12,027 | 8,473 | 187,976 | 135,168 | 48 | matched |
| basic-5 | 0.31.1 | 12,030 | 6,231 | 189,320 | 142,064 | 360 | matched |
| declare-id | 0.31.1 | 197 | 137 | 162,184 | 107,032 | 12 | matched |
| multiple-suites | 0.31.1 | 201 | 137 | 162,376 | 108,488 | 12 | matched |
| system-accounts | 0.31.1 | 809 | 189 | 163,616 | 108,192 | 32 | matched |
| floats | 0.31.1 | 6,028 | 2,752 | 185,752 | 132,504 | 48 | matched |
| events | 0.31.1 | 10,172 | 8,552 | 172,568 | 123,888 | 56 | matched |
| errors | 0.31.1 | 6,740 | 2,132 | 198,080 | 136,736 | 304 | matched |
| relations-derivation | 0.31.1 | 16,622 | 12,743 | 193,888 | 149,000 | 104 | matched |
| v0_31_1-tests-spl-metadata-programs-metadata | 0.31.1 | ? | ? | 161,928 | 106,920 | 0 | matched |
| v0_31_1-tests-custom-coder-programs-spl-token | 0.31.1 | ? | ? | 162,072 | 106,936 | 0 | matched |
| v0_31_1-tests-validator-clone-programs-validator-clone | 0.31.1 | 197 | 137 | 162,192 | 107,040 | 12 | matched |
| v0_31_1-tests-anchor-cli-idl-programs-idl-commands-one | 0.31.1 | 197 | 137 | 162,200 | 107,040 | 12 | matched |
| v0_31_1-tests-anchor-cli-idl-programs-idl-commands-two | 0.31.1 | 201 | 138 | 162,240 | 107,032 | 12 | matched |
| v0_31_1-tests-multiple-suites-run-single-programs-multiple-suites-run-single | 0.31.1 | 197 | 137 | 162,216 | 107,048 | 12 | matched |
| v0_31_1-tests-custom-coder-programs-spl-associated-token | 0.31.1 | ? | ? | 162,096 | 106,952 | 0 | matched |
| v0_31_1-tests-safety-checks-programs-account-info | 0.31.1 | 559 | 152 | 163,344 | 107,304 | 32 | matched |
| v0_31_1-tests-safety-checks-programs-unchecked-account | 0.31.1 | 504 | 152 | 162,840 | 107,304 | 32 | matched |
| v0_31_1-tests-sysvars-programs-sysvars | 0.31.1 | 1,132 | 384 | 167,488 | 113,104 | 32 | matched |
| v0_31_1-tests-typescript-programs-typescript | 0.31.1 | 197 | 137 | 162,184 | 107,032 | 12 | matched |
| v0_31_1-tests-idl-programs-external | 0.31.1 | 196 | 136 | 162,040 | 107,016 | 12 | matched |
| v0_31_1-tests-safety-checks-programs-ignore-non-accounts | 0.31.1 | 810 | 166 | 163,744 | 107,576 | 32 | matched |
| v0_31_1-tests-idl-programs-docs | 0.31.1 | 576 | 183 | 164,944 | 109,640 | 32 | matched |
| v0_31_1-tests-optional-programs-allow-missing-optionals | 0.31.1 | 4,490 | 2,295 | 178,808 | 126,944 | 24 | matched |
| v0_31_1-tests-zero-copy-programs-zero-cpi | 0.31.1 | 9,925 | 7,921 | 179,176 | 134,776 | 32 | matched |
| v0_31_1-tests-anchor-cli-account-programs-account-command | 0.31.1 | 5,297 | 2,667 | 185,592 | 138,472 | 16 | matched |
| v0_31_1-tests-composite-programs-composite | 0.31.1 | 3,690 | 1,085 | 179,776 | 126,880 | 48 | matched |
| v0_31_1-tests-cpi-returns-programs-callee | 0.31.1 | 7,306 | 3,060 | 183,032 | 131,384 | 360 | matched |
| v0_31_1-tests-idl-programs-relations-derivation | 0.31.1 | 12,129 | 8,785 | 188,952 | 137,192 | 32 | matched |
| v0_31_1-tests-misc-programs-init-if-needed | 0.31.1 | 10,355 | 4,175 | 192,688 | 147,224 | 64 | matched |
| v0_31_1-tests-idl-programs-generics | 0.31.1 | 1,920 | 968 | 172,360 | 120,080 | 32 | matched |
| v0_31_1-tests-custom-discriminator-programs-custom-discriminator | 0.31.1 | 9,507 | 4,896 | 182,600 | 130,792 | 248 | matched |
| v0_31_1-tests-misc-programs-remaining-accounts | 0.31.1 | 10,538 | 4,620 | 198,504 | 146,296 | 64 | matched |
| v0_31_1-tests-realloc-programs-realloc | 0.31.1 | 18,244 | 13,176 | 210,376 | 194,256 | 92 | matched |
| v0_31_1-tests-cpi-returns-programs-caller | 0.31.1 | 13,133 | 8,795 | 185,128 | 149,008 | 132 | matched |
| v0_31_1-tests-misc-programs-lamports | 0.31.1 | 13,294 | 10,229 | 206,952 | 173,520 | 48 | matched |
| v0_31_1-tests-chat-programs-chat | 0.31.1 | 12,365 | 7,913 | 201,224 | 157,120 | 64 | matched |
| v0_31_1-tests-declare-program-programs-external | 0.31.1 | 21,915 | 15,856 | 192,632 | 149,952 | 220 | matched |
| v0_31_1-tests-misc-programs-overflow-checks | 0.31.1 | 10,936 | 3,450 | 189,984 | 136,984 | 64 | matched |
| v0_31_1-tests-custom-coder-programs-native-system | 0.31.1 | 12,941 | 4,070 | 196,152 | 138,072 | 424 | matched |
| v0_31_1-tests-optional-programs-optional | 0.31.1 | 20,302 | 12,701 | 235,360 | 231,736 | 156 | matched |
| v0_31_1-tests-zero-copy-programs-zero-copy | 0.31.1 | 21,301 | 14,434 | 207,528 | 165,416 | 376 | matched |
| v0_31_1-tests-cashiers-check-programs-cashiers-check | 0.31.1 | 23,018 | 18,964 | 217,896 | 183,184 | 40 | matched |
| v0_31_1-tests-spl-token-proxy-programs-token-proxy | 0.31.1 | 71,905 | 62,932 | 300,640 | 233,424 | 216 | matched |
| v0_31_1-tests-escrow-programs-escrow | 0.31.1 | 41,057 | 35,728 | 245,208 | 208,488 | 40 | matched |
| v0_31_1-tests-spl-token-wrapper-programs-token-wrapper | 0.31.1 | 73,714 | 66,015 | 292,280 | 222,504 | 117 | matched |
| v0_31_1-tests-misc-programs-misc-optional | 0.31.1 | 349,522 | 279,423 | 767,752 | 769,376 | 1,162 | matched |
| v0_31_1-tests-misc-programs-misc | 0.31.1 | 409,795 | 324,438 | 716,496 | 754,552 | 900 | matched |
| v0_31_1-tests-bench-programs-bench | 0.31.1 | 442,975 | 232,638 | 1,066,792 | 1,318,768 | 1,944 | matched |
| v0_32_3-tests-spl-metadata-programs-metadata | 0.32.3 | ? | ? | 161,288 | 106,920 | 0 | matched |
| v0_32_3-tests-custom-coder-programs-spl-token | 0.32.3 | ? | ? | 161,432 | 106,936 | 0 | matched |
| v0_32_3-examples-tutorial-basic-0-programs-basic-0 | 0.32.3 | 197 | 137 | 161,536 | 107,032 | 12 | matched |
| v0_32_3-tests-validator-clone-programs-validator-clone | 0.32.3 | 197 | 137 | 161,552 | 107,040 | 12 | matched |
| v0_32_3-tests-anchor-cli-idl-programs-idl-commands-one | 0.32.3 | 197 | 137 | 161,560 | 107,040 | 12 | matched |
| v0_32_3-tests-anchor-cli-idl-programs-idl-commands-two | 0.32.3 | 201 | 138 | 161,600 | 107,032 | 12 | matched |
| v0_32_3-tests-multiple-suites-run-single-programs-multiple-suites-run-single | 0.32.3 | 197 | 137 | 161,576 | 107,048 | 12 | matched |
| v0_32_3-tests-custom-coder-programs-spl-associated-token | 0.32.3 | ? | ? | 161,456 | 106,952 | 0 | matched |
| v0_32_3-tests-safety-checks-programs-account-info | 0.32.3 | 559 | 152 | 162,704 | 107,304 | 32 | matched |
| v0_32_3-tests-safety-checks-programs-unchecked-account | 0.32.3 | 504 | 152 | 162,200 | 107,304 | 32 | matched |
| v0_32_3-tests-declare-id-programs-declare-id | 0.32.3 | 197 | 137 | 161,544 | 107,032 | 12 | matched |
| v0_32_3-tests-system-accounts-programs-system-accounts | 0.32.3 | 809 | 189 | 162,976 | 108,192 | 32 | matched |
| v0_32_3-tests-multiple-suites-programs-multiple-suites | 0.32.3 | 201 | 137 | 161,736 | 108,488 | 12 | matched |
| v0_32_3-tests-sysvars-programs-sysvars | 0.32.3 | 1,132 | 384 | 166,848 | 113,104 | 32 | matched |
| v0_32_3-tests-typescript-programs-typescript | 0.32.3 | 197 | 137 | 161,544 | 107,032 | 12 | matched |
| v0_32_3-tests-idl-programs-external | 0.32.3 | 196 | 136 | 161,400 | 107,016 | 12 | matched |
| v0_32_3-tests-safety-checks-programs-ignore-non-accounts | 0.32.3 | 810 | 166 | 163,104 | 107,576 | 32 | matched |
| v0_32_3-examples-tutorial-basic-3-programs-puppet | 0.32.3 | 5,034 | 2,007 | 181,288 | 126,200 | 48 | matched |
| v0_32_3-examples-tutorial-basic-3-programs-puppet-master | 0.32.3 | 3,353 | 2,124 | 174,400 | 121,256 | 32 | matched |
| v0_32_3-tests-idl-programs-docs | 0.32.3 | 576 | 183 | 164,304 | 109,640 | 32 | matched |
| v0_32_3-tests-optional-programs-allow-missing-optionals | 0.32.3 | 4,343 | 2,295 | 178,296 | 126,944 | 24 | matched |
| v0_32_3-examples-tutorial-basic-1-programs-basic-1 | 0.32.3 | 5,038 | 2,019 | 181,632 | 126,656 | 48 | matched |
| v0_32_3-examples-tutorial-basic-2-programs-basic-2 | 0.32.3 | 5,589 | 2,199 | 183,944 | 131,384 | 48 | matched |
| v0_32_3-tests-zero-copy-programs-zero-cpi | 0.32.3 | 9,755 | 7,921 | 178,456 | 134,776 | 32 | matched |
| v0_32_3-tests-events-programs-events | 0.32.3 | 10,041 | 8,552 | 171,928 | 123,888 | 56 | matched |
| v0_32_3-tests-anchor-cli-account-programs-account-command | 0.32.3 | 5,150 | 2,667 | 185,080 | 138,472 | 16 | matched |
| v0_32_3-tests-floats-programs-floats | 0.32.3 | 5,881 | 2,752 | 185,240 | 132,504 | 48 | matched |
| v0_32_3-tests-composite-programs-composite | 0.32.3 | 3,690 | 1,085 | 179,136 | 126,880 | 48 | matched |
| v0_32_3-tests-cpi-returns-programs-callee | 0.32.3 | 7,151 | 3,060 | 182,504 | 131,384 | 360 | matched |
| v0_32_3-tests-idl-programs-relations-derivation | 0.32.3 | 11,982 | 8,785 | 188,440 | 137,192 | 32 | matched |
| v0_32_3-tests-misc-programs-init-if-needed | 0.32.3 | 10,060 | 4,175 | 192,176 | 147,224 | 64 | matched |
| v0_32_3-examples-tutorial-basic-4-programs-basic-4 | 0.32.3 | 11,880 | 8,473 | 187,464 | 135,168 | 48 | matched |
| v0_32_3-tests-idl-programs-generics | 0.32.3 | 1,920 | 968 | 171,720 | 120,080 | 32 | matched |
| v0_32_3-tests-custom-discriminator-programs-custom-discriminator | 0.32.3 | 9,360 | 4,896 | 182,088 | 130,792 | 248 | matched |
| v0_32_3-tests-misc-programs-remaining-accounts | 0.32.3 | 10,244 | 4,620 | 197,992 | 146,296 | 64 | matched |
| v0_32_3-tests-relations-derivation-programs-relations-derivation | 0.32.3 | 16,475 | 12,743 | 193,376 | 149,000 | 104 | matched |
| v0_32_3-tests-realloc-programs-realloc | 0.32.3 | 17,966 | 13,176 | 209,840 | 194,256 | 92 | matched |
| v0_32_3-tests-cpi-returns-programs-caller | 0.32.3 | 12,698 | 8,789 | 184,352 | 149,008 | 132 | matched |
| v0_32_3-tests-misc-programs-lamports | 0.32.3 | 13,000 | 10,229 | 206,440 | 173,520 | 48 | matched |
| v0_32_3-tests-chat-programs-chat | 0.32.3 | 12,218 | 7,913 | 200,712 | 157,120 | 64 | matched |
| v0_32_3-tests-declare-program-programs-external | 0.32.3 | 21,766 | 15,856 | 192,104 | 149,952 | 220 | matched |
| v0_32_3-examples-tutorial-basic-5-programs-basic-5 | 0.32.3 | 11,883 | 6,231 | 188,808 | 142,064 | 360 | matched |
| v0_32_3-tests-misc-programs-overflow-checks | 0.32.3 | 10,789 | 3,450 | 189,472 | 136,984 | 64 | matched |
| v0_32_3-tests-custom-coder-programs-native-system | 0.32.3 | 12,941 | 4,070 | 195,512 | 138,072 | 424 | matched |
| v0_32_3-tests-optional-programs-optional | 0.32.3 | 19,858 | 12,701 | 234,832 | 231,736 | 156 | matched |
| v0_32_3-tests-zero-copy-programs-zero-copy | 0.32.3 | 21,154 | 14,434 | 207,016 | 165,416 | 376 | matched |
| v0_32_3-tests-errors-programs-errors | 0.32.3 | 6,740 | 2,132 | 197,424 | 136,736 | 304 | matched |
| v0_32_3-tests-cashiers-check-programs-cashiers-check | 0.32.3 | 22,529 | 18,964 | 217,336 | 182,760 | 40 | matched |
| v0_32_3-tests-spl-token-proxy-programs-token-proxy | 0.32.3 | 70,312 | 62,937 | 300,440 | 233,232 | 216 | matched |
| v0_32_3-tests-escrow-programs-escrow | 0.32.3 | 40,112 | 35,731 | 244,824 | 208,112 | 40 | matched |
| v0_32_3-tests-spl-token-wrapper-programs-token-wrapper | 0.32.3 | 71,828 | 66,021 | 292,080 | 222,312 | 117 | matched |
| v0_32_3-tests-idl-programs-new-idl | 0.32.3 | 35,181 | 19,281 | 283,536 | 259,536 | 204 | matched |
| v0_32_3-tests-misc-programs-misc-optional | 0.32.3 | 344,897 | 279,430 | 767,632 | 769,608 | 1,162 | matched |
| v0_32_3-tests-misc-programs-misc | 0.32.3 | 403,429 | 324,451 | 716,376 | 754,784 | 900 | matched |
| v0_32_3-tests-bench-programs-bench | 0.32.3 | 429,745 | 232,638 | 1,066,280 | 1,318,768 | 1,944 | matched |
| v1_2_2-tests-spl-metadata-programs-metadata | 1.2.2 | ? | ? | 54,768 | 42,656 | 0 | matched |
| v1_2_2-tests-custom-coder-programs-spl-token | 1.2.2 | ? | ? | 54,784 | 42,672 | 0 | matched |
| v1_2_2-examples-tutorial-basic-0-programs-basic-0 | 1.2.2 | 185 | 128 | 54,992 | 42,744 | 12 | matched |
| v1_2_2-tests-idl-programs-env-id | 1.2.2 | 185 | 128 | 54,992 | 42,744 | 12 | matched |
| v1_2_2-tests-validator-clone-programs-validator-clone | 1.2.2 | 185 | 128 | 54,992 | 42,744 | 12 | matched |
| v1_2_2-tests-anchor-cli-idl-programs-idl-commands-one | 1.2.2 | 185 | 128 | 54,992 | 42,744 | 12 | matched |
| v1_2_2-tests-anchor-cli-idl-programs-idl-commands-two | 1.2.2 | 187 | 128 | 54,992 | 42,752 | 12 | matched |
| v1_2_2-tests-solana-program-deps-matching-programs-matching-solana-program | 1.2.2 | 185 | 128 | 54,992 | 42,744 | 12 | matched |
| v1_2_2-tests-solana-program-deps-mismatched-programs-mismatched-solana-program | 1.2.2 | 185 | 128 | 54,992 | 42,744 | 12 | matched |
| v1_2_2-tests-multiple-suites-run-single-programs-multiple-suites-run-single | 1.2.2 | 185 | 128 | 54,992 | 42,744 | 12 | matched |
| v1_2_2-tests-custom-coder-programs-spl-associated-token | 1.2.2 | ? | ? | 54,784 | 42,672 | 0 | matched |
| v1_2_2-tests-multiple-scripts-programs-example | 1.2.2 | 288 | 254 | 55,072 | 45,536 | 12 | matched |
| v1_2_2-tests-safety-checks-programs-account-info | 1.2.2 | 581 | 145 | 56,128 | 58,776 | 32 | matched |
| v1_2_2-tests-safety-checks-programs-unchecked-account | 1.2.2 | 526 | 145 | 55,616 | 58,776 | 32 | matched |
| v1_2_2-tests-declare-id-programs-declare-id | 1.2.2 | 185 | 128 | 54,992 | 42,744 | 12 | matched |
| v1_2_2-tests-system-accounts-programs-system-accounts | 1.2.2 | 870 | 185 | 56,608 | 59,672 | 32 | matched |
| v1_2_2-tests-multiple-suites-programs-multiple-suites | 1.2.2 | 189 | 129 | 72,608 | 58,400 | 12 | matched |
| v1_2_2-tests-sysvars-programs-sysvars | 1.2.2 | 1,228 | 375 | 60,856 | 64,536 | 32 | matched |
| v1_2_2-tests-typescript-programs-typescript | 1.2.2 | 185 | 128 | 54,992 | 42,744 | 12 | matched |
| v1_2_2-tests-anchor-cli-legacy-idl-programs-anchor-cli-legacy-idl | 1.2.2 | 538 | 167 | 167,752 | 105,320 | 32 | matched |
| v1_2_2-tests-interface-account-programs-old | 1.2.2 | 4,400 | 1,764 | 117,208 | 78,368 | 16 | matched |
| v1_2_2-tests-idl-programs-external | 1.2.2 | 581 | 177 | 61,032 | 61,152 | 32 | matched |
| v1_2_2-tests-account-generation-test-programs-account-generation-test | 1.2.2 | 4,371 | 1,769 | 116,576 | 76,816 | 16 | matched |
| v1_2_2-tests-safety-checks-programs-ignore-non-accounts | 1.2.2 | 869 | 160 | 56,520 | 58,968 | 32 | matched |
| v1_2_2-tests-test-instruction-validation-pass-args-count-programs-test-instruction-validation | 1.2.2 | 1,724 | 975 | 75,016 | 64,512 | 64 | matched |
| v1_2_2-examples-tutorial-basic-3-programs-puppet | 1.2.2 | 5,679 | 2,002 | 129,408 | 82,808 | 48 | matched |
| v1_2_2-examples-tutorial-basic-3-programs-puppet-master | 1.2.2 | 4,416 | 3,182 | 99,960 | 72,456 | 32 | matched |
| v1_2_2-tests-idl-programs-docs | 1.2.2 | 596 | 178 | 78,840 | 61,264 | 32 | matched |
| v1_2_2-tests-optional-programs-allow-missing-optionals | 1.2.2 | 4,420 | 2,281 | 117,824 | 82,528 | 24 | matched |
| v1_2_2-tests-cpi-returns-programs-malicious | 1.2.2 | 666 | 262 | 56,016 | 59,208 | 32 | matched |
| v1_2_2-examples-tutorial-basic-1-programs-basic-1 | 1.2.2 | 5,686 | 2,012 | 129,736 | 83,024 | 48 | matched |
| v1_2_2-tests-interface-account-programs-new | 1.2.2 | 8,977 | 3,745 | 128,272 | 92,272 | 32 | matched |
| v1_2_2-examples-tutorial-basic-2-programs-basic-2 | 1.2.2 | 6,267 | 2,205 | 132,368 | 87,616 | 48 | matched |
| v1_2_2-tests-events-programs-events | 1.2.2 | 3,824 | 2,470 | 91,352 | 76,872 | 56 | matched |
| v1_2_2-tests-anchor-cli-account-programs-account-command | 1.2.2 | 5,200 | 2,587 | 129,808 | 93,152 | 16 | matched |
| v1_2_2-tests-floats-programs-floats | 1.2.2 | 6,561 | 2,791 | 133,872 | 88,344 | 48 | matched |
| v1_2_2-tests-composite-programs-composite | 1.2.2 | 7,436 | 1,639 | 122,760 | 94,576 | 48 | matched |
| v1_2_2-tests-test-instruction-validation-pass-type-programs-test-instruction-validation | 1.2.2 | 2,667 | 1,552 | 76,272 | 65,696 | 96 | matched |
| v1_2_2-tests-idl-programs-relations-derivation | 1.2.2 | 11,993 | 8,686 | 127,928 | 93,576 | 32 | matched |
| v1_2_2-tests-misc-programs-init-if-needed | 1.2.2 | 11,325 | 4,172 | 145,760 | 102,920 | 64 | matched |
| v1_2_2-examples-tutorial-basic-4-programs-basic-4 | 1.2.2 | 12,499 | 8,423 | 135,720 | 92,200 | 48 | matched |
| v1_2_2-tests-cpi-returns-programs-callee | 1.2.2 | 8,221 | 3,516 | 122,584 | 87,072 | 188 | matched |
| v1_2_2-tests-custom-program-programs-custom-program | 1.2.2 | 21,479 | 20,778 | 72,472 | 81,000 | 32 | matched |
| v1_2_2-tests-custom-discriminator-programs-custom-discriminator | 1.2.2 | 9,541 | 4,878 | 121,472 | 87,848 | 248 | matched |
| v1_2_2-tests-misc-programs-remaining-accounts | 1.2.2 | 10,418 | 4,538 | 135,288 | 104,808 | 64 | matched |
| v1_2_2-tests-idl-programs-generics | 1.2.2 | 2,016 | 962 | 89,008 | 71,288 | 32 | matched |
| v1_2_2-tests-relations-derivation-programs-relations-derivation | 1.2.2 | 16,823 | 12,732 | 133,640 | 107,672 | 104 | matched |
| v1_2_2-tests-misc-programs-lamports | 1.2.2 | 12,955 | 10,192 | 149,376 | 132,056 | 48 | matched |
| v1_2_2-tests-chat-programs-chat | 1.2.2 | 12,315 | 7,780 | 143,872 | 108,624 | 64 | matched |
| v1_2_2-examples-tutorial-basic-5-programs-basic-5 | 1.2.2 | 14,556 | 6,244 | 138,400 | 94,616 | 360 | matched |
| v1_2_2-tests-misc-programs-overflow-checks | 1.2.2 | 12,698 | 3,454 | 139,880 | 93,224 | 64 | matched |
| v1_2_2-tests-accountloader-realloc-programs-accountloader-realloc | 1.2.2 | 29,425 | 21,866 | 161,768 | 145,544 | 324 | matched |
| v1_2_2-tests-custom-coder-programs-native-system | 1.2.2 | 13,833 | 4,068 | 113,856 | 90,664 | 424 | matched |
| v1_2_2-tests-realloc-programs-realloc | 1.2.2 | 35,996 | 24,087 | 188,744 | 193,392 | 308 | matched |
| v1_2_2-tests-optional-programs-optional | 1.2.2 | 23,103 | 12,587 | 186,048 | 187,592 | 156 | matched |
| v1_2_2-tests-errors-programs-errors | 1.2.2 | 7,585 | 2,121 | 126,984 | 92,464 | 304 | matched |
| v1_2_2-tests-duplicate-mutable-accounts-programs-duplicate-mutable-accounts | 1.2.2 | 32,345 | 12,134 | 182,288 | 189,840 | 264 | matched |
| v1_2_2-tests-declare-program-programs-declare-program | 1.2.2 | 36,744 | 31,467 | 134,176 | 119,400 | 72 | matched |
| v1_2_2-tests-tictactoe-programs-tictactoe | 1.2.2 | 12,185 | 3,499 | 123,816 | 106,848 | 134 | matched |
| v1_2_2-tests-cashiers-check-programs-cashiers-check | 1.2.2 | 26,269 | 18,628 | 152,184 | 140,832 | 40 | matched |
| v1_2_2-tests-declare-program-programs-external | 1.2.2 | 37,968 | 24,714 | 172,560 | 148,440 | 396 | matched |
| v1_2_2-tests-cpi-returns-programs-caller | 1.2.2 | 39,748 | 30,516 | 137,864 | 140,832 | 260 | matched |
| v1_2_2-tests-spl-token-proxy-programs-token-proxy | 1.2.2 | 75,485 | 63,187 | 244,520 | 190,456 | 200 | matched |
| v1_2_2-tests-escrow-programs-escrow | 1.2.2 | 45,864 | 36,124 | 195,656 | 168,048 | 40 | matched |
| v1_2_2-tests-spl-token-wrapper-programs-token-wrapper | 1.2.2 | 79,250 | 66,349 | 236,232 | 184,992 | 108 | matched |
| v1_2_2-tests-misc-programs-misc-optional | 1.2.2 | 355,469 | 276,713 | 695,384 | 712,528 | 1,162 | matched |
| v1_2_2-tests-misc-programs-misc | 1.2.2 | 414,297 | 323,301 | 636,304 | 712,520 | 900 | matched |
| v1_2_2-tests-bench-programs-bench | 1.2.2 | 438,316 | 233,894 | 833,576 | 1,263,960 | 1,944 | matched |
| tare-registry | 1.2.2 | 84,919 | 66,967 | 281,992 | 261,072 | 7 | matched |

## Not built

| Program | Anchor | Construct or observed failure |
|---|---|---|
| v0_31_1-tests-custom-discriminator-programs-ambiguous-discriminator | 0.31.1 | stock IDL validation; [transcript](transcripts/v0_31_1-tests-custom-discriminator-programs-ambiguous-discriminator.txt) |
| v0_31_1-tests-bpf-upgradeable-state-programs-bpf-upgradeable-state | 0.31.1 | generated corpus account/type resolution; [transcript](transcripts/v0_31_1-tests-bpf-upgradeable-state-programs-bpf-upgradeable-state.txt) |
| v0_31_1-tests-lazy-account-programs-lazy-account | 0.31.1 | LazyAccount; [transcript](transcripts/v0_31_1-tests-lazy-account-programs-lazy-account.txt) |
| v0_31_1-tests-pyth-programs-pyth | 0.31.1 | std::cell::RefMut account-data guard; [transcript](transcripts/v0_31_1-tests-pyth-programs-pyth.txt) |
| v0_31_1-tests-declare-program-programs-declare-program | 0.31.1 | SBF stack frame > 4096 bytes; [transcript](transcripts/v0_31_1-tests-declare-program-programs-declare-program.txt) |
| v0_31_1-tests-spl-transfer-hook-programs-transfer-hook | 0.31.1 | SPL AccountInfo interoperability; [transcript](transcripts/v0_31_1-tests-spl-transfer-hook-programs-transfer-hook.txt) |
| v0_31_1-tests-tictactoe-programs-tictactoe | 0.31.1 | error[E0277]: the trait bound anchor_lang::Error: From<ErrorCode> is not satisfied; [transcript](transcripts/v0_31_1-tests-tictactoe-programs-tictactoe.txt) |
| v0_31_1-tests-pda-derivation-programs-pda-derivation | 0.31.1 | generated corpus account/type resolution; [transcript](transcripts/v0_31_1-tests-pda-derivation-programs-pda-derivation.txt) |
| v0_31_1-tests-idl-programs-idl | 0.31.1 | SBF stack frame > 4096 bytes; [transcript](transcripts/v0_31_1-tests-idl-programs-idl.txt) |
| v0_31_1-tests-multisig-programs-multisig | 0.31.1 | error[E0308]: mismatched types; [transcript](transcripts/v0_31_1-tests-multisig-programs-multisig.txt) |
| v0_31_1-tests-spl-token-extensions-programs-token-extensions | 0.31.1 | extensions::metadata_pointer::authority; [transcript](transcripts/v0_31_1-tests-spl-token-extensions-programs-token-extensions.txt) |
| v0_31_1-tests-idl-programs-new-idl | 0.31.1 | stock IDL validation; [transcript](transcripts/v0_31_1-tests-idl-programs-new-idl.txt) |
| v0_31_1-tests-swap-programs-swap | 0.31.1 | upstream dependency resolution; [transcript](transcripts/v0_31_1-tests-swap-programs-swap.txt) |
| v0_31_1-tests-lockup-programs-lockup | 0.31.1 | error[E0432]: unresolved import anchor_lang::accounts::state; [transcript](transcripts/v0_31_1-tests-lockup-programs-lockup.txt) |
| v0_31_1-tests-ido-pool-programs-ido-pool | 0.31.1 | generated corpus account/type resolution; [transcript](transcripts/v0_31_1-tests-ido-pool-programs-ido-pool.txt) |
| v0_31_1-tests-cfo-programs-cfo | 0.31.1 | upstream dependency resolution; [transcript](transcripts/v0_31_1-tests-cfo-programs-cfo.txt) |
| v0_31_1-tests-lockup-programs-registry | 0.31.1 | error[E0432]: unresolved import anchor_lang::accounts::state; [transcript](transcripts/v0_31_1-tests-lockup-programs-registry.txt) |
| v0_31_1-tests-auction-house-programs-auction-house | 0.31.1 | error[E0308]: mismatched types; [transcript](transcripts/v0_31_1-tests-auction-house-programs-auction-house.txt) |
| v0_32_3-tests-custom-discriminator-programs-ambiguous-discriminator | 0.32.3 | stock IDL validation; [transcript](transcripts/v0_32_3-tests-custom-discriminator-programs-ambiguous-discriminator.txt) |
| v0_32_3-tests-bpf-upgradeable-state-programs-bpf-upgradeable-state | 0.32.3 | generated corpus account/type resolution; [transcript](transcripts/v0_32_3-tests-bpf-upgradeable-state-programs-bpf-upgradeable-state.txt) |
| v0_32_3-tests-lazy-account-programs-lazy-account | 0.32.3 | LazyAccount; [transcript](transcripts/v0_32_3-tests-lazy-account-programs-lazy-account.txt) |
| v0_32_3-tests-pyth-programs-pyth | 0.32.3 | std::cell::RefMut account-data guard; [transcript](transcripts/v0_32_3-tests-pyth-programs-pyth.txt) |
| v0_32_3-tests-declare-program-programs-declare-program | 0.32.3 | SBF stack frame > 4096 bytes; [transcript](transcripts/v0_32_3-tests-declare-program-programs-declare-program.txt) |
| v0_32_3-tests-spl-transfer-hook-programs-transfer-hook | 0.32.3 | SPL AccountInfo interoperability; [transcript](transcripts/v0_32_3-tests-spl-transfer-hook-programs-transfer-hook.txt) |
| v0_32_3-tests-tictactoe-programs-tictactoe | 0.32.3 | error[E0277]: the trait bound anchor_lang::Error: From<ErrorCode> is not satisfied; [transcript](transcripts/v0_32_3-tests-tictactoe-programs-tictactoe.txt) |
| v0_32_3-tests-pda-derivation-programs-pda-derivation | 0.32.3 | generated corpus account/type resolution; [transcript](transcripts/v0_32_3-tests-pda-derivation-programs-pda-derivation.txt) |
| v0_32_3-tests-idl-programs-idl | 0.32.3 | SBF stack frame > 4096 bytes; [transcript](transcripts/v0_32_3-tests-idl-programs-idl.txt) |
| v0_32_3-tests-multisig-programs-multisig | 0.32.3 | error[E0308]: mismatched types; [transcript](transcripts/v0_32_3-tests-multisig-programs-multisig.txt) |
| v0_32_3-tests-spl-token-extensions-programs-token-extensions | 0.32.3 | extensions::metadata_pointer::authority; [transcript](transcripts/v0_32_3-tests-spl-token-extensions-programs-token-extensions.txt) |
| v0_32_3-tests-swap-programs-swap | 0.32.3 | upstream dependency resolution; [transcript](transcripts/v0_32_3-tests-swap-programs-swap.txt) |
| v0_32_3-tests-lockup-programs-lockup | 0.32.3 | error[E0432]: unresolved import anchor_lang::accounts::state; [transcript](transcripts/v0_32_3-tests-lockup-programs-lockup.txt) |
| v0_32_3-tests-ido-pool-programs-ido-pool | 0.32.3 | generated corpus account/type resolution; [transcript](transcripts/v0_32_3-tests-ido-pool-programs-ido-pool.txt) |
| v0_32_3-tests-cfo-programs-cfo | 0.32.3 | upstream dependency resolution; [transcript](transcripts/v0_32_3-tests-cfo-programs-cfo.txt) |
| v0_32_3-tests-lockup-programs-registry | 0.32.3 | error[E0432]: unresolved import anchor_lang::accounts::state; [transcript](transcripts/v0_32_3-tests-lockup-programs-registry.txt) |
| v0_32_3-tests-auction-house-programs-auction-house | 0.32.3 | error[E0308]: mismatched types; [transcript](transcripts/v0_32_3-tests-auction-house-programs-auction-house.txt) |
| v1_2_2-tests-errors-programs-multiple-errors | 1.2.2 | stock IDL validation; [transcript](transcripts/v1_2_2-tests-errors-programs-multiple-errors.txt) |
| v1_2_2-tests-custom-discriminator-programs-zeroed-discriminator | 1.2.2 | stock all-zero discriminator rejection; [transcript](transcripts/v1_2_2-tests-custom-discriminator-programs-zeroed-discriminator.txt) |
| v1_2_2-tests-custom-discriminator-programs-ambiguous-discriminator | 1.2.2 | stock IDL validation; [transcript](transcripts/v1_2_2-tests-custom-discriminator-programs-ambiguous-discriminator.txt) |
| v1_2_2-tests-zero-copy-programs-zero-cpi | 1.2.2 | upstream dependency resolution; [transcript](transcripts/v1_2_2-tests-zero-copy-programs-zero-cpi.txt) |
| v1_2_2-tests-test-instruction-validation-fail-type-programs-test-instruction-validation | 1.2.2 | error[E0308]: mismatched types; [transcript](transcripts/v1_2_2-tests-test-instruction-validation-fail-type-programs-test-instruction-validation.txt) |
| v1_2_2-tests-test-instruction-validation-fail-args-count-programs-test-instruction-validation | 1.2.2 | stock instruction validation; [transcript](transcripts/v1_2_2-tests-test-instruction-validation-fail-args-count-programs-test-instruction-validation.txt) |
| v1_2_2-tests-interface-account-programs-interface-account | 1.2.2 | generated corpus account/type resolution; [transcript](transcripts/v1_2_2-tests-interface-account-programs-interface-account.txt) |
| v1_2_2-tests-bpf-upgradeable-state-programs-bpf-upgradeable-state | 1.2.2 | generated corpus account/type resolution; [transcript](transcripts/v1_2_2-tests-bpf-upgradeable-state-programs-bpf-upgradeable-state.txt) |
| v1_2_2-tests-pyth-programs-pyth | 1.2.2 | std::cell::RefMut account-data guard; [transcript](transcripts/v1_2_2-tests-pyth-programs-pyth.txt) |
| v1_2_2-tests-lazy-account-programs-lazy-account | 1.2.2 | LazyAccount; [transcript](transcripts/v1_2_2-tests-lazy-account-programs-lazy-account.txt) |
| v1_2_2-tests-zero-copy-programs-zero-copy | 1.2.2 | upstream dependency resolution; [transcript](transcripts/v1_2_2-tests-zero-copy-programs-zero-copy.txt) |
| v1_2_2-tests-spl-transfer-hook-programs-transfer-hook | 1.2.2 | SPL AccountInfo interoperability; [transcript](transcripts/v1_2_2-tests-spl-transfer-hook-programs-transfer-hook.txt) |
| v1_2_2-tests-pda-derivation-programs-pda-derivation | 1.2.2 | generated corpus account/type resolution; [transcript](transcripts/v1_2_2-tests-pda-derivation-programs-pda-derivation.txt) |
| v1_2_2-tests-multisig-programs-multisig | 1.2.2 | error[E0308]: mismatched types; [transcript](transcripts/v1_2_2-tests-multisig-programs-multisig.txt) |
| v1_2_2-tests-idl-programs-idl | 1.2.2 | error[E0433]: cannot find feature in solana_program; [transcript](transcripts/v1_2_2-tests-idl-programs-idl.txt) |
| v1_2_2-tests-lockup-programs-lockup | 1.2.2 | error[E0432]: unresolved import anchor_lang::accounts::state; [transcript](transcripts/v1_2_2-tests-lockup-programs-lockup.txt) |
| v1_2_2-tests-spl-token-extensions-programs-token-extensions | 1.2.2 | extensions::metadata_pointer::authority; [transcript](transcripts/v1_2_2-tests-spl-token-extensions-programs-token-extensions.txt) |
| v1_2_2-tests-ido-pool-programs-ido-pool | 1.2.2 | generated corpus account/type resolution; [transcript](transcripts/v1_2_2-tests-ido-pool-programs-ido-pool.txt) |
| v1_2_2-tests-lockup-programs-registry | 1.2.2 | error[E0432]: unresolved import anchor_lang::accounts::state; [transcript](transcripts/v1_2_2-tests-lockup-programs-registry.txt) |
| v1_2_2-tests-auction-house-programs-auction-house | 1.2.2 | error[E0107]: struct takes 4 lifetime arguments but 1 lifetime argument was supplied; [transcript](transcripts/v1_2_2-tests-auction-house-programs-auction-house.txt) |

## Remaining performance requirement

The fixed round-two corpora are compared separately from the expanded generated inputs. [The performance ledger](gaps/round-three/performance/README.md) records every residual instruction-median increase. A matching proof does not waive either a size or compute regression.

Featured terminal program: `v0_32_3-tests-idl-programs-new-idl`, with 13 measured instructions. Its command captures and hash-verified entrypoint disassembly are in `transcripts/`.


## Input coverage

The final corpus includes 2,040 successful System/SPL setup transactions. Forty-two instruction/version rows still lack successful generated fixtures, 48 deliberately failing source handlers are unmeasured, and one upstream source/IDL mismatch cannot dispatch successfully. Nine helper IDLs declare no user instructions. Every instruction measured successfully in round two still has a successful sample in the final expanded corpus. See [the full unmeasured audit](gaps/unmeasured.json).

## Evidence

[Frozen toolchain sources](provenance/frozen-build.json), [fixed-corpus performance comparisons](gaps/round-three/performance/), [bytecode controls](gaps/round-three/opt/), and [registry native money tests](../registry/README.md) retain their separate provenance. Native recorded registry tests count as tests; upstream validator suites remain unrun here. The Linux entry point records those outcomes separately.

[Lead commands and environment](README.md), [portable report corpus](../toolchain/contract-additions.md), and [registry deployment](../registry/README.md) describe the unexecuted network/container steps.
