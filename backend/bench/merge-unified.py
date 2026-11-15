"""Validate and merge the complete frozen 0.4.0 results in manifest order."""
from pathlib import Path
import argparse
import hashlib
import json
import os
import subprocess
import sys

parser = argparse.ArgumentParser()
parser.add_argument("--publish", action="store_true")
parser.add_argument("--supplement", type=Path, action="append", default=[])
options = parser.parse_args()
root = Path(__file__).resolve().parent.parent
base = root / "bench/history/round-four"
manifest = json.loads((root / "bench/manifest.json").read_text())
paths = [base / f"final-0.4.0-{version}.json" for version in ["0.31.1", "0.32.3", "1.2.2"]]
paths += [base / "final-0.4-production.json", base / "final-0.4-registry.json"]
rows = {}
provenance = {"baseFiles": {}, "supplements": []}
for file in paths:
    report = json.loads(file.read_text())
    if report["toolchainVersion"] != "0.4.0":
        raise RuntimeError(f"Wrong toolchain version: {file}")
    provenance["baseFiles"][file.relative_to(root).as_posix()] = hashlib.sha256(file.read_bytes()).hexdigest()
    for row in report["programs"]:
        if row["id"] in rows:
            raise RuntimeError(f"Duplicate program: {row['id']}")
        rows[row["id"]] = row
expected = {p["id"] for p in manifest["programs"]}
if set(rows) != expected:
    raise RuntimeError(f"Missing {sorted(expected-set(rows))}; extra {sorted(set(rows)-expected)}")
for version in ["0.31.1", "0.32.3", "1.2.2"]:
    report = json.loads((base / f"final-0.4.0-{version}-provenance.json").read_text())
    if "exitCode" not in report:
        raise RuntimeError(f"Cohort still running: {version}")
    if report["sourceChangesDuringRun"] or report["cliChangedDuringRun"]:
        raise RuntimeError(f"Frozen source changed: {version}")
# Supplements preserve the original full-run rows and explicitly record every
# replacement after a separately documented source/CLI correction.
for file in options.supplement:
    file = file.resolve()
    supplement = json.loads(file.read_text())
    if supplement["toolchainVersion"] != "0.4.0":
        raise RuntimeError(f"Wrong supplemental toolchain version: {file}")
    sidecar = file.with_name(file.stem + "-provenance.json")
    if not sidecar.exists():
        raise RuntimeError(f"Missing supplemental provenance: {sidecar}")
    evidence = json.loads(sidecar.read_text())
    if "exitCode" not in evidence or evidence.get("sourceChangesDuringRun") or evidence.get("cliChangedDuringRun"):
        raise RuntimeError(f"Incomplete or changed supplemental run: {sidecar}")
    replacements = []
    for row in supplement["programs"]:
        id = row["id"]
        if id not in rows:
            raise RuntimeError(f"Unknown supplemental program: {id}")
        row_hash = lambda value: hashlib.sha256(json.dumps(value,sort_keys=True,separators=(",",":")).encode()).hexdigest()
        replacements.append({"id":id,"beforeRowSha256":row_hash(rows[id]),"afterRowSha256":row_hash(row)})
        rows[id] = row
    provenance["supplements"].append({"path":file.relative_to(root).as_posix(),"sha256":hashlib.sha256(file.read_bytes()).hexdigest(),"provenance":sidecar.relative_to(root).as_posix(),"provenanceSha256":hashlib.sha256(sidecar.read_bytes()).hexdigest(),"replacements":replacements})
for id, row in rows.items():
    if row["status"] != "measured":
        continue
    if row["proof"]["status"] != "matched":
        raise RuntimeError(f"Diverging proof: {id}")
    for phase in ["stock", "tare"]:
        file = root / "bench/artifacts" / id / f"{phase}.so"
        record = row["weighing"]["builds"][phase]
        if not file.exists() or file.stat().st_size != record["sizeBytes"] or hashlib.sha256(file.read_bytes()).hexdigest() != record["hash"]:
            raise RuntimeError(f"Stale artifact: {id}/{phase}")
ordered = [rows[p["id"]] for p in manifest["programs"]]
value = {"schemaVersion":"1", "fixture":False, "toolchainVersion":"0.4.0", "generatedAt":json.loads(paths[-1].read_text())["generatedAt"], "programs":ordered}
merged = base / "final-0.4.0-merged.json"
merged.write_text(json.dumps(value,indent=2)+"\n",encoding="utf-8",newline="\n")
(base / "final-0.4.0-merge-provenance.json").write_text(json.dumps(provenance,indent=2)+"\n",encoding="utf-8",newline="\n")
if not options.publish:
    print(f"Validated {len(rows)} rows; add --publish to update canonical JSON")
    raise SystemExit(0)
canonical = root / "bench/results.json"
backup = base / "canonical-before-0.4.0.json"
if not backup.exists():
    backup.write_bytes(canonical.read_bytes())
env = os.environ.copy()
env["TARE_BENCH_REFRESH"] = "1"
env["TARE_BENCH_RESULTS"] = str(merged)
result = subprocess.run(["node",str(root/"bench/run.mjs"),"--tare",str(root/"toolchain/target/tare-0.4.0-frozen.exe"),manifest["programs"][0]["id"]],cwd=root,env=env)
if result.returncode:
    raise SystemExit(result.returncode)
canonical.write_bytes(merged.read_bytes())
(root / "bench/compat.json").write_bytes(merged.with_name(merged.stem + "-compat.json").read_bytes())
print(f"Published {len(rows)} validated rows")
