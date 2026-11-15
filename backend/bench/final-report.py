"""Render measured round-four facts without replacing the report before --publish."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import subprocess
import sys

BENCH = Path(__file__).resolve().parent
ROOT = BENCH.parent


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def link(path, base):
    import os
    return Path(os.path.relpath(path, base)).as_posix()


def cell(value):
    return str(value).replace("|", "&#124;").replace("\n", " ")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--results", type=Path, default=BENCH / "results.json")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--publish", action="store_true")
    parser.add_argument("--run-audits", action="store_true")
    args = parser.parse_args()
    result_path = args.results.resolve()
    output = args.output or (BENCH / "round-four.md" if args.publish else BENCH / "gaps/round-four/final-report-preview.md")
    output = output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    facts_path = output.with_suffix(".json")
    unmeasured_path = output.with_name(output.stem + "-unmeasured.json")
    manifest = read(BENCH / "manifest.json")
    entries = {p["id"]: p for p in manifest["programs"]}
    document = read(result_path)
    rows = document["programs"]
    measured = [r for r in rows if r["status"] == "measured"]
    ids = [r["id"] for r in rows]
    issues = []
    if len(ids) != len(set(ids)): issues.append("Duplicate program rows")
    missing = sorted(set(entries) - set(ids))
    extra = sorted(set(ids) - set(entries))
    if missing: issues.append(f"Missing {len(missing)} manifest programs")
    if extra: issues.append(f"Unknown program IDs: {extra}")
    versions = sorted({r.get("weighing", {}).get("toolchainVersion", document.get("toolchainVersion")) for r in rows})
    totals = {"programs": len(rows), "measured": len(measured), "notBuilt": len(rows)-len(measured), "instructions":0,
              "weights":dict.fromkeys(["gross", "net", "tare", "inputs"], 0),
              "binarySize":dict.fromkeys(["gross", "net", "tare"], 0),
              "rentLamports":dict.fromkeys(["gross", "net", "tare"], 0),
              "corpus":dict.fromkeys(["tests", "generated", "mainnet"], 0)}
    size_exceptions, regressions, artifact_checks = [], [], []
    for row in rows:
        entry = entries.get(row["id"])
        if not entry: continue
        if row["anchorVersion"] != entry["anchorVersion"] or row["source"] != {"repository":entry.get("repository"), "commit":entry.get("revision")}:
            issues.append(f"Manifest identity mismatch: {row['id']}")
        if row["status"] != "measured": continue
        w = row["weighing"]
        if row["proof"]["status"] != "matched": issues.append(f"Observable divergence: {row['id']}")
        if entry.get("mainnetAddress") and w["program"]["address"] != entry["mainnetAddress"]:
            issues.append(f"Mainnet address mismatch: {row['id']}")
        totals["instructions"] += len(w["instructions"])
        for key in totals["weights"]: totals["weights"][key] += w["totals"][key]
        for group, field in [("binarySize", "sizeBytes"), ("rentLamports", "rentLamports")]:
            gross, net = w["builds"]["stock"][field], w["builds"]["tare"][field]
            totals[group]["gross"] += gross; totals[group]["net"] += net; totals[group]["tare"] += gross-net
        for key in totals["corpus"]: totals["corpus"][key] += row["proof"]["corpus"]["counts"][key]
        stock, net = w["builds"]["stock"]["sizeBytes"], w["builds"]["tare"]["sizeBytes"]
        if net >= stock: size_exceptions.append({"program":row["id"], "stock":stock, "tare":net, "increase":net-stock})
        for ix in w["instructions"]:
            if ix["inputs"] and ix["net"] > ix["gross"]:
                regressions.append({"program":row["id"], "instruction":ix["name"], "gross":ix["gross"], "net":ix["net"], "increase":ix["net"]-ix["gross"], "inputs":ix["inputs"]})
        archive = BENCH / "artifacts" / row["id"]
        for kind in ["stock", "tare"]:
            path = archive / f"{kind}.so"; expected = w["builds"][kind]
            match = path.exists() and path.stat().st_size == expected["sizeBytes"] and sha(path) == expected["hash"]
            artifact_checks.append({"program":row["id"], "kind":kind, "matches":match})
            if not match: issues.append(f"Stale or missing {kind} ELF: {row['id']}")
    if totals != document["totals"]: issues.append("Published totals differ from recomputed rows")
    command = [sys.executable, str(BENCH / "gaps/audit-unmeasured.py"), str(result_path), str(unmeasured_path)]
    subprocess.run(command, cwd=ROOT, check=True, capture_output=True)
    unmeasured = read(unmeasured_path)
    fixed_path = BENCH / "gaps/round-four/scenario-fix/fixed-bumps.json"
    fixed = {(x["program"], x["instruction"]):x for x in read(fixed_path)["programs"]} if fixed_path.exists() else {}
    for row in unmeasured["instructions"]:
        evidence = fixed.get((row["program"], row["instruction"]))
        if evidence and sha(ROOT / evidence["source"]["path"]) == evidence["source"]["sha256"]:
            row.update(category="source_constraints_impossible", reason=evidence["reason"], source=evidence["source"], evidence=link(fixed_path, output.parent))
    coverage_path = BENCH / "production/coverage-notes.json"
    coverage = read(coverage_path)
    production_reasons = {(f"production-{profile}", item["instruction"]):item for profile, detail in coverage.items() for item in detail.get("unmeasuredInstructions", [])}
    for profile, detail in coverage.items():
        for instruction, value in detail.get("unmeasured", {}).items():
            production_reasons[(f"production-{profile}",instruction)] = {
                "category":"unreachable_by_source" if value["reason"] == "unconditional source error" else "fixed_production_authority_unavailable",
                "reason":value["reason"], "source":value["source"],
                "observedBaselineError":f"InstructionError(0, Custom({value['observedError']}))"}
    for row in unmeasured["instructions"]:
        detail = production_reasons.get((row["program"], row["instruction"]))
        if detail:
            row.update({key:value for key,value in detail.items() if key != "instruction"})
    category_counts = dict(Counter(r["category"] for r in unmeasured["instructions"]))
    unmeasured["categoryCounts"] = category_counts
    unmeasured_path.write_text(json.dumps(unmeasured, indent=2) + "\n", encoding="utf-8")
    audit = None
    if args.run_audits:
        audit_path = output.with_name(output.stem + "-audit.json")
        run = subprocess.run([sys.executable, str(BENCH / "gaps/audit-final.py"), str(result_path), "--output", str(audit_path)], cwd=ROOT)
        audit = {"path":link(audit_path, output.parent), "passed":run.returncode == 0}
        if run.returncode: issues.append("Full schema/artifact/source audit failed")
    baseline_path = BENCH / "history/round-four/final-0.4.0-baseline/summary.json"
    supplement_path = BENCH / "history/round-four/final-0.4.0-supplement-baseline/summary.json"
    baseline = {"path":link(baseline_path,output.parent), "supplement":link(supplement_path,output.parent), "programs":[], "staleOrMissing":[], "unavailable":[]}
    current = {r["id"]:r for r in measured}
    comparisons = {}
    for index in [baseline_path, supplement_path]:
        if not index.exists(): continue
        for comparison in read(index)["programs"]:
            comparisons[comparison["id"]] = (comparison,index.parent)
    for ident,(comparison,directory) in comparisons.items():
        row = current.get(ident)
        if not comparison.get("afterSha256"):
            baseline["unavailable"].append(comparison); continue
        report = directory / (ident + ".json")
        if row is None or row["weighing"]["builds"]["tare"]["hash"] != comparison["afterSha256"]:
            baseline["staleOrMissing"].append(ident); continue
        if not report.exists() or sha(report) != comparison.get("nativeReportSha256"):
            baseline["staleOrMissing"].append(ident); continue
        native = read(report)
        if native.get("firstDivergence") is not None: issues.append(f"Old-corpus observable divergence: {ident}")
        baseline["programs"].append({**comparison,"archive":link(directory,output.parent)})
    baseline["counts"] = dict(Counter(x["status"] for x in baseline["programs"]))
    baseline["instructionsRegressed"] = sum(len(x.get("medianRegressions",[])) for x in baseline["programs"])
    facts = {"results":link(result_path, output.parent), "resultsSha256":sha(result_path), "manifestSha256":sha(BENCH/"manifest.json"), "complete":not missing and not extra,
             "versions":versions, "totals":totals, "missing":missing, "issues":issues, "artifactChecks":artifact_checks,
             "sizeExceptions":size_exceptions, "computeRegressionsAgainstStock":regressions, "unmeasuredCategories":category_counts,
             "unmeasured":link(unmeasured_path, output.parent), "audit":audit, "oldCorpusBaseline":baseline}
    facts_path.write_text(json.dumps(facts, indent=2) + "\n", encoding="utf-8")
    if args.publish and (issues or versions != ["0.4.0"] or not args.run_audits):
        raise SystemExit("Refusing to publish: final rows must be complete, current 0.4.0, and pass full --run-audits schema/artifact/source checks; see JSON facts")
    proof_inputs = sum(r["proof"]["comparisons"]["inputs"] for r in measured)
    lines = ["# Tare round four", "", f"Tare {document['toolchainVersion']}; {document['generatedAt']}. This report reads [{result_path.name}]({link(result_path,output.parent)}) and its archived artifacts. Totals sum successful per-instruction medians; they are not transaction-sequence totals.", "",
             f"{totals['programs']} of {len(entries)} manifest programs are recorded: {totals['measured']} measured and {totals['notBuilt']} not built. {sum(r['proof']['status']=='matched' for r in measured)} proofs match across {proof_inputs:,} compared transactions. Corpus counts: {totals['corpus']['generated']:,} generated, {totals['corpus']['tests']:,} recorded tests, {totals['corpus']['mainnet']:,} mainnet.", "",
             f"Compute: {totals['weights']['gross']:,} gross / {totals['weights']['net']:,} net / {totals['weights']['tare']:,} tare CU from {totals['weights']['inputs']:,} successful samples. ELF bytes: {totals['binarySize']['gross']:,} stock / {totals['binarySize']['net']:,} Tare. Rent lamports: {totals['rentLamports']['gross']:,} stock / {totals['rentLamports']['net']:,} Tare.", ""]
    if issues: lines += ["This is a preview, not a completed final report: " + str(len(issues)) + " pending identity/artifact checks; see the JSON facts for every issue.", ""]
    lines += ["## Production programs", "", "| Program | Status | Gross CU | Net CU | Successful instructions | Compared inputs |", "|---|---|---:|---:|---:|---:|"]
    for row in rows:
        if not row["id"].startswith("production-"): continue
        if row["status"] == "measured":
            w=row["weighing"]; lines.append(f"| {row['id']} | {row['proof']['status']} | {w['totals']['gross']:,} | {w['totals']['net']:,} | {sum(x['inputs']>0 for x in w['instructions'])}/{len(w['instructions'])} | {row['proof']['comparisons']['inputs']} |")
        else: lines.append(f"| {row['id']} | {cell(row.get('construct') or row['reason'].splitlines()[0])} | ? | ? | ? | ? |")
    lines += ["", "Program addresses are checked through `weighing.program.address`. These native source builds do not claim equality with deployed mainnet binaries. Directed production scenarios retain explicit config snapshots and real System/SPL initialization transactions.", "", "## Remaining measured gaps", "",
              f"{len(size_exceptions)} measured programs fail the strictly-smaller ELF requirement. {len(regressions)} instruction medians in {len(set(x['program'] for x in regressions))} programs exceed stock. Every difference remains recorded; matching observable behavior does not waive a performance regression.", "", "| Program | Stock bytes | Tare bytes | Increase |", "|---|---:|---:|---:|"]
    for x in size_exceptions: lines.append(f"| {x['program']} | {x['stock']:,} | {x['tare']:,} | {x['increase']:,} |")
    lines += ["", "| Program | Instruction | Gross CU | Net CU | Increase | Successful samples |", "|---|---|---:|---:|---:|---:|"]
    for x in regressions: lines.append(f"| {x['program']} | {x['instruction']} | {x['gross']:,} | {x['net']:,} | {x['increase']:,} | {x['inputs']} |")
    lines += ["", "## Frozen old-corpus comparisons", "", f"{len(baseline['programs'])} native baseline reports match the current Tare ELF hash and their recorded native-report hash. {baseline.get('instructionsRegressed',0)} instruction medians regress on those unchanged old inputs. {len(baseline['staleOrMissing'])} baseline records do not yet match current artifacts and are excluded from this count.", "", "| Program | Instruction | Prior Tare CU | Current Tare CU | Increase |", "|---|---|---:|---:|---:|"]
    for comparison in baseline["programs"]:
        for ix in comparison["medianRegressions"]:
            lines.append(f"| {comparison['id']} | {ix['instruction']} | {ix['before']:,} | {ix['after']:,} | {ix['increase']:,} |")
    lines += ["", "## Instructions without successful inputs", "", f"{len(unmeasured['instructions'])} instruction rows remain unmeasured. Categories: " + ", ".join(f"{k}={v}" for k,v in sorted(category_counts.items())) + ".", "",
              "The source-resolved misc scenarios retain the entire original corpus and add client-derived account relationships, explicit signer/payer roles, rent-plus-debit funding, and real SPL mint setup. The targeted controls show 39 former gaps gaining successful inputs; the final rows above remain the measurement authority. The three fixed-bump `test_used_identifiers` rows require an on-curve bump4 address under fixed argument constraints; their exact source and native derivation evidence classify them as source-impossible. All failure inputs remain in the proof corpus.", "",
              f"[Per-instruction reasons and source evidence]({link(unmeasured_path,output.parent)}) distinguish source failures, upstream IDL mismatches, and unresolved generated state. [Generator control evidence]({link(BENCH/'gaps/round-four/scenario-fix/summary.json',output.parent)}) retains the old self-close defect and corrected comparisons.", "", "## Not built", "", "| Program | Anchor | Construct or observed failure |", "|---|---|---|"]
    for row in rows:
        if row["status"] == "measured": continue
        reason = row.get("construct") or row["reason"].splitlines()[0]
        lines.append(f"| {row['id']} | {row['anchorVersion']} | {cell(reason)}; [transcript]({link(BENCH/'transcripts'/f"{row['id']}.txt",output.parent)}) |")
    lines += ["", "## Evidence and lead runs", "", "Stock-relative regressions above use the current corpus. Frozen old-corpus baseline comparisons are separate evidence and must not be confused with those values; see the round-four baseline reports under `history/round-four/final-0.4.0-baseline/`.", "", f"[Exact report facts and artifact checks]({link(facts_path,output.parent)}) retain current hashes and all exceptions. [Production provenance]({link(BENCH/'history/round-four/final-0.4-production/provenance.json',output.parent)}) identifies the frozen compiler and source trees. [Native flow evidence]({link(BENCH/'rehearsal/chain-verification-0.4.json',output.parent)}) and [reproducibility evidence]({link(BENCH/'repro',output.parent)}) remain separate from benchmark totals.", "",
              f"Docker, a validator, and public-chain transactions are not run on this Windows host. The lead runs the exact image, validator, registry, and API commands in [lead-run.md]({link(BENCH/'lead-run.md',output.parent)}). Recorded validator/mainnet counts are reported only when present in the final files.", ""]
    output.write_text("\n".join(lines), encoding="utf-8")
    print(json.dumps({"output":str(output), "facts":str(facts_path), "programs":len(rows), "complete":not missing and not extra, "issues":len(issues), "sizeExceptions":len(size_exceptions), "instructionRegressions":len(regressions)}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
