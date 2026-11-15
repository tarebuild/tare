"""Append factual failed-check diagnostics to Compat without altering measured rows."""
from pathlib import Path
import json
root=Path(__file__).resolve().parent
results=json.loads((root/"results.json").read_text())
compat=json.loads((root/"compat.json").read_text())
manifest={p["id"]:p for p in json.loads((root/"manifest.json").read_text())["programs"]}
groups={}
evidence=[]
for row in results["programs"]:
    if row["status"]!="not_built" or row.get("construct"):
        continue
    id=row["id"]
    if "tictactoe" in id:
        name="legacy #[error] conversion to anchor_lang::Error"
        phase="Tare compilation"
        location="programs/tictactoe/src/lib.rs:199 (attribute), :96 (conversion)"
    elif "multisig" in id:
        name="generated multisig change_threshold signer resolution"
        phase="generated corpus construction"
        location="input change_threshold:short-instruction"
    elif "lockup" in id:
        name="legacy ProgramState / #[state] / #[interface] API"
        phase="Tare compilation"
        location="programs/lockup/src/lib.rs:4, :18, :539"
    elif "fail-type" in id:
        name="stock instruction argument type validation"
        phase="stock compilation"
        location="programs/test-instruction-validation/src/lib.rs:28"
    elif id.endswith("tests-idl-programs-idl"):
        name="solana_program::feature::Feature facade"
        phase="Tare compilation"
        location="programs/idl/src/lib.rs:490, :501, :509"
    else:
        raise RuntimeError(f"Unclassified failure: {id}")
    record={"id":id,"anchorVersion":row["anchorVersion"],"construct":name,"phase":phase,"location":location,"transcript":"transcripts/"+id+".txt","reason":row["reason"]}
    evidence.append(record);groups.setdefault(name,[]).append(record)
existing={entry["construct"] for entry in compat["constructs"]}
for name, failures in groups.items():
    if name in existing:
        raise RuntimeError(f"Refusing to replace existing construct evidence: {name}")
    versions=[]
    for version in compat["anchorVersions"]:
        cases=[x for x in failures if x["anchorVersion"]==version]
        note="\n".join(f"{x['id']}: {x['phase']}, {x['location']}; {x['reason']} [transcript: {x['transcript']}]" for x in cases) if cases else "No attempted failure of this check in this cohort; this entry does not assert language support."
        versions.append({"anchorVersion":version,"supported":False,"programs":[],"note":note})
    compat["constructs"].append({"construct":name,"versions":versions})
compat["constructs"].sort(key=lambda x:x["construct"])
(root/"compat.json").write_text(json.dumps(compat,indent=2)+"\n",encoding="utf-8",newline="\n")
out=root/"history/round-four/final-0.4.0-refusal-annotations.json"
out.write_text(json.dumps({"description":"Postprocessed only Compat; source result rows and original refusal reasons remain unchanged.","programs":evidence},indent=2)+"\n",encoding="utf-8",newline="\n")
print(f"Annotated {len(evidence)} failures in {len(groups)} Compat check entries")
