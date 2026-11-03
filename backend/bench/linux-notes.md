# Linux validator prerequisite and first-run evidence

The first lead run (`docs/lead-runs/bench-linux-1.log`) built and proved basic-0, basic-1, basic-4 and the registry. It did not record stock transactions or execute passing upstream validator tests. The saved recorder logs show `Cannot find module '@solana/web3.js'`; the saved validator log shows `io_uring NOT supported: Operation not permitted` followed by `assertion failed: io_uring_supported()`. These failures are independent.

The recorder resolves web3, bs58 and toml through the actual linked Anchor client, whose dependency graph owns them. Setup installs/builds and links those TypeScript clients before installing the matching Anchor CLI, then verifies this resolution path for tutorials and tests. A failed preflight names the missing dependency instead of timing out as an unexplained recorder startup failure.

Agave in the pinned Linux image requires `io_uring`. [Docker documents that its default seccomp profile blocks io_uring_setup, io_uring_enter and io_uring_register](https://docs.docker.com/engine/security/seccomp/). For this trusted, disposable benchmark container, add `--security-opt seccomp=unconfined` to the run invocation shown in the benchmark README. No ports are published and no wallets or secrets are mounted. This does not change the Docker daemon's policy or other containers. A hardened deployment can instead supply its own reviewed profile permitting the required syscalls. Kernel or host policy can still disable io_uring even without the default container seccomp profile.

`bench/check-linux.py` tries a one-entry io_uring setup and closes its file descriptor immediately. `linux.sh` records this check in `transcripts/validator-preflight.txt`; a denial skips validator stages while retaining a nonzero pipeline exit and still producing the end summary. It does not pretend zero executed tests passed. Build and SVM proof remain separate from validator-based suites.

The final table lists each selected program, captured transaction frames, stock-suite passed/ran counts and Tare-suite passed/ran counts. Counts are accepted only from matching source/build hashes. `stock-tests.json`, `upstream-tests.json` and `linux-summary.json` retain failures as well as successes. A registry row labels its native tests explicitly; those are not validator Mocha tests. Unsupported siblings may still prevent an upstream workspace suite from starting.

These script changes are checked natively on Windows; Docker and validators have not been rerun by this agent. The lead must execute the updated Linux command to establish new runtime evidence.

Anchor suites are launched asynchronously in their own Linux process group. On normal exit or SIGINT/SIGTERM, the runner sends termination to that owned group, then escalates to SIGKILL if necessary. This includes its validator descendants; no unrelated process name is targeted. The group lifecycle needs verification in the next Linux run.

The fresh-volume entry point links `/work/toolchain` to the packaged `/opt/tare` implementation before compiling the production corpus helper. The work volume therefore needs only `bench/` and `contract/schema/`; it does not need a Windows toolchain checkout. Bootstrap or fetch failures still produce a failing summary rather than an empty success.
