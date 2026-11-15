"""Run the ordinary live full benchmark without replacing canonical results."""
from pathlib import Path
import hashlib
import json
import os
import subprocess
import sys

root = Path(__file__).resolve().parent.parent
exe = Path(sys.argv[1]).resolve()
output = root / "bench/history/round-four"
output.mkdir(parents=True, exist_ok=True)
results = output / "full-stable.json"
if results.exists():
    raise SystemExit(f"Refusing to overwrite existing run: {results}")
env = os.environ.copy()
for key in ["TARE_HOME", "TARE_REHEARSAL_DEFECT", "TARE_BENCH_MEASURE_ONLY", "TARE_BENCH_REFRESH"]:
    env.pop(key, None)
env["CARGO_NET_OFFLINE"] = "true"
env["CARGO_TERM_COLOR"] = "never"
env["NO_COLOR"] = "1"
env["NODE_DISABLE_COLORS"] = "1"
env["TARE_BENCH_RESULTS"] = str(results)
if os.name == "nt":
    env["OPENSSL_SRC_PERL"] = str(root / "toolchain/host/strawberry-perl/perl/bin/perl.exe")
    env.pop("PERL5LIB", None)
source_hashes = {}
for crate in ["tare-lang", "tare-lang-macros", "tare-spl", "tare-opt"]:
    for source in (root / "toolchain/crates" / crate).rglob("*.rs"):
        if "target" not in source.parts:
            source_hashes[source.relative_to(root).as_posix()] = hashlib.sha256(source.read_bytes()).hexdigest()
provenance = {"cli": str(exe), "cliSha256": hashlib.sha256(exe.read_bytes()).hexdigest(), "sourceSha256": source_hashes}
(output / "full-stable-provenance.json").write_text(json.dumps(provenance, indent=2), encoding="utf-8")
manifest = json.loads((root / "bench/manifest.json").read_text())
selected = [program["id"] for program in manifest["programs"] if program["kind"] != "production"]
provenance["programs"] = selected
command = ["node", str(root / "bench/run.mjs"), "--tare", str(exe), *selected]
with (output / "full-stable.txt").open("w", encoding="utf-8", newline="\n") as log:
    result = subprocess.run(command, cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT)
    log.write(f"\n[full runner exit {result.returncode}]\n")
provenance["exitCode"] = result.returncode
provenance["sourceChangesDuringRun"] = [name for name, digest in source_hashes.items() if hashlib.sha256((root/name).read_bytes()).hexdigest() != digest]
(output / "full-stable-provenance.json").write_text(json.dumps(provenance, indent=2), encoding="utf-8")
print(f"Full benchmark exit {result.returncode}")
sys.exit(result.returncode)
