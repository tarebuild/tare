from pathlib import Path
import hashlib,json,os,subprocess,sys
root=Path(__file__).resolve().parent.parent
version=sys.argv[1]
exe=Path(sys.argv[2]).resolve()
manifest=json.loads((root/"bench/manifest.json").read_text())
ids=[p["id"] for p in manifest["programs"] if p["kind"]!="production" and p["anchorVersion"]==version]
if not ids:raise SystemExit("empty partition")
out=root/"bench/history/round-four"
file=out/f"full-stable-{version}.json"
if file.exists():raise SystemExit(f"existing partition {file}")
env=os.environ.copy()
for key in ["TARE_HOME","TARE_REHEARSAL_DEFECT","TARE_BENCH_MEASURE_ONLY","TARE_BENCH_REFRESH"]:env.pop(key,None)
env.update(CARGO_NET_OFFLINE="true",CARGO_TERM_COLOR="never",NO_COLOR="1",NODE_DISABLE_COLORS="1",TARE_BENCH_RESULTS=str(file))
if os.name=="nt":
 env["OPENSSL_SRC_PERL"]=str(root/"toolchain/host/strawberry-perl/perl/bin/perl.exe");env.pop("PERL5LIB",None)
with (out/f"full-stable-{version}.txt").open("w",encoding="utf-8",newline="\n") as log:
 r=subprocess.run(["node",str(root/"bench/run.mjs"),"--tare",str(exe),*ids],cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT)
 log.write(f"\n[partition exit {r.returncode}]\n")
(out/f"full-stable-{version}-exit.json").write_text(json.dumps({"exitCode":r.returncode,"cliSha256":hashlib.sha256(exe.read_bytes()).hexdigest(),"programs":ids},indent=2),encoding="utf-8")
sys.exit(r.returncode)
