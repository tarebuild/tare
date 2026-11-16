"""Run an isolated candidate subset/canary batch without touching canonical artifacts."""
from __future__ import annotations
import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import threading
import time

ROOT = Path(__file__).resolve().parent.parent
BENCH = ROOT / "bench"
HISTORY = BENCH / "history/round-six"
LOCK = threading.Lock()


def read(path): return json.loads(Path(path).read_text(encoding="utf-8"))
def digest(path): return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def save(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2)+"\n", encoding="utf-8", newline="\n")
def run(command, **kwargs):
    return subprocess.run([str(x) for x in command], check=True, **kwargs)
def rel(path): return Path(path).resolve().relative_to(ROOT).as_posix()


def snapshot(source, destination):
    def ignore(folder, names):
        return [n for n in names if n in {"target", ".git", "node_modules", "__pycache__"} or n.endswith((".so", ".pdb", ".log"))]
    shutil.copytree(source, destination, ignore=ignore)


def toolchain_snapshot(source, destination):
    destination.mkdir()
    for name in ["Cargo.toml", "Cargo.lock"]:
        shutil.copy2(source/name, destination/name)
    for name in ["crates", "versions"]:
        if (source/name).exists():snapshot(source/name, destination/name)


def source_hashes(home):
    result={}
    for base in [home/"Cargo.toml",home/"Cargo.lock"]:
        if base.exists():result[base.relative_to(home).as_posix()]=digest(base)
    for folder in [home/"crates",home/"versions"]:
        if folder.exists():
            for p in folder.rglob("*"):
                if p.is_file() and "target" not in p.parts and p.suffix in [".rs",".toml"]:result[p.relative_to(home).as_posix()]=digest(p)
    return result


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--home",type=Path,required=True,help="Candidate toolchain source home to snapshot")
    parser.add_argument("--cli",type=Path,required=True)
    parser.add_argument("--cli-sha256",required=True)
    parser.add_argument("--run",required=True,help="New folder name beneath bench/history/round-six")
    parser.add_argument("--set",choices=["canary","failures","production","full"],default="canary")
    parser.add_argument("--ids",type=Path,help="Explicit JSON list, replacing the selected set")
    parser.add_argument("--jobs",type=int,default=2,choices=[1,2,3,4])
    parser.add_argument("--prepare-only",action="store_true")
    parser.add_argument("--resume",action="store_true",help="Run an already prepared immutable snapshot")
    parser.add_argument("--seed-stock-cache",action="store_true",help="Copy QUIESCENT stock/IDL caches only; unsafe while another build is writing")
    parser.add_argument("--seed-cache-home",type=Path,help="Quiescent source home for copied stock/IDL caches")
    parser.add_argument("--seed-version-cache",action="append",default=[],metavar="VERSION=PATH",help="Override a version's quiescent cache root (contains stock/idl or repro-v1)")
    parser.add_argument("--seed-phases",default="stock",help="Comma-separated cache phases to copy; default stock, optional idl")
    parser.add_argument("--cache-quiescent",action="store_true",help="Confirm seed-cache owners are stopped; copying live caches is rejected")
    args=parser.parse_args()
    seed_phases=args.seed_phases.split(',')
    if not seed_phases or any(p not in ['stock','idl'] for p in seed_phases):raise ValueError("Seed phases must be stock and/or idl")
    seed_overrides={}
    for option in args.seed_version_cache:
        version,separator,value=option.partition("=")
        if not separator:raise ValueError("--seed-version-cache requires VERSION=PATH")
        seed_overrides[version]=Path(value).resolve()
    if (args.seed_stock_cache or args.seed_cache_home or seed_overrides) and not args.cache_quiescent:
        raise ValueError("Copy only stopped cache owners: pass --cache-quiescent after coordinating them")
    if Path(args.run).name!=args.run:raise ValueError("--run must be a single new folder name")
    home=args.home.resolve();cli=args.cli.resolve();out=HISTORY/args.run
    if out.exists() and not args.resume:raise ValueError("Run folder exists; use a new run name or --resume")
    if args.resume and not (out/"provenance.json").exists():raise ValueError("Resume requires a prepared run")
    if digest(cli)!=args.cli_sha256.lower():raise ValueError("CLI SHA256 does not match")
    if not (home/"crates/tare-lang/src/lib.rs").exists():raise ValueError("Candidate home has no language implementation")
    manifest=read(BENCH/"manifest.json");known={p['id']:p for p in manifest['programs']}
    base=read(HISTORY/"baseline.json")
    ids=read(args.ids) if args.ids else base['canaryIds'] if args.set=='canary' else [r['id'] for r in base['sizeFailures']] if args.set=='failures' else list(known) if args.set=='full' else ['production-squads-v4','production-whirlpools','production-marinade']
    if not ids or len(ids)!=len(set(ids)) or set(ids)-set(known):raise ValueError("IDs must be a nonempty unique known set")
    selected=[known[i] for i in ids]
    lanes={}
    for p in selected:lanes.setdefault(p['anchorVersion'],[]).append(p)
    out.mkdir(parents=True,exist_ok=True)
    before=source_hashes(home)
    if args.resume:
        provenance=read(out/'provenance.json')
        if not provenance.get('prepared'):raise ValueError("The previous preparation did not complete; use a new run name. Incomplete snapshots cannot resume.")
        if provenance['ids']!=ids or provenance['cliSha256']!=digest(cli):raise ValueError("Resume IDs/CLI differ from prepared snapshot")
        before=provenance['candidateSourceSha256']
        for version in lanes:
            if source_hashes(out/version/'toolchain')!=before:raise ValueError("Prepared candidate source changed")
    else:
        provenance={'cli':rel(cli),'cliSha256':digest(cli),'candidateHome':rel(home),'candidateSourceSha256':before,'baselineSha256':base['baselineSha256'],'ids':ids,'jobs':args.jobs,'prepared':False,'results':{},'runnerSha256':digest(Path(__file__)), 'benchInputs':{name:digest(BENCH/name) for name in ['run.mjs','validate.py','gate.mjs','manifest.json','production/Cargo.toml','production/Cargo.lock']}}
        save(out/'provenance.json',provenance)
        # Each lane owns its source adapter directories, SBF cache and full workspace clone.
        # Repository clones are local --shared reads of immutable Git object storage;
        # source files and Cargo.lock are independent working files, never hardlinks.
        for version,programs in lanes.items():
            lane=out/version;lane.mkdir();lane_home=lane/'toolchain';toolchain_snapshot(home,lane_home)
            lane_cli=lane/('tare.exe' if os.name=='nt' else 'tare')
            shutil.copy2(cli,lane_cli)
            bench=lane/'bench';bench.mkdir();(lane/'contract').mkdir();shutil.copytree(ROOT/'contract/schema',lane/'contract/schema')
            for name in ['run.mjs','validate.py','terminal.mjs','gate.mjs','manifest.json']:
                shutil.copy2(BENCH/name,bench/name)
            snapshot(BENCH/'production',bench/'production')
            # The copied path dependency follows this lane's immutable home.
            check=subprocess.run(['cargo','metadata','--offline','--locked','--format-version','1','--manifest-path',str(bench/'production/Cargo.toml')],capture_output=True,text=True,encoding='utf-8',errors='replace')
            if check.returncode and any(p.get('productionCorpus') for p in programs):raise ValueError('Production helper lock is incompatible with the candidate version. Refresh bench/production/Cargo.lock before preparing a run: '+check.stderr)
            save(bench/'selected-manifest.json',{'programs':programs})
            for program in programs:
                if program.get('suppliedIdl'):
                    target=bench/program['suppliedIdl'];target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(BENCH/program['suppliedIdl'],target)
            if any(p.get('local') for p in programs):snapshot(ROOT/'registry',lane/'registry')
            preparation_issues=[]
            copied=set()
            for program in programs:
                if program.get('local'):continue
                checkout=program.get('checkout',manifest['checkout'])
                if checkout in copied:continue
                copied.add(checkout);src=BENCH/checkout;dest=bench/checkout;dest.parent.mkdir(parents=True,exist_ok=True)
                run(['git','clone','--shared','--no-checkout','--no-hardlinks',src,dest],stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
                run(['git','-C',dest,'-c','core.autocrlf=false','checkout','--detach',program.get('revision',manifest['revision'])],stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
                # Initialize only submodules under selected workspace roots; unrelated
                # historical CPI examples must not trigger network work for basic tests.
                if (src/'.gitmodules').exists():
                    listed=subprocess.run(['git','-C',str(src),'config','--file','.gitmodules','--get-regexp','path'],capture_output=True,text=True,encoding='utf-8',errors='replace')
                    if listed.returncode not in [0,1]:raise ValueError('Cannot read submodule paths: '+listed.stderr)
                    workspaces=[p['workspace'].replace('\\','/').rstrip('/') for p in programs if p.get('checkout',manifest['checkout'])==checkout]
                    needed=[line.split(None,1)[1] for line in listed.stdout.splitlines() if any(w=='.' or line.split(None,1)[1].startswith(w+'/') for w in workspaces)]
                    if needed:
                        sub=subprocess.run(['git','-C',str(dest),'submodule','update','--init','--recursive','--',*needed],capture_output=True,text=True,encoding='utf-8',errors='replace')
                        if sub.returncode:preparation_issues.append({'checkout':checkout,'submodules':needed,'exitCode':sub.returncode,'stderr':sub.stderr})
            cache_seed=None
            if args.seed_stock_cache or args.seed_cache_home or seed_overrides:
                source_cache=seed_overrides.get(version,(args.seed_cache_home or home).resolve()/'target/sbf-cache'/version/'repro-v1')
                if (source_cache/'repro-v1').exists():source_cache=source_cache/'repro-v1'
                cache_seed={'source':rel(source_cache),'phases':[],'quiescent':True}
                for phase in seed_phases:
                    src=source_cache/phase
                    if src.exists():
                        before_files={str(p.relative_to(src)):(p.stat().st_size,p.stat().st_mtime_ns) for p in src.rglob('*') if p.is_file()}
                        try:shutil.copytree(src,lane_home/'target/sbf-cache'/version/'repro-v1'/phase)
                        except (OSError,shutil.Error) as error:
                            raise ValueError("Stock-cache seed changed during copy; do not copy an active Cargo cache. Use a new run without cache seed, or stop the cache owner before copying.") from error
                        after_files={str(p.relative_to(src)):(p.stat().st_size,p.stat().st_mtime_ns) for p in src.rglob('*') if p.is_file()}
                        if after_files!=before_files:raise ValueError("Seed cache changed during copy; discard incomplete run and use a quiescent owner")
                        cache_seed['phases'].append({'name':phase,'files':len(before_files),'bytes':sum(v[0] for v in before_files.values())})
            provenance['results'][version]={'directory':rel(lane),'ids':[p['id'] for p in programs],'status':'prepared','preparationIssues':preparation_issues,'cacheSeed':cache_seed}
            save(out/'provenance.json',provenance)
    after=source_hashes(home)
    if not args.resume and after!=before:raise ValueError("Candidate sources changed while snapshots were copied")
    provenance['prepared']=True;save(out/'provenance.json',provenance)
    if args.prepare_only:
        print(f"Prepared {len(ids)} isolated programs in {len(lanes)} lanes. No builds started.");return 0
    env=os.environ.copy()
    for key in ['RUSTFLAGS','TARE_BENCH_MEASURE_ONLY','TARE_BENCH_REFRESH','TARE_BENCH_STOP_FILE','TARE_HOME','PERL5LIB']:env.pop(key,None)
    env.update(CARGO_NET_OFFLINE='true',CARGO_TERM_COLOR='never',NO_COLOR='1',NODE_DISABLE_COLORS='1')
    if os.name=='nt':env['OPENSSL_SRC_PERL']=str(ROOT/'toolchain/host/strawberry-perl/perl/bin/perl.exe')
    def lane_run(version,programs):
        lane=out/version;lane_home=lane/'toolchain';lane_cli=lane/('tare.exe' if os.name=='nt' else 'tare');bench=lane/'bench'
        local={**env,'TARE_HOME':str(lane_home),'TARE_BENCH_RESULTS':str(bench/'results.json')}
        log_path=lane/'run.txt';started=time.time()
        with log_path.open('w',encoding='utf-8',newline='\n') as log:
            log.write('$ tare --version\n');log.flush();run([lane_cli,'--version'],stdout=log,stderr=subprocess.STDOUT,env=local)
            prepare=subprocess.run(['node',str(bench/'production/prepare.mjs'),*[p['id'] for p in programs]],cwd=lane,env=local,stdout=log,stderr=subprocess.STDOUT)
            if prepare.returncode:code=prepare.returncode
            else:code=subprocess.run(['node',str(bench/'run.mjs'),'--tare',str(lane_cli),*[p['id'] for p in programs]],cwd=lane,env=local,stdout=log,stderr=subprocess.STDOUT).returncode
        gate=subprocess.run(['node',str(bench/'gate.mjs'),'--summary','--results',str(bench/'results.json'),'--manifest',str(bench/'selected-manifest.json'),'--output',str(lane/'gate.json')],cwd=lane,capture_output=True,text=True,encoding='utf-8',errors='replace') if (bench/'results.json').exists() else None
        if gate:(lane/'gate.txt').write_text(gate.stdout+gate.stderr,encoding='utf-8',newline='\n')
        frozen=source_hashes(lane_home)
        result={'directory':rel(lane),'ids':[p['id'] for p in programs],'status':'complete','preparationIssues':provenance['results'][version].get('preparationIssues',[]),'cacheSeed':provenance['results'][version].get('cacheSeed'),'exitCode':code,'gateExit':gate.returncode if gate else None,'seconds':round(time.time()-started,3),'candidateSourceUnchanged':frozen==before,'log':rel(log_path),'logSha256':digest(log_path)}
        if (bench/'results.json').exists():result['resultSha256']=digest(bench/'results.json')
        result['checkoutChanges']=[]
        for checkout in sorted({p.get('checkout',manifest['checkout']) for p in programs if not p.get('local')}):
            status=subprocess.run(['git','-C',str(bench/checkout),'diff','HEAD','--name-only','--','*.rs','*Cargo.toml','*Cargo.lock'],capture_output=True,text=True,encoding='utf-8',errors='replace',check=True)
            if status.stdout.strip():result['checkoutChanges'].append({'checkout':checkout,'files':status.stdout.splitlines()})
        with LOCK:provenance['results'][version]=result;save(out/'provenance.json',provenance)
        return code or (gate.returncode if gate else 1) or (0 if frozen==before and not result['checkoutChanges'] else 1)
    exits=[]
    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        for future in as_completed([pool.submit(lane_run,v,p) for v,p in lanes.items()]):exits.append(future.result())
    # Reports keep lane-local paths; no copying into canonical artifact locations.
    all_rows=[]
    for version in lanes:
        file=out/version/'bench/results.json'
        if file.exists():all_rows+=read(file)['programs']
    save(out/'summary.json',{'programs':len(all_rows),'requested':len(ids),'measured':sum(r['status']=='measured' for r in all_rows),'proofsMatched':sum(r['status']=='measured' and r['proof']['status']=='matched' for r in all_rows),'notBuilt':[{ 'id':r['id'],'reason':r['reason']} for r in all_rows if r['status']!='measured'],'laneExitCodes':exits,'canonicalArtifactsUntouched':True})
    print(f"Completed isolated {len(all_rows)}/{len(ids)} programs; details: {rel(out/'summary.json')}")
    return int(any(exits) or len(all_rows)!=len(ids))


if __name__=='__main__':
    try:sys.exit(main())
    except (ValueError,OSError,subprocess.CalledProcessError) as error:
        print(str(error),file=sys.stderr);
        if isinstance(error,subprocess.CalledProcessError) and error.stderr:print(error.stderr.decode(errors="replace") if isinstance(error.stderr,bytes) else error.stderr,file=sys.stderr)
        sys.exit(2)
