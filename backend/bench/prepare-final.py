"""Stage a complete benchmark merge, exact round-three gate and measured gap ledger."""
from __future__ import annotations
import argparse
import base64
import datetime as dt
import gzip
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys

import jsonschema

ROOT = Path(__file__).resolve().parent.parent
BENCH = ROOT / "bench"
HISTORY = BENCH / "history/round-five"


def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def digest(path):
    h = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def relative(path):
    return str(Path(path).resolve().relative_to(ROOT)).replace("\\", "/")


def write(path, value):
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    Path(path).write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8", newline="\n")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def validator(name):
    return jsonschema.Draft202012Validator(read(ROOT / "contract/schema" / (name + ".json")))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--previous", type=Path, default=BENCH / "results.json")
    parser.add_argument("--cohort", type=Path, action="append")
    parser.add_argument("--supplement", type=Path, action="append", default=[])
    parser.add_argument("--promotion-provenance", type=Path)
    parser.add_argument("--gate-reports", type=Path, action="append")
    parser.add_argument("--out-dir", type=Path, default=HISTORY / "final-assembled")
    parser.add_argument("--source-helper", type=Path, default=ROOT / "toolchain/target/debug" / ("families.exe" if sys.platform == "win32" else "families"))
    parser.add_argument("--cli", type=Path, default=ROOT / "toolchain/target/tare-0.5.0-frozen.exe" if sys.platform == "win32" else ROOT / "toolchain/target/release/tare")
    parser.add_argument("--validate-replays", action="store_true", help="Also decode and hash every archived replay payload")
    args = parser.parse_args()
    cohorts = args.cohort or [HISTORY / ("final-" + version + ".json") for version in ["0.31.1", "0.32.3", "1.2.2"]]
    report_dirs = args.gate_reports or [HISTORY / ("gate-reports-" + version) for version in ["0.31.1", "0.32.3", "1.2.2"]]
    version_run = subprocess.run([str(args.cli.resolve()), "--version"], capture_output=True, text=True, check=True)
    transcript_header = "$ tare --version\n" + version_run.stdout
    out = args.out_dir.resolve()
    require(out != BENCH and out != ROOT, "Use a separate staging output directory")
    require(not (out / "results.json").exists(), "Staged results already exist; use a new output directory")
    manifest = read(BENCH / "manifest.json")
    known = {p["id"]: p for p in manifest["programs"]}
    previous = read(args.previous)
    old = {p["id"]: p for p in previous["programs"]}
    require(set(old) == set(known), "Previous results must cover the whole current manifest")
    expected = {key for key, row in old.items() if row["status"] == "measured"}
    selected = {}
    provenance = {}
    inputs = []
    versions = set()
    for index, file in enumerate(cohorts + args.supplement):
        document = read(file)
        validator("BenchResults").validate(document)
        versions.add(document["toolchainVersion"])
        inputs.append({"file": relative(file), "sha256": digest(file), "supplement": index >= len(cohorts)})
        seen = set()
        for row in document["programs"]:
            key = row["id"]
            require(key in known and key not in seen, f"Unknown/duplicate row in {file}: {key}")
            seen.add(key)
            if index < len(cohorts):
                require(key not in selected, f"Cohorts overlap: {key}")
            selected[key] = row
            provenance[key] = {"file": relative(file), "sha256": digest(file), "supplement": index >= len(cohorts)}
    require(expected <= set(selected), f"Fresh rows missing for previously measured programs: {sorted(expected - set(selected))}")
    require(len(versions) == 1, f"Mixed candidate toolchain versions: {sorted(versions)}")
    version = versions.pop()
    for key, row in old.items():
        if key not in selected:
            require(row["status"] == "not_built", f"Cannot retain a stale measured row: {key}")
            selected[key] = row
            provenance[key] = {"file": relative(args.previous), "sha256": digest(args.previous), "retainedRefusal": True, "priorToolchainVersion": previous["toolchainVersion"]}
    rows = [selected[p["id"]] for p in manifest["programs"]]
    for row in rows:
        entry = known[row["id"]]
        require(row["name"] == entry["name"] and row["anchorVersion"] == entry["anchorVersion"], f"Manifest identity differs: {row['id']}")
        require(row["source"]["commit"] == entry.get("revision", manifest.get("revision")), f"Source commit differs: {row['id']}")
    measured = [r for r in rows if r["status"] == "measured"]
    totals = {"programs": len(rows), "measured": len(measured), "notBuilt": len(rows) - len(measured), "instructions": 0, "weights": dict.fromkeys(["gross", "net", "tare", "inputs"], 0), "binarySize": dict.fromkeys(["gross", "net", "tare"], 0), "rentLamports": dict.fromkeys(["gross", "net", "tare"], 0), "corpus": dict.fromkeys(["tests", "generated", "mainnet"], 0)}
    artifacts = []
    full_proof_validator = validator("ProofReport")
    for row in measured:
        key, weighing = row["id"], row["weighing"]
        require(row["proof"]["status"] == "matched", f"Observable divergence: {key}")
        directory = BENCH / "artifacts" / key
        evidence = {"id": key, "files": {}}
        for kind in ["stock", "tare"]:
            file = directory / (kind + ".so")
            expected_build = weighing["builds"][kind]
            require(file.stat().st_size == expected_build["sizeBytes"] and digest(file) == expected_build["hash"], f"Stale {kind} ELF: {key}")
            evidence["files"][kind] = {"file": relative(file), "sha256": expected_build["hash"], "sizeBytes": file.stat().st_size}
        proof_file = directory / "proof.json"
        proof = read(proof_file)
        full_proof_validator.validate(proof)
        require(proof["builds"] == weighing["builds"], f"Proof builds differ: {key}")
        for field in ["status", "comparisons", "corpus"]:
            require(proof["proof"][field] == row["proof"][field], f"Proof {field} differs: {key}")
        require(proof["proof"].get("firstDivergence") is None, f"Native divergence: {key}")
        if args.validate_replays:
            replay = proof["replay"]
            require(replay["encoding"] == "gzip+base64", f"Replay encoding: {key}")
            # Same bounded uncompressed limit as the CLI/schema validator.
            import io
            with gzip.GzipFile(fileobj=io.BytesIO(base64.b64decode(replay["data"], validate=True))) as stream:
                payload = stream.read(256 * 1024 * 1024 + 1)
            require(len(payload) <= 256 * 1024 * 1024, f"Oversized replay: {key}")
            require(hashlib.sha256(payload).hexdigest() == replay["sha256"] == row["proof"]["corpus"]["hash"], f"Replay hash differs: {key}")
            json.loads(payload)
        evidence["files"]["proof"] = {"file": relative(proof_file), "sha256": digest(proof_file), "replayDecoded": args.validate_replays}
        artifacts.append(evidence)
        totals["instructions"] += len(weighing["instructions"])
        for field in totals["weights"]:
            totals["weights"][field] += weighing["totals"][field] or 0
        for field, source in [("binarySize", "sizeBytes"), ("rentLamports", "rentLamports")]:
            gross, net = (weighing["builds"][kind][source] for kind in ["stock", "tare"])
            totals[field]["gross"] += gross
            totals[field]["net"] += net
            totals[field]["tare"] += gross - net
        for field in totals["corpus"]:
            totals["corpus"][field] += row["proof"]["corpus"]["counts"][field]
    now = dt.datetime.now(dt.timezone.utc).isoformat(timespec="milliseconds").replace("+00:00", "Z")
    merged = {"schemaVersion": "1", "fixture": False, "toolchainVersion": version, "generatedAt": now, "programs": rows, "totals": totals}
    anchors = list(dict.fromkeys(row["anchorVersion"] for row in rows))
    construct_names = sorted(set(c for p in manifest["programs"] for c in p["constructs"]) | {r["construct"] for r in rows if r["status"] == "not_built" and r.get("construct")})
    constructs = []
    for name in construct_names:
        entries = []
        for anchor in anchors:
            exercises = [r["id"] for r in measured if r["anchorVersion"] == anchor and r["weighing"]["totals"]["inputs"] > 0 and name in known[r["id"]]["constructs"]]
            refusals = [r for r in rows if r["anchorVersion"] == anchor and r["status"] == "not_built" and r.get("construct") == name]
            note = "Compiled in programs with matched proofs and successful instruction samples; this is not branch coverage or full language conformance." if exercises else "\n".join(f"{r['id']}: {r['reason']}" for r in refusals) or "Not exercised by a matched corpus run."
            entries.append({"anchorVersion": anchor, "supported": bool(exercises), "programs": exercises, "note": note})
        constructs.append({"construct": name, "versions": entries})
    compat = {"schemaVersion": "1", "fixture": False, "toolchainVersion": version, "generatedAt": now, "anchorVersions": anchors, "constructs": constructs}
    validator("BenchResults").validate(merged)
    validator("Compat").validate(compat)
    write(out / "results.json", merged)
    write(out / "compat.json", compat)
    write(out / "merge-provenance.json", {"schemaVersion": 1, "previous": {"file": relative(args.previous), "sha256": digest(args.previous)}, "inputs": inputs, "promotionProvenance": {"file": relative(args.promotion_provenance), "sha256": digest(args.promotion_provenance)} if args.promotion_provenance else None, "command": sys.argv, "cli": {"file": relative(args.cli), "sha256": digest(args.cli)}, "rows": provenance, "artifacts": artifacts, "priorRefusals": sum(r["status"] == "not_built" for r in old.values()), "retainedRefusals": sum(p.get("retainedRefusal", False) for p in provenance.values())})
    baseline_file = BENCH / "history/round-three/final-preserved/results.json"
    baseline_hash = digest(baseline_file)
    baseline = read(baseline_file)
    gate_dir = out / "gate-reports"
    gate_dir.mkdir(parents=True, exist_ok=True)
    choices = []
    for old_row in baseline["programs"]:
        if old_row["status"] != "measured":
            continue
        key = old_row["id"]
        current = selected[key]
        candidates = []
        if current["status"] == "measured":
            for directory in report_dirs:
                file = directory / (key + ".json")
                if file.exists():
                    report = read(file)
                    if report.get("gate") == {"baselineResultsSha256": baseline_hash, "candidateSha256": current["weighing"]["builds"]["tare"]["hash"], "proofCorpusHash": old_row["proof"]["corpus"]["hash"]}:
                        candidates.append(file)
        if candidates:
            # Exact hash matching selects evidence; later explicitly supplied directories win.
            file = candidates[-1]
            shutil.copyfile(file, gate_dir / (key + ".json"))
            choices.append({"id": key, "file": relative(file), "sha256": digest(file)})
    write(out / "gate-evidence.json", choices)
    gate = subprocess.run(["node", str(BENCH / "gate.mjs"), "--summary", "--results", str(out / "results.json"), "--reports", str(gate_dir), "--output", str(out / "gate.json")], cwd=ROOT, capture_output=True, text=True)
    require(gate.returncode in [0, 1], gate.stderr)
    (out / "gate.txt").write_text(transcript_header + "$ node bench/gate.mjs --summary [staged results and exact reports]\n" + gate.stdout + gate.stderr, encoding="utf-8", newline="\n")
    historical = subprocess.run(["node", str(BENCH / "gate.mjs"), "--against-round-three", "--summary", "--results", str(out / "results.json"), "--reports", str(gate_dir), "--output", str(out / "round-three-comparison.json")], cwd=ROOT, capture_output=True, text=True)
    (out / "round-three-comparison.txt").write_text(historical.stdout + historical.stderr, encoding="utf-8", newline="\n")
    report = read(out / "gate.json")
    compute_gaps = [{"id": r["id"], **gap, "evidence": {"result": relative(out / "results.json"), "proof": "bench/artifacts/" + r["id"] + "/proof.json"}, "cause": "Source and ELF evidence pending."} for r in report["programs"] for gap in r["regressions"]]
    compute_gaps.sort(key=lambda r: (-r["increase"], r["id"], r["instruction"]))
    size_gaps = [{"id": r["id"], "stock": r["stockBytes"], "candidate": r["candidateBytes"], "increase": r["candidateBytes"] - r["stockBytes"], "evidence": {"result": relative(out / "results.json"), "proof": "bench/artifacts/" + r["id"] + "/proof.json"}, "cause": "Source and ELF evidence pending."} for r in report["programs"] if r["candidateBytes"] is not None and r["candidateBytes"] > r["stockBytes"]]
    size_gaps.sort(key=lambda r: (-r["increase"], r["id"]))
    stock_gaps = [{"id": r["id"], "instruction": ix["name"], "stock": ix["gross"], "tare": ix["net"], "increase": ix["net"] - ix["gross"], "inputs": ix["inputs"]} for r in measured for ix in r["weighing"]["instructions"] if ix["inputs"] and ix["net"] > ix["gross"]]
    stock_gaps.sort(key=lambda r: (-r["increase"], r["id"], r["instruction"]))
    unmeasured = [{"id": r["id"], "instruction": ix["name"], "reason": "No successful input in this measured corpus; no compute estimate emitted.", "corpusHash": r["proof"]["corpus"]["hash"]} for r in measured for ix in r["weighing"]["instructions"] if not ix["inputs"]]
    ledger = {"schemaVersion": 1, "resultsSha256": digest(out / "results.json"), "gateSha256": digest(out / "gate.json"), "counts": {"stockInstructionFailures": len(compute_gaps), "stockSizeFailures": len(size_gaps), "stockComputeRegressions": len(stock_gaps), "unmeasuredInstructions": len(unmeasured)}, "compute": compute_gaps, "size": size_gaps, "aboveStock": stock_gaps, "unmeasured": unmeasured}
    write(out / "gaps.json", ledger)
    causes = subprocess.run([sys.executable, str(BENCH / "gaps/round-five-gaps.py"), "--results", str(out / "results.json"), "--gate", str(out / "gate.json"), "--out", str(out / "gap-causes.json"), "--source-helper", str(args.source_helper)], cwd=ROOT, capture_output=True, text=True)
    require(causes.returncode == 0, causes.stderr or causes.stdout)
    annotations = {r["id"]: r for r in read(out / "gap-causes.json")["programs"]}
    for row in ledger["compute"] + ledger["size"]:
        row["cause"] = annotations[row["id"]]["cause"]
        row["causeEvidence"] = relative(out / "gap-causes.json")
    write(out / "gaps.json", ledger)
    summary = f"Staged {len(rows)} programs: {len(measured)} measured, {len(rows)-len(measured)} not built; all measured proofs matched.\n" + gate.stdout
    (out / "merge.txt").write_text(transcript_header + "$ python bench/prepare-final.py [arguments recorded in merge-provenance.json]\n" + summary, encoding="utf-8", newline="\n")
    print(summary, end="")
    # A failing gate remains an unsuccessful final preparation, with all evidence retained.
    return gate.returncode


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError, jsonschema.ValidationError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(2)
