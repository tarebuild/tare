"""Render round-five facts from results and hash-matched gate evidence; never fill missing measurements."""
from collections import Counter
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import jsonschema

BENCH = Path(__file__).resolve().parent
ROOT = BENCH.parent


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def link(path, folder):
    return Path(os.path.relpath(path, folder)).as_posix()


def cell(text):
    return str(text).replace("|", "&#124;").replace("\n", " ")


def number(value):
    return "not measured" if value is None else f"{value:,}"


def totals(rows):
    measured = [r for r in rows if r["status"] == "measured"]
    value = {"programs": len(rows), "measured": len(measured), "notBuilt": len(rows)-len(measured), "instructions": 0,
             "weights": dict.fromkeys(["gross", "net", "tare", "inputs"], 0),
             "binarySize": dict.fromkeys(["gross", "net", "tare"], 0),
             "rentLamports": dict.fromkeys(["gross", "net", "tare"], 0),
             "corpus": dict.fromkeys(["tests", "generated", "mainnet"], 0)}
    for r in measured:
        w = r["weighing"]
        value["instructions"] += len(w["instructions"])
        for key in value["weights"]: value["weights"][key] += w["totals"][key] or 0
        for group, field in [("binarySize", "sizeBytes"), ("rentLamports", "rentLamports")]:
            gross, net = w["builds"]["stock"][field], w["builds"]["tare"][field]
            value[group]["gross"] += gross
            value[group]["net"] += net
            value[group]["tare"] += gross-net
        for key in value["corpus"]: value["corpus"][key] += r["proof"]["corpus"]["counts"][key]
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--results", type=Path, default=BENCH / "results.json")
    parser.add_argument("--gate", type=Path, default=BENCH / "history/round-five/final-assembled/gate.json")
    parser.add_argument("--causes", type=Path, default=BENCH / "history/round-five/final-assembled/gap-causes.json")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--publish", action="store_true")
    parser.add_argument("--round", default="five", choices=["five","six"])
    args = parser.parse_args()
    output = (args.output or (BENCH / f"round-{args.round}.md" if args.publish else BENCH / f"history/round-{args.round}/report-preview.md")).resolve()
    if output == (BENCH / f"round-{args.round}.md").resolve() and not args.publish:
        raise SystemExit("Use --publish to replace the final report")
    output.parent.mkdir(parents=True, exist_ok=True)
    document = read(args.results)
    rows = document["programs"]
    manifest = {p["id"]: p for p in read(BENCH / "manifest.json")["programs"]}
    baseline_file = BENCH / "history/round-three/final-preserved/results.json"
    old = {p["id"]: p for p in read(baseline_file)["programs"] if p["status"] == "measured"}
    measured = [r for r in rows if r["status"] == "measured"]
    measured_by_id = {r["id"]: r for r in measured}
    by_id = {r["id"]: r for r in rows}
    issues = []
    schema = read(ROOT / "contract/schema/BenchResults.json")
    errors = list(jsonschema.Draft202012Validator(schema).iter_errors(document))
    if errors: issues.append("BenchResults schema: " + jsonschema.exceptions.best_match(errors).message)
    if len(rows) != len(by_id) or set(by_id) != set(manifest): issues.append("Result IDs do not uniquely cover the current manifest")
    computed = totals(rows)
    if computed != document["totals"]: issues.append("Published totals differ from recomputed rows")
    for row in measured:
        if row["proof"]["status"] != "matched": issues.append(f"Observable divergence: {row['id']}")
        address = manifest[row["id"]].get("mainnetAddress")
        if address and row["weighing"]["program"]["address"] != address: issues.append(f"Source program address differs: {row['id']}")
    current_size_gaps = sorted([{"id": r["id"], "stock": r["weighing"]["builds"]["stock"]["sizeBytes"], "tare": r["weighing"]["builds"]["tare"]["sizeBytes"], "increase": r["weighing"]["builds"]["tare"]["sizeBytes"] - r["weighing"]["builds"]["stock"]["sizeBytes"]} for r in measured if r["weighing"]["builds"]["tare"]["sizeBytes"] > r["weighing"]["builds"]["stock"]["sizeBytes"]], key=lambda r: (-r["increase"], r["id"]))
    artifact_checks = []
    if args.publish:
        for row in measured:
            for kind in ["stock", "tare"]:
                file = BENCH / "artifacts" / row["id"] / (kind + ".so")
                build = row["weighing"]["builds"][kind]
                matched = file.exists() and file.stat().st_size == build["sizeBytes"] and sha(file) == build["hash"]
                artifact_checks.append({"id": row["id"], "kind": kind, "matched": matched})
                if not matched: issues.append(f"Stale {kind} ELF: {row['id']}")
    gate = read(args.gate) if args.gate.exists() else None
    gate_valid = gate is not None and gate.get("comparison") == "current_stock" and gate.get("candidateResultsSha256") == sha(args.results)
    if gate is not None and not gate_valid: issues.append("Current-stock gate does not match the result file hash")
    gate_rows = {r["id"]: r for r in gate["programs"]} if gate_valid else {}
    complete = gate_valid and set(gate_rows) == set(measured_by_id) and not gate.get("coverageIssues")
    gate_status = "incomplete" if not complete else "passed" if gate["passed"] else "failed"
    causes = read(args.causes) if args.causes.exists() else None
    causes_valid = causes is not None and gate_valid and causes.get("resultsSha256") == sha(args.results) and causes.get("gateSha256") == sha(args.gate)
    annotations = {r["id"]: r for r in causes["programs"]} if causes_valid else {}
    if causes is not None and not causes_valid: issues.append("Gap-cause artifact does not match the result/gate file hashes")
    failures = [r for r in gate_rows.values() if not r["passed"]]
    missing_causes = [r["id"] for r in failures if r["id"] not in annotations]
    if missing_causes: issues.append(f"Missing source/ELF cause evidence for {len(missing_causes)} gate failures")
    failures.sort(key=lambda r: (-max([x["increase"] for x in r["regressions"]] or [0]), -(r.get("candidateBytes") or 0) + r["stockBytes"], r["id"]))
    size_failures = sorted([r for r in failures if r.get("candidateBytes") is not None and r["candidateBytes"] > r["stockBytes"]], key=lambda r: (r["stockBytes"] - r["candidateBytes"], r["id"]))
    exact_rows = []
    historical_path = args.gate.with_name("round-three-comparison.json")
    historical = read(historical_path) if historical_path.exists() else None
    historical_valid = historical is not None and historical.get("candidateResultsSha256") == sha(args.results) and historical.get("baselineResultsSha256") == sha(baseline_file)
    for item in historical.get("programs", []) if historical_valid else []:
        if not item.get("evidence"): continue
        path = Path(item["evidence"]["file"])
        if not path.is_absolute(): path = ROOT / path
        if not path.exists() or sha(path) != item["evidence"]["sha256"]: continue
        report = read(path)
        exact_rows.append({"id": item["id"], "comparisons": report["comparisons"]["inputs"], "gross": sum(x["gross"] or 0 for x in report["instructions"] if x["inputs"]), "net": sum(x["net"] or 0 for x in report["instructions"] if x["inputs"]), "roundThree": old[item["id"]]["weighing"]["totals"]["net"]})
    # Build fresh reasons only for instructions that remain unmeasured now.
    unmeasured_path = output.with_name(output.stem + "-unmeasured.json")
    process = subprocess.run([sys.executable, str(BENCH / "gaps/audit-unmeasured.py"), str(args.results.resolve()), str(unmeasured_path)], cwd=ROOT, capture_output=True, text=True)
    if process.returncode: raise RuntimeError(process.stderr)
    unmeasured = read(unmeasured_path)
    fixed_path = BENCH / "gaps/round-four/scenario-fix/fixed-bumps.json"
    fixed = {(x["program"], x["instruction"]): x for x in read(fixed_path)["programs"]} if fixed_path.exists() else {}
    coverage_path = BENCH / "production/coverage-notes.json"
    coverage = read(coverage_path) if coverage_path.exists() else {}
    production_reasons = {}
    for profile, details in coverage.items():
        for item in details.get("unmeasuredInstructions", []): production_reasons[("production-"+profile, item["instruction"])] = item
        for name, item in details.get("unmeasured", {}).items():
            production_reasons[("production-"+profile, name)] = {"category": "unreachable_by_source" if item["reason"] == "unconditional source error" else "fixed_production_authority_unavailable", **item}
    for item in unmeasured["instructions"]:
        key = (item["program"], item["instruction"])
        evidence = fixed.get(key)
        if evidence and sha(ROOT / evidence["source"]["path"]) == evidence["source"]["sha256"]:
            item.update(category="source_constraints_impossible", reason=evidence["reason"], source=evidence["source"], evidence=link(fixed_path, output.parent))
        evidence = production_reasons.get(key)
        if evidence: item.update({k: v for k, v in evidence.items() if k != "instruction"})
    unmeasured["categoryCounts"] = dict(Counter(r["category"] for r in unmeasured["instructions"]))
    unmeasured_path.write_text(json.dumps(unmeasured, indent=2) + "\n", encoding="utf-8", newline="\n")
    above_stock = [{"id":r["id"],"instruction":ix["name"],"gross":ix["gross"],"net":ix["net"],"increase":ix["net"]-ix["gross"],"inputs":ix["inputs"]} for r in measured for ix in r["weighing"]["instructions"] if ix["inputs"] and ix["net"] > ix["gross"]]
    above_stock.sort(key=lambda x: (-x["increase"], x["id"], x["instruction"]))
    facts = {"results":link(args.results.resolve(),output.parent),"resultsSha256":sha(args.results),"gate":link(args.gate.resolve(),output.parent),"gateStatus":gate_status,"gateComplete":complete,"gapCausesMatched":causes_valid,"totals":computed,"issues":issues,"artifactChecks":artifact_checks,"exactInputTotals":{"programs":len(exact_rows),"comparedTransactions":sum(x["comparisons"] for x in exact_rows),"roundThreeNet":sum(x["roundThree"] for x in exact_rows),"currentGross":sum(x["gross"] for x in exact_rows),"currentNet":sum(x["net"] for x in exact_rows)},"currentSizeGaps":current_size_gaps,"unmeasuredCategories":unmeasured["categoryCounts"],"aboveStock":above_stock,"remainingPrograms":[r["id"] for r in failures]}
    output.with_suffix(".json").write_text(json.dumps(facts,indent=2)+"\n",encoding="utf-8",newline="\n")
    if args.publish and (issues or not complete):
        raise SystemExit("Refusing final report publication: " + "; ".join(issues or ["current-stock gate is incomplete"]))
    proof_inputs = sum(r["proof"]["comparisons"]["inputs"] for r in measured)
    lines = [f"# Tare round {args.round}", "", f"Tare {document['toolchainVersion']}; report source [{args.results.name}]({link(args.results.resolve(),output.parent)}). All CU totals sum measured per-instruction medians, not transaction-sequence costs.", "", f"Recorded programs: {computed['programs']}/{len(manifest)}; measured {computed['measured']}; not built {computed['notBuilt']}. Matched proofs: {sum(r['proof']['status']=='matched' for r in measured)}/{len(measured)}, comparing {number(proof_inputs)} transactions.", "", f"Current corpus: gross {number(computed['weights']['gross'])} / net {number(computed['weights']['net'])} / tare {number(computed['weights']['tare'])} CU over {number(computed['weights']['inputs'])} successful samples. Stock/Tare ELF bytes: {number(computed['binarySize']['gross'])} / {number(computed['binarySize']['net'])}. Test/generated/mainnet inputs: {number(computed['corpus']['tests'])} / {number(computed['corpus']['generated'])} / {number(computed['corpus']['mainnet'])}.", "", "## Current-stock acceptance gate", "", f"**Gate {gate_status.upper()}.** Every measured program must have a matched proof and a Tare ELF no larger than its current stock build. Every reached instruction requires net <= gross; production instructions require net < gross. All {len(manifest)} manifest programs need an explicit result.", ""]
    if gate_valid:
        c=gate["counts"];lines += [f"{c['passed']}/{c['programs']} measured programs pass; {c['sizeRegressions']} size failures and {c['computeRegressions']} instruction-CU failures remain. {c['notBuilt']} programs are explicitly not built.", ""]
    else: lines += ["A matching complete current-stock gate file is unavailable.", ""]
    e=facts["exactInputTotals"]
    lines += [f"Hash-verified old-input evidence covers {e['programs']} programs and {number(e['comparedTransactions'])} compared transactions. For those programs only: round-three net {number(e['roundThreeNet'])}; candidate net {number(e['currentNet'])}; current-stock gross {number(e['currentGross'])} CU. This historical comparison is informational only and never changes current-stock acceptance.", "", "## Production and registry", "", "| Program | Status | Current gross/net CU | Round-three net / exact current net | Stock/Tare bytes | Round-three bytes | Successful methods | Proof inputs | Tests passed/ran |", "|---|---|---:|---:|---:|---:|---:|---:|---:|"]
    exact_by_id={x["id"]:x for x in exact_rows}
    for row in rows:
        if row["kind"] != "production" and row["id"] != "tare-registry":continue
        ident=row["id"]
        if row["status"] != "measured":lines.append(f"| {ident} | {cell(row.get('construct') or row['reason'].splitlines()[0])} | not measured | not measured | not measured | not measured | not measured | 0 | 0/0 |");continue
        w=row["weighing"];b=old.get(ident);e=exact_by_id.get(ident);tests=row["tests"]
        lines.append(f"| {ident} | {row['proof']['status']} | {number(w['totals']['gross'])}/{number(w['totals']['net'])} | {number(b['weighing']['totals']['net']) if b else 'outside round three'} / {number(e['net']) if e else 'not replayed'} | {number(w['builds']['stock']['sizeBytes'])}/{number(w['builds']['tare']['sizeBytes'])} | {number(b['weighing']['builds']['tare']['sizeBytes']) if b else 'outside round three'} | {sum(x['inputs']>0 for x in w['instructions'])}/{len(w['instructions'])} | {row['proof']['comparisons']['inputs']} | {tests['passed']}/{tests['ran']} |")
    lines += ["", "Program addresses come from the current source/IDL and `weighing.program.address`; production source builds are not evidence that their ELF hashes equal currently deployed mainnet programs. Zero validator test counts mean no verified matching suite result, not a passing suite.", "", "## Remaining program gaps", "", "Every remaining failed program appears below, ordered by its largest CU increase. Code families describe source-selected paths; static sections and symbols do not establish an exact causal CU split.", "", "| Program | Maximum CU increase | Size increase | Cause and evidence |", "|---|---:|---:|---|"]
    for row in failures:
        ident=row["id"];cause=annotations.get(ident,{}).get("cause") or "Evidence incomplete: "+"; ".join(row["reasons"] or ["matching cause artifact missing"])
        lines.append(f"| {ident} | {number(max([x['increase'] for x in row['regressions']] or [0]))} | {number(row['candidateBytes']-row['stockBytes']) if row.get('candidateBytes') is not None else 'not measured'} | {cell(cause)} |")
    if not failures: lines.append("| none recorded | 0 | 0 | Gate completeness above remains authoritative. |")
    lines += ["", "### Size increases, largest first", "", "| Program | Stock bytes | Current bytes | Increase |", "|---|---:|---:|---:|"]
    for row in size_failures:lines.append(f"| {row['id']} | {number(row['stockBytes'])} | {number(row['candidateBytes'])} | {number(row['candidateBytes']-row['stockBytes'])} |")
    lines += ["", "### Current-stock instruction failures, largest first", "", "| Program | Instruction | Stock CU | Current CU | Increase | Samples | Source family |", "|---|---|---:|---:|---:|---:|---|"]
    instruction_gaps=sorted([(row['id'],x) for row in failures for x in row['regressions']],key=lambda p:(-p[1]['increase'],p[0],p[1]['instruction']))
    for ident,x in instruction_gaps:
        family=annotations.get(ident,{}).get('instructionFamilies',{}).get(x['instruction'],{}).get('families',[])
        lines.append(f"| {ident} | {x['instruction']} | {number(x['stock'])} | {number(x['candidate'])} | {number(x['increase'])} | {x['inputs']} | {cell('; '.join(family) or 'cause evidence incomplete')} |")
    lines += ["", "## Current binaries larger than stock", "", "| Program | Stock bytes | Tare bytes | Increase |", "|---|---:|---:|---:|"]
    for x in current_size_gaps: lines.append(f"| {x['id']} | {number(x['stock'])} | {number(x['tare'])} | {number(x['increase'])} |")
    lines += ["", "## Current-corpus medians above stock", "", "These use the current generated/recorded corpus, not the frozen round-three gate inputs.", "", "| Program | Instruction | Gross CU | Net CU | Increase | Samples |", "|---|---|---:|---:|---:|---:|"]
    for x in above_stock:lines.append(f"| {x['id']} | {x['instruction']} | {number(x['gross'])} | {number(x['net'])} | {number(x['increase'])} | {x['inputs']} |")
    lines += ["", "## Instructions without a successful input", "", f"{len(unmeasured['instructions'])} current instruction rows remain unmeasured. " + ", ".join(f"{k}: {v}" for k,v in sorted(unmeasured['categoryCounts'].items())) + ".", "", "| Program | Instruction | Category | Source reason |", "|---|---|---|---|"]
    for x in unmeasured['instructions']:lines.append(f"| {x['program']} | {x['instruction']} | {x['category']} | {cell(x['reason'])} |")
    lines += ["", "## Not built", "", "| Program | Anchor | Construct or actual failure |", "|---|---|---|"]
    for row in rows:
        if row['status']=='not_built':lines.append(f"| {row['id']} | {row['anchorVersion']} | {cell(row.get('construct') or row['reason'])} |")
    lines += ["", "## Evidence and remaining checks", "", f"[Exact report facts]({link(output.with_suffix('.json'),output.parent)}), [gate]({link(args.gate.resolve(),output.parent)}), [per-program cause evidence]({link(args.causes.resolve(),output.parent)}) and [unmeasured source reasons]({link(unmeasured_path,output.parent)}) retain the measured facts. No divergent input is filtered. Historical round-three differences are optional information, never the acceptance threshold.", "", "The Windows run does not execute Docker or a Linux validator. The lead's fresh-volume, registry and CLI commands are in [lead-run.md]("+link(BENCH/'lead-run.md',output.parent)+"). Native loader checks and synthetic RPC tests do not substitute for the real validator suite.", ""]
    if issues:lines += ["Report validation remains incomplete: " + "; ".join(issues), ""]
    output.write_text("\n".join(lines),encoding="utf-8",newline="\n")
    print(json.dumps({"output":str(output),"programs":len(rows),"measured":len(measured),"gate":gate_status,"issues":len(issues),"remainingPrograms":len(failures)}))
    return 0 if not issues and gate_status == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
