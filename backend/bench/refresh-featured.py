"""Refresh selected canonical benchmark rows after a frozen ordinary CLI is ready."""
from pathlib import Path
import argparse
import os
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
PROGRAMS = ["basic-0", "basic-1", "basic-4", "v0_32_3-tests-idl-programs-new-idl"]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--tare", type=Path, required=True)
parser.add_argument("--results", type=Path, default=ROOT / "bench/results.json")
parser.add_argument("--run", action="store_true", help="Run after compiler freeze; otherwise print the prepared command.")
args = parser.parse_args()
command = ["node", str(ROOT / "bench/run.mjs"), "--tare", str(args.tare.resolve()), *PROGRAMS]
print(subprocess.list2cmdline(command), flush=True)
if not args.run:
    print("Prepared only; pass --run after compiler freeze.")
    raise SystemExit(0)
env = os.environ.copy()
for key in ["TARE_HOME", "TARE_REHEARSAL_DEFECT", "TARE_BENCH_MEASURE_ONLY", "TARE_BENCH_REFRESH"]:
    env.pop(key, None)
env["TARE_BENCH_RESULTS"] = str(args.results.resolve())
env["CARGO_NET_OFFLINE"] = "true"
if os.name == "nt":
    perl = ROOT / "toolchain/host/strawberry-perl/perl/bin/perl.exe"
    env["OPENSSL_SRC_PERL"] = str(perl)
    env.pop("PERL5LIB", None)
log = ROOT / "bench/transcripts/featured-refresh.txt"
log.parent.mkdir(parents=True, exist_ok=True)
with log.open("w", encoding="utf-8", newline="\n") as stream:
    result = subprocess.run(command, cwd=ROOT, env=env, stdout=stream, stderr=subprocess.STDOUT)
print(f"Benchmark exit {result.returncode}; transcript {log}")
sys.exit(result.returncode)
