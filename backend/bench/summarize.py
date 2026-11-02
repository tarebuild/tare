"""Render current measured facts; never infer a proof or successful sample."""
import json
from pathlib import Path

root = Path(__file__).resolve().parent
result = json.loads((root / "results.json").read_text(encoding="utf-8"))
rows = result["programs"]
measured = [row for row in rows if row["status"] == "measured"]
larger = [row for row in measured if row["weighing"]["builds"]["tare"]["sizeBytes"] >= row["weighing"]["builds"]["stock"]["sizeBytes"]]
t = result["totals"]
lines = ["# Toolchain round three", "",
    f"Tare {result['toolchainVersion']}; recorded {result['generatedAt']}. All figures come from results.json. Totals sum successful instruction medians, excluding null weights.", "",
    f"{len(measured)} measured program reports, {sum(p['proof']['status'] == 'matched' for p in measured)} matching proofs, {t['notBuilt']} not built. "
    f"Compute: {t['weights']['gross']:,} gross / {t['weights']['net']:,} net. "
    f"ELF size: {t['binarySize']['gross']:,} stock / {t['binarySize']['net']:,} Tare bytes. "
    f"Inputs: {t['corpus']['generated']:,} generated, {t['corpus']['tests']:,} recorded tests, {t['corpus']['mainnet']:,} mainnet.", "",
    f"{len(larger)} measured programs do not meet the strictly-smaller ELF requirement. [Exact section/symbol evidence](gaps/round-three/size/README.md) retains each exception. "
    "[Unmeasured instruction audit](gaps/unmeasured.json) separates empty IDLs, intentional errors and unresolved generated state.", "",
    "## Measured", "", "| Program | Anchor | Gross | Net | Stock bytes | Tare bytes | Successful samples | Proof |",
    "|---|---|---:|---:|---:|---:|---:|---|"]
for row in measured:
    w = row["weighing"]
    values = [f"{w['totals'][key]:,}" if w['totals']['inputs'] else "?" for key in ['gross', 'net']]
    lines.append(f"| {row['id']} | {row['anchorVersion']} | {' | '.join(values)} | {w['builds']['stock']['sizeBytes']:,} | {w['builds']['tare']['sizeBytes']:,} | {w['totals']['inputs']:,} | {row['proof']['status']} |")
lines += ["", "## Not built", "", "| Program | Anchor | Construct or observed failure |", "|---|---|---|"]
for row in rows:
    if row['status'] != 'not_built':
        continue
    reason = row.get('construct') or next((s for s in row['reason'].splitlines() if s.strip()), row['reason'])
    reason = reason.replace('|', '&#124;').replace('`', '')
    lines.append(f"| {row['id']} | {row['anchorVersion']} | {reason}; [transcript](transcripts/{row['id']}.txt) |")
lines += ["", "## Remaining performance requirement", "", "The fixed round-two corpora are compared separately from the expanded generated inputs. [The performance ledger](gaps/round-three/performance/README.md) records every residual instruction-median increase. A matching proof does not waive either a size or compute regression.", "", "Featured terminal program: `v0_32_3-tests-idl-programs-new-idl`, with 13 measured instructions. Its command captures and hash-verified entrypoint disassembly are in `transcripts/`.", ""]
lines += ["", "## Evidence", "",
    "[Frozen toolchain sources](provenance/frozen-build.json), [fixed-corpus performance comparisons](gaps/round-three/performance/), [bytecode controls](gaps/round-three/opt/), and [registry native money tests](../registry/README.md) retain their separate provenance. "
    "Native recorded registry tests count as tests; upstream validator suites remain unrun here. The Linux entry point records those outcomes separately.", "",
    "[Lead commands and environment](README.md), [portable report corpus](../toolchain/contract-additions.md), and [registry deployment](../registry/README.md) describe the unexecuted network/container steps.", ""]
(root / 'round-three.md').write_bytes(('\n'.join(lines)).encode())
print(root / 'round-three.md')
