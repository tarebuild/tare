# Terminal captures

Per-program `-build.txt`, `-weigh.txt`, and `-prove.txt` files contain real command output. The matching `<id>.txt` file combines the run. Runtime paths are replaced with workspace-relative labels or a home placeholder, ANSI escape sequences are removed, and line endings are LF. `tare-version.txt` records the actual CLI version.

`tare-opt-tests.txt` is the bytecode rule-test run. The featured program is Anchor 0.32.3 `tests/idl/new-idl`: its 13 successfully measured instructions fit in one terminal table and exercise primitive, compound and empty account forms. Its verified Tare disassembly excerpt is `v0_32_3-tests-idl-programs-new-idl-tare-disassembly.txt`. Native RPC rehearsals live under `../gaps/round-three/flows/` because they are not validator transcripts. Attest/check/flag validator captures are produced by the lead. Historical and failed experiment captures stay in `../history/` or their named evidence folders.
