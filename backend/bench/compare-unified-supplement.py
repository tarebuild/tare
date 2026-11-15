from pathlib import Path
import json, subprocess, shutil, time
root=Path(__file__).resolve().parent.parent
base=root/"bench/history/round-four"
results=base/"final-0.4.0-supplement.json"
provenance=base/"final-0.4.0-supplement-provenance.json"
archive=base/"final-0.4.0-supplement-baseline"
archive.mkdir(exist_ok=True)
old={p["id"] for p in json.loads((root/"bench/gaps/round-three/opt/fixed-corpus/index.json").read_text())["programs"]}
seen={}
while True:
 done=provenance.exists() and "exitCode" in json.loads(provenance.read_text())
 rows=json.loads(results.read_text())["programs"] if results.exists() else []
 ids=[p["id"] for p in rows if p["id"] in old and p["id"] not in seen]
 if ids:
  with (archive/"run.txt").open("a",encoding="utf-8",newline="\n") as log:
   subprocess.run(["node",str(root/"bench/compare-baseline.mjs"),"--results",str(results),*ids],cwd=root,stdout=log,stderr=subprocess.STDOUT)
  source=root/"bench/gaps/round-three/performance"
  for id in ids:
   seen[id]=json.loads((source/f"{id}.comparison.json").read_text())["baselineComparison"]
   for suffix in [".json",".comparison.json",".log"]:
    p=source/(id+suffix)
    if p.exists():shutil.copy2(p,archive/p.name)
  (archive/"summary.json").write_text(json.dumps({"programs":list(seen.values())},indent=2)+"\n",encoding="utf-8",newline="\n")
 if done:break
 time.sleep(15)
