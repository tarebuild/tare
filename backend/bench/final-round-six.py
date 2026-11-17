"""Prepare/run an isolated full round-six matrix, then stage validated publication."""
import argparse
import base64
from concurrent.futures import ThreadPoolExecutor,as_completed
import datetime as dt
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import jsonschema

ROOT=Path(__file__).resolve().parent.parent
BENCH=ROOT/'bench'
HISTORY=BENCH/'history/round-six'


def read(p):return json.loads(p.read_text(encoding='utf-8'))
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def write(p,x):p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(x,indent=2)+'\n',encoding='utf-8',newline='\n')
def schema(name,value):jsonschema.Draft202012Validator(read(ROOT/'contract/schema'/f'{name}.json')).validate(value)
def rel(p):return p.resolve().relative_to(ROOT).as_posix()


def assemble(folder,publish,supplements):
    if not folder.is_dir():raise ValueError('Prepared run directory is missing')
    manifest=read(BENCH/'manifest.json');known={p['id']:p for p in manifest['programs']}
    rows={};origins={};artifacts={};inputs=[]
    previous=read(BENCH/'results.json')
    prior={r['id']:r for r in previous['programs']}
    retained_refusals={k:r for k,r in prior.items() if r['status']=='not_built'}
    sources=[]
    for lane in sorted(p for p in folder.iterdir() if p.is_dir() and (p/'bench/results.json').exists()):
        prov=read(folder/'provenance.json')['results'].get(lane.name)
        if not prov or prov.get('status')!='complete' or not prov.get('candidateSourceUnchanged') or prov.get('checkoutChanges'):raise ValueError(f'Incomplete or changed lane: {lane.name}')
        sources.append((lane/'bench/results.json',lane/'bench/artifacts',lane.name,False))
    for option in supplements:
        file,artifact,provenance=map(Path,option.split('|'))
        prov=read(provenance);main=read(folder/'provenance.json')
        if prov.get('cliSha256')!=main['cliSha256'] or prov.get('sourceSha256')!=main['candidateSourceSha256'] or not prov.get('candidateSourceUnchanged'):
            raise ValueError('Supplement differs from frozen CLI/source: '+str(file))
        sources.append((file,artifact,'supplement:'+rel(file),True))
        inputs.append({'provenance':rel(provenance),'sha256':sha(provenance)})
    for file,artifact_root,lane_name,supplement in sources:
        doc=read(file);schema('BenchResults',doc);inputs.append({'file':rel(file),'sha256':sha(file),'supplement':supplement})
        for row in doc['programs']:
            ident=row['id']
            if ident not in known or (ident in rows and not supplement):raise ValueError(f'Unknown/duplicate {ident}')
            if supplement and row['status']!='measured':raise ValueError('Supplement must contain successful measured proof: '+ident)
            if row['anchorVersion']!=known[ident]['anchorVersion'] or row['source']['commit']!=known[ident].get('revision'):raise ValueError(f'Source mismatch {ident}')
            rows[ident]=row;origins[ident]={'file':rel(file),'sha256':sha(file),'lane':lane_name,'supplement':supplement}
            if row['status']=='measured':
                if row['proof']['status']!='matched':raise ValueError(f'Diverged proof {ident}')
                archive=artifact_root/ident;artifacts[ident]=archive
                for kind in ['stock','tare']:
                    elf=archive/f'{kind}.so';build=row['weighing']['builds'][kind]
                    if sha(elf)!=build['hash'] or elf.stat().st_size!=build['sizeBytes']:raise ValueError(f'Stale {kind} ELF {ident}')
                proof=read(archive/'proof.json');schema('ProofReport',proof)
                if proof['builds']!=row['weighing']['builds'] or proof['proof']['status']!='matched':raise ValueError(f'Stale native proof {ident}')
                replay=proof['replay'];compressed=base64.b64decode(replay['data'],validate=True)
                with gzip.GzipFile(fileobj=io.BytesIO(compressed)) as stream:payload=stream.read(256*1024*1024+1)
                if len(payload)>256*1024*1024 or hashlib.sha256(payload).hexdigest()!=replay['sha256'] or replay['sha256']!=row['proof']['corpus']['hash']:raise ValueError(f'Replay checksum {ident}')
                json.loads(payload)
    for ident,row in retained_refusals.items():
        if ident not in rows:
            rows[ident]=row;origins[ident]={'file':'bench/results.json','sha256':sha(BENCH/'results.json'),'retainedRefusal':True,'priorVersion':previous['toolchainVersion']}
    if set(rows)!=set(known):raise ValueError(f'Matrix incomplete: {len(rows)}/{len(known)}; missing {sorted(set(known)-set(rows))}')
    lost=[ident for ident,row in prior.items() if row['status']=='measured' and rows[ident]['status']!='measured']
    if lost:raise ValueError('Previously measured programs are no longer measured; publication blocked: '+', '.join(lost))
    stage=folder/'assembled';stage.mkdir(exist_ok=True)
    ordered=[rows[p['id']] for p in manifest['programs']];measured=[r for r in ordered if r['status']=='measured']
    totals={'programs':len(ordered),'measured':len(measured),'notBuilt':len(ordered)-len(measured),'instructions':0,'weights':dict.fromkeys(['gross','net','tare','inputs'],0),'binarySize':dict.fromkeys(['gross','net','tare'],0),'rentLamports':dict.fromkeys(['gross','net','tare'],0),'corpus':dict.fromkeys(['tests','generated','mainnet'],0)}
    for row in measured:
        w=row['weighing'];totals['instructions']+=len(w['instructions'])
        for k in totals['weights']:totals['weights'][k]+=w['totals'][k] or 0
        for group,field in [('binarySize','sizeBytes'),('rentLamports','rentLamports')]:
            gross,net=(w['builds'][p][field] for p in ['stock','tare']);totals[group]['gross']+=gross;totals[group]['net']+=net;totals[group]['tare']+=gross-net
        for k in totals['corpus']:totals['corpus'][k]+=row['proof']['corpus']['counts'][k]
    provenance=read(folder/'provenance.json');version=subprocess.check_output([str(folder/next(iter(provenance['results']))/('tare.exe' if os.name=='nt' else 'tare')),'--version'],text=True).strip().removeprefix('tare ')
    document={'schemaVersion':'1','fixture':False,'toolchainVersion':version,'generatedAt':dt.datetime.now(dt.timezone.utc).isoformat(timespec='milliseconds').replace('+00:00','Z'),'programs':ordered,'totals':totals}
    versions=list(dict.fromkeys(row['anchorVersion'] for row in ordered))
    construct_names=sorted({c for p in manifest['programs'] for c in p['constructs']}|{r.get('construct') for r in ordered if r['status']=='not_built' and r.get('construct')})
    constructs=[]
    for construct in construct_names:
        entries=[]
        for anchor in versions:
            exercises=[r['id'] for r in measured if r['anchorVersion']==anchor and r['weighing']['totals']['inputs']>0 and construct in known[r['id']]['constructs']]
            refusals=[r for r in ordered if r['anchorVersion']==anchor and r['status']=='not_built' and r.get('construct')==construct]
            entries.append({'anchorVersion':anchor,'supported':bool(exercises),'programs':exercises,'note':'Compiled with matched proof and successful inputs; not complete branch coverage.' if exercises else '\n'.join(r['id']+': '+r['reason'] for r in refusals) or 'Not exercised by a matched corpus run.'})
        constructs.append({'construct':construct,'versions':entries})
    compat={'schemaVersion':'1','fixture':False,'toolchainVersion':version,'generatedAt':document['generatedAt'],'anchorVersions':versions,'constructs':constructs}
    schema('BenchResults',document);schema('Compat',compat);write(stage/'results.json',document);write(stage/'compat.json',compat);write(stage/'provenance.json',{'inputs':inputs,'rows':origins,'artifacts':{k:rel(v) for k,v in artifacts.items()},'runtime':rel(folder/'provenance.json')})
    gate=subprocess.run(['node',str(BENCH/'gate.mjs'),'--summary','--results',str(stage/'results.json'),'--output',str(stage/'gate.json')],capture_output=True,text=True)
    (stage/'gate.txt').write_text(gate.stdout+gate.stderr,encoding='utf-8',newline='\n')
    if publish:
        # All rows, full proof schemas, every replay checksum and every ELF are
        # validated before the first canonical artifact is changed.
        backup=folder/'before-publication';backup.mkdir()
        for name in ['results.json','compat.json']:
            if (BENCH/name).exists():shutil.copy2(BENCH/name,backup/name)
        for ident,source in artifacts.items():shutil.copytree(source,BENCH/'artifacts'/ident,dirs_exist_ok=True)
        for lane in provenance['results']:
            source=folder/lane/'bench/transcripts'
            if source.exists():
                for p in source.iterdir():
                    if p.is_file():shutil.copy2(p,BENCH/'transcripts'/p.name)
        shutil.copy2(stage/'results.json',BENCH/'results.json');shutil.copy2(stage/'compat.json',BENCH/'compat.json')
    print(gate.stdout,end='')
    return gate.returncode


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--home',type=Path);p.add_argument('--cli',type=Path);p.add_argument('--cli-sha256');p.add_argument('--run',required=True);p.add_argument('--seed-cache-home',type=Path);p.add_argument('--seed-phases',default='stock');p.add_argument('--seed-version-cache',action='append',default=[]);p.add_argument('--cache-quiescent',action='store_true');p.add_argument('--jobs',type=int,default=3,choices=[1,2,3,4]);p.add_argument('--prepare-only',action='store_true');p.add_argument('--resume',action='store_true');p.add_argument('--assemble',action='store_true');p.add_argument('--publish',action='store_true');p.add_argument('--supplement',action='append',default=[],help='RESULTS|ARTIFACT_DIRECTORY|PROVENANCE');a=p.parse_args()
    if Path(a.run).name!=a.run:raise ValueError('--run must be a single folder name')
    folder=HISTORY/a.run
    if a.assemble:return assemble(folder,a.publish,a.supplement)
    if a.publish:raise ValueError('--publish requires --assemble')
    if not a.home or not a.cli or not a.cli_sha256:raise ValueError('Run requires --home --cli --cli-sha256')
    ids=[r['id'] for r in read(BENCH/'results.json')['programs'] if r['status']=='measured']
    selection=HISTORY/(a.run+'-ids.json');write(selection,ids)
    command=[sys.executable,str(BENCH/'economic-run.py'),'--home',str(a.home),'--cli',str(a.cli),'--cli-sha256',a.cli_sha256,'--run',a.run,'--ids',str(selection),'--jobs',str(a.jobs)]
    if a.seed_cache_home:command += ['--seed-cache-home',str(a.seed_cache_home),'--seed-phases',a.seed_phases]
    for option in a.seed_version_cache:command += ['--seed-version-cache',option]
    if a.cache_quiescent:command.append('--cache-quiescent')
    if a.prepare_only:command.append('--prepare-only')
    if a.resume:command.append('--resume')
    return subprocess.run(command).returncode


if __name__=='__main__':
    try:sys.exit(main())
    except (ValueError,OSError,jsonschema.ValidationError) as e:print(str(e),file=sys.stderr);sys.exit(2)
