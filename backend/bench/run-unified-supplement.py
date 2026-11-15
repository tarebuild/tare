"""Run one immutable CLI cohort without replacing the canonical benchmark feed."""
from pathlib import Path
import hashlib
import json
import os
import subprocess
import sys

root = Path(__file__).resolve().parent.parent
binary, expected_sha = sys.argv[1:]
version = "supplement"
exe = Path(binary).resolve()
digest = lambda file: hashlib.sha256(file.read_bytes()).hexdigest()
if digest(exe) != expected_sha.lower():
    raise SystemExit("CLI SHA256 differs from the frozen release")
manifest = json.loads((root / "bench/manifest.json").read_text())
ids = json.loads((root / "bench/history/round-four/final-0.4.0-supplement-ids.json").read_text())
if not ids:
    raise SystemExit("Empty ordinary-program cohort")
out = root / "bench/history/round-four"
out.mkdir(parents=True, exist_ok=True)
prefix = out / "final-0.4.0-supplement"
results = Path(str(prefix) + ".json")
if results.exists():
    raise SystemExit(f"Refusing to replace existing cohort: {results}")
source_paths = set()
for crate in ["tare-lang", "tare-lang-macros", "tare-spl", "tare-opt", "tare-prove", "tare"]:
    for file in (root / "toolchain/crates" / crate).rglob("*"):
        if file.is_file() and "target" not in file.parts and file.suffix in [".rs", ".toml"]:
            source_paths.add(file)
source_paths.update([root / "toolchain/Cargo.toml", root / "toolchain/Cargo.lock", root / "bench/manifest.json", root / "bench/run.mjs"])
source_hashes = {p.relative_to(root).as_posix(): digest(p) for p in sorted(source_paths)}
env = os.environ.copy()
for key in ["TARE_HOME", "TARE_REHEARSAL_DEFECT", "TARE_BENCH_MEASURE_ONLY", "TARE_BENCH_REFRESH", "TARE_BENCH_STOP_FILE", "TARE_REGISTRY_SOURCE"]:
    env.pop(key, None)
env.update(CARGO_NET_OFFLINE="true", CARGO_TERM_COLOR="never", NO_COLOR="1", NODE_DISABLE_COLORS="1", TARE_BENCH_RESULTS=str(results))
if os.name == "nt":
    env["OPENSSL_SRC_PERL"] = str(root / "toolchain/host/strawberry-perl/perl/bin/perl.exe")
    env.pop("PERL5LIB", None)
command = ["node", str(root / "bench/run.mjs"), "--tare", str(exe), *ids]
provenance = {"cli": str(exe), "cliSha256": expected_sha.lower(), "anchorVersion": version, "programs": ids, "sourceSha256": source_hashes, "command": command}
provenance_file = Path(str(prefix) + "-provenance.json")
def save():
    provenance_file.write_text(json.dumps(provenance, indent=2) + "\n", encoding="utf-8", newline="\n")
save()
with Path(str(prefix) + ".txt").open("w", encoding="utf-8", newline="\n") as log:
    log.write("$ node bench/run.mjs --tare " + str(exe) + " [" + str(len(ids)) + " " + version + " programs]\n")
    log.flush()
    process = subprocess.Popen(command, cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT)
    provenance["driverPid"], provenance["runnerPid"] = os.getpid(), process.pid
    save()
    result = process.wait()
    log.write(f"\n[cohort exit {result}]\n")
provenance["exitCode"] = result
provenance["sourceChangesDuringRun"] = [name for name, sha in source_hashes.items() if digest(root / name) != sha]
provenance["cliChangedDuringRun"] = digest(exe) != expected_sha.lower()
save()
raise SystemExit(result)
