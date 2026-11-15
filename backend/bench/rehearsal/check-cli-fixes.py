"""Exercise recorded/mainnet CLI routing against local RPC with real SBF execution."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
parser = argparse.ArgumentParser()
parser.add_argument("--tare", type=Path, required=True)
parser.add_argument("--label", required=True)
parser.add_argument("--expect-fixed", action="store_true")
args = parser.parse_args()
folder = ROOT / "bench/history/round-four/cli-replay" / args.label
folder.mkdir(parents=True, exist_ok=True)
workspace = ROOT / "bench/repro/fresh-a/tests/idl"
manifest = workspace / "programs/new-idl/Cargo.toml"
replay = workspace / "target/tare/replay.json"
bundle = json.loads(replay.read_text())
bundle["mainnet"] = None
bundle["tests"] = None
replay_input = folder / "input-replay.json"
replay_input.write_text(json.dumps(bundle) + "\n", encoding="utf-8")
request_file = folder / "rpc-requests.json"
server_log = (folder / "rpc.txt").open("wb")
server = subprocess.Popen([sys.executable, str(ROOT / "bench/rehearsal/mainnet-rpc.py"), "--requests", str(request_file)], cwd=ROOT, stdout=server_log, stderr=subprocess.STDOUT)
env = os.environ.copy()
env.pop("TARE_REHEARSAL_DEFECT", None)
command = [str(args.tare.resolve()), "prove", "--host", "--manifest-path", str(manifest), "--replay", str(replay_input), "--program", "Newid11111111111111111111111111111111111111", "--samples", "1", "--url", "http://127.0.0.1:7814"]
results = []
try:
    time.sleep(.5)
    if server.poll() is not None:
        raise RuntimeError("local fixture did not start")
    def run(name, cmd):
        result = subprocess.run(cmd, cwd=ROOT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        text = result.stdout.decode("utf-8", errors="replace").replace(str(ROOT), ".").replace(str(ROOT).replace("\\", "/"), ".")
        (folder / (name + ".txt")).write_text(text, encoding="utf-8", newline="\n")
        row = {"case":name,"exitCode":result.returncode}
        results.append(row)
        return row
    row = run("sample-with-replay", command)
    proof = json.loads((workspace / "target/tare/new_idl.proof.json").read_text())
    row.update(mainnet=proof["proof"]["corpus"]["counts"]["mainnet"], generated=proof["proof"]["corpus"]["counts"]["generated"], matched=proof["proof"]["status"])
    assert row["exitCode"] == 0 and row["matched"] == "matched"
    assert row["mainnet"] == int(args.expect_fixed), row
    if args.expect_fixed:
        sampled = json.loads((workspace / "target/tare/mainnet-corpus.json").read_text())
        assert json.loads(replay.read_text())["mainnet"] == sampled
        ordinary = command[:command.index("--program")]
        for name, change in [("wrong-kind", {"kind":"tests"}), ("wrong-target", {"targetProgram":"11111111111111111111111111111111"})]:
            invalid = {**sampled, **change}
            path = folder / (name + ".json")
            path.write_text(json.dumps(invalid) + "\n", encoding="utf-8")
            row = run(name, ordinary + ["--mainnet-corpus", str(path)])
            assert row["exitCode"] == 1
        before = len(json.loads(request_file.read_text()))
        invalid_program = command.copy()
        invalid_program[invalid_program.index("--program") + 1] = "11111111111111111111111111111111"
        row = run("wrong-program-before-rpc", invalid_program)
        assert row["exitCode"] == 1
        assert len(json.loads(request_file.read_text())) == before
finally:
    server.terminate()
    server.wait(timeout=10)
    server_log.close()
(folder / "summary.json").write_text(json.dumps({"serverStopped":server.poll() is not None,"fixtureOnly":True,"cases":results}, indent=2) + "\n", encoding="utf-8")
print(json.dumps(results))
