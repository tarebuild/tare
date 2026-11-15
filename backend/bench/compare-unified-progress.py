"""Replay completed immutable-CLI rows against the exact old corpus, incrementally."""
from pathlib import Path
from collections import Counter
import hashlib
import json
import shutil
import subprocess
import time

root = Path(__file__).resolve().parent.parent
base = root / "bench/history/round-four"
archive = base / "final-0.4.0-baseline"
archive.mkdir(parents=True, exist_ok=True)
index = json.loads((root / "bench/gaps/round-three/opt/fixed-corpus/index.json").read_text())
old = {p["id"]: p for p in index["programs"]}
versions = ["0.31.1", "0.32.3", "1.2.2"]
records = {}
helper = root / "toolchain/target/release/examples/fixed_corpus.exe"
helper_hash = hashlib.sha256(helper.read_bytes()).hexdigest()
while True:
    finished = True
    for version in versions:
        file = base / f"final-0.4.0-{version}.json"
        provenance = base / f"final-0.4.0-{version}-provenance.json"
        state = json.loads(provenance.read_text()) if provenance.exists() else {}
        done = "exitCode" in state
        finished &= done
        if not file.exists():
            continue
        rows = json.loads(file.read_text())["programs"]
        pending = [p["id"] for p in rows if p["id"] in old and p["id"] not in records]
        if len(pending) < 6 and not done:
            continue
        if not pending:
            continue
        with (archive / f"{version}.txt").open("a", encoding="utf-8", newline="\n") as log:
            result = subprocess.run(["node", str(root / "bench/compare-baseline.mjs"), "--results", str(file), "--cohort", version, "--helper", str(helper), *pending], cwd=root, stdout=log, stderr=subprocess.STDOUT)
            log.write(f"\n[batch exit {result.returncode}]\n")
        source = root / "bench/gaps/round-three/performance"
        for id in pending:
            sidecar = source / f"{id}.comparison.json"
            if not sidecar.exists():
                raise RuntimeError(f"Missing comparison metadata: {id}")
            record = json.loads(sidecar.read_text())["baselineComparison"]
            for suffix in [".json", ".comparison.json", ".log"]:
                path = source / (id + suffix)
                if path.exists():
                    shutil.copy2(path, archive / path.name)
            records[id] = record
            print(id, record["status"], flush=True)
        counts = dict(Counter(r["status"] for r in records.values()))
        summary = {"helperSha256": helper_hash, "completed": len(records), "counts": counts, "programs": list(records.values())}
        (archive / "summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8", newline="\n")
    if finished:
        break
    time.sleep(15)
