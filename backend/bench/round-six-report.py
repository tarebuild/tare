"""Render the short round-six current-stock report from verified final facts."""
from collections import Counter
from pathlib import Path
import argparse
import hashlib
import json
import os
import sys
import jsonschema

ROOT=Path(__file__).resolve().parent.parent
BENCH=ROOT/'bench'

def read(p):return json.loads(p.read_text(encoding='utf-8'))
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def link(p,base):return Path(os.path.relpath(p,base)).as_posix()
def cell(s):return str(s).replace('|','&#124;').replace('\n',' ')
def n(x):return f'{x:,}' if isinstance(x,int) else str(x)

def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--results',type=Path,default=BENCH/'results.json');p.add_argument('--gate',type=Path,default=BENCH/'gaps/round-six/gate.json');p.add_argument('--causes',type=Path,default=BENCH/'gaps/round-six/gap-causes.json');p.add_argument('--output',type=Path,default=BENCH/'round-six.md');p.add_argument('--preview',action='store_true');a=p.parse_args()
 results=read(a.results);gate=read(a.gate);causes=read(a.causes);manifest=read(BENCH/'manifest.json');rows=results['programs'];measured=[r for r in rows if r['status']=='measured'];issues=[]
 jsonschema.Draft202012Validator(read(ROOT/'contract/schema/BenchResults.json')).validate(results)
 if gate.get('comparison')!='current_stock' or gate['candidateResultsSha256']!=sha(a.results):issues.append('Gate/result hash mismatch')
 if causes['resultsSha256']!=sha(a.results) or causes['gateSha256']!=sha(a.gate):issues.append('Gap evidence hash mismatch')
 if set(r['id'] for r in rows)!=set(r['id'] for r in manifest['programs']):issues.append('Manifest coverage mismatch')
 if gate.get('coverageIssues'):issues+=gate['coverageIssues']
 checks=[]
 for r in measured:
  if r['proof']['status']!='matched':issues.append('Proof diverged: '+r['id'])
  for kind in ['stock','tare']:
   elf=BENCH/'artifacts'/r['id']/(kind+'.so');b=r['weighing']['builds'][kind];ok=elf.exists() and elf.stat().st_size==b['sizeBytes'] and sha(elf)==b['hash'];checks.append({'id':r['id'],'kind':kind,'matched':ok})
   if not ok:issues.append('Artifact differs: '+r['id']+'/'+kind)
 if issues and not a.preview:raise ValueError('; '.join(issues))
 by_id={r['id']:r for r in rows};why={r['id']:r for r in causes['programs']};t=results['totals'];fail=[r for r in gate['programs'] if not r['passed']];fail.sort(key=lambda r:(r['stockBytes']-r['candidateBytes'],-max([x['increase'] for x in r['regressions']] or [0]),r['id']))
 proof_count=sum(r['proof']['comparisons']['inputs'] for r in measured)
 lines=['# Tare round six','',f"Tare {results['toolchainVersion']}. {len(rows)} manifest entries: {len(measured)} measured, {len(rows)-len(measured)} explicitly not built. All {sum(r['proof']['status']=='matched' for r in measured)} measured proofs match across {n(proof_count)} transactions.",'',f"**Current-stock gate: {'PASS' if gate['passed'] else 'FAIL'}.** {gate['counts']['passed']}/{gate['counts']['programs']} measured programs pass; {gate['counts']['sizeRegressions']} ELF size failures; {gate['counts']['computeRegressions']} instruction-CU failures. The threshold is Tare size <= stock and net <= gross for reached instructions; production instructions require net < gross.",'',f"Measured instruction-median totals: gross **{n(t['weights']['gross'])}**, net **{n(t['weights']['net'])}**, tare **{n(t['weights']['tare'])}** CU over {n(t['weights']['inputs'])} successful samples. Binary totals: stock {n(t['binarySize']['gross'])} B, Tare {n(t['binarySize']['net'])} B. Corpus: {n(t['corpus']['generated'])} generated, {n(t['corpus']['tests'])} recorded tests, {n(t['corpus']['mainnet'])} mainnet inputs.",'','## Production and registry','', '| Program | Gross/net CU | Stock/Tare bytes | Measured instructions | Proof inputs | Own tests |','|---|---:|---:|---:|---:|---:|']
 for r in rows:
  if not (r['id'].startswith('production-') or r['id']=='tare-registry'):continue
  if r['status']!='measured':lines.append(f"| {r['id']} | not built: {cell(r.get('construct') or r['reason'].splitlines()[0])} | - | - | - | - |");continue
  w=r['weighing'];lines.append(f"| {r['id']} | {n(w['totals']['gross'])}/{n(w['totals']['net'])} | {n(w['builds']['stock']['sizeBytes'])}/{n(w['builds']['tare']['sizeBytes'])} | {sum(x['inputs']>0 for x in w['instructions'])}/{len(w['instructions'])} | {r['proof']['comparisons']['inputs']} | {r['tests']['passed']}/{r['tests']['ran']} |")
 lines+=['','## Remaining failures','', '| Program | Excess bytes | Largest CU failure | Cause |','|---|---:|---:|---|']
 for r in fail:
  excess=max(0,r['candidateBytes']-r['stockBytes']);max_cu=max([x['increase'] for x in r['regressions']] or [0]);desc=why.get(r['id'],{}).get('cause','Evidence missing');lines.append(f"| {r['id']} | {n(excess)} | {n(max_cu)} | {cell(desc)} |")
 if not fail:lines.append('| none | 0 | 0 | All measured programs satisfy the current-stock threshold. |')
 cu=[(r['id'],x) for r in fail for x in r['regressions']];cu.sort(key=lambda p:-p[1]['increase'])
 if cu:
  lines+=['','| Program / instruction | Stock CU | Tare CU | Difference |','|---|---:|---:|---:|']
  for ident,x in cu:lines.append(f"| {ident} / {x['instruction']} | {n(x['stock'])} | {n(x['candidate'])} | {n(x['increase'])} |")
 lines+=['','## Checks and handoff','',f"The final result passes its published schema; {len(checks)} stock/Tare artifact hashes and sizes are checked against the rows. Full proof/replay validation is retained by the final-run assembly. [Exact gate]({link(a.gate,a.output.parent)}), [size and code-family evidence]({link(a.causes,a.output.parent)}) and [all result rows]({link(a.results,a.output.parent)}) retain every failure.",'','```sh','node bench/gate.mjs --summary','python bench/round-six-report.py','```','','The Linux/container/validator checks are separate. Run the exact fresh-volume and registry/API commands in [lead-run.md]('+link(BENCH/'lead-run.md',a.output.parent)+'). No Linux validator success is inferred from native SVM proofs.','','Assumptions: totals add per-instruction medians, not transaction-sequence cost; unmeasured instructions contribute no estimate; production addresses identify source programs and do not prove deployed mainnet ELF identity; the 49 earlier refusals retain their recorded provenance. Static section/symbol evidence identifies retained code families, not an exact causal CU split.','']
 facts={'resultsSha256':sha(a.results),'gateSha256':sha(a.gate),'causesSha256':sha(a.causes),'issues':issues,'artifactChecks':checks,'totals':t,'proofTransactions':proof_count,'failures':len(fail),'gatePassed':gate['passed']}
 a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text('\n'.join(lines),encoding='utf-8',newline='\n');a.output.with_suffix('.json').write_text(json.dumps(facts,indent=2)+'\n',encoding='utf-8',newline='\n');print(f"Wrote {a.output}; {len(measured)} measured, {len(fail)} failing current-stock gate")
 return int(bool(issues) or not gate['passed'])

if __name__=='__main__':
 try:sys.exit(main())
 except (ValueError,OSError,jsonschema.ValidationError) as e:print(str(e),file=sys.stderr);sys.exit(2)
