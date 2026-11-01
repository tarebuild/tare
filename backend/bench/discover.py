"""Enumerate unchanged SBF program manifests from pinned Anchor checkouts."""
import json, os, pathlib, re, subprocess, tomllib
HERE = pathlib.Path(__file__).resolve().parent
old = json.loads((HERE/'manifest.json').read_text())
repos = [
    {'id':'anchor-0.31.1','anchorVersion':'0.31.1','checkout':'corpus/anchor','tag':'v0.31.1'},
    {'id':'anchor-0.32.3','anchorVersion':'0.32.3','checkout':'corpus/anchor-0.32.3','tag':'v0.32.3'},
    {'id':'anchor-1.2.2','anchorVersion':'1.2.2','checkout':'corpus/anchor-1.2.2','tag':'v1.2.2'},
]
known = {p['manifest']:p for p in old['programs'] if p.get('anchorVersion')=='0.31.1'}
labels = ['Account','Signer','Program','SystemAccount','UncheckedAccount','AccountLoader','InterfaceAccount','Interface','Context','CpiContext','Sysvar','Option','Box','Lamports','AccountsClose','Space','InitSpace','Migration','AnchorSerialize','AnchorDeserialize','LazyAccount','RefMut','ExtraAccountMetaList','ProgramData','UpgradeableLoaderState']
attrs = ['program','account','event','event_cpi','error_code','access_control','interface','constant','instruction','zero_copy']
macros = ['emit','emit_cpi','require','require_eq','require_neq','require_keys_eq','require_keys_neq','require_gt','require_gte','err','error','declare_program']
constraints = ['init','init_if_needed','zero','realloc','close','seeds','bump','has_one','constraint','address','owner','rent_exempt','executable','payer','space','token','mint','associated_token','zero_copy','token_2022','token_interface','metadata','memo']
programs=[]
for repo in repos:
 root = HERE/repo['checkout']
 repo['repository']='https://github.com/solana-foundation/anchor'
 repo['revision']=subprocess.check_output(['git','-C',str(root),'rev-parse','HEAD'],text=True).strip()
 found=[]
 for area in ['examples','tests']:
  for directory,dirs,files in os.walk(root/area):
   dirs[:]=[d for d in dirs if d not in {'.git','target','node_modules'} and not (pathlib.Path(directory)/d/'.git').exists()]
   if 'Cargo.toml' not in files: continue
   file=pathlib.Path(directory)/'Cargo.toml'
   data=tomllib.loads(file.read_text(encoding='utf-8'))
   if not isinstance(data.get('package'),dict): continue
   if 'cdylib' not in data.get('lib',{}).get('crate-type',[]): continue
   package=data['package']; name=data.get('lib',{}).get('name',package['name'].replace('-','_'))
   rel=file.relative_to(root).as_posix()
   sources='\n'.join(f.read_text(encoding='utf-8') for f in (file.parent/'src').rglob('*.rs')) if (file.parent/'src').exists() else ''
   features=[]
   for label in labels:
    if re.search(r'\b'+re.escape(label)+r'\b',sources):features.append(label)
   for attr in attrs:
    if re.search(r'#\[\s*'+attr+r'\b',sources):features.append('#['+attr+']')
   if re.search(r'\bAccounts\b',sources): features.append('#[derive(Accounts)]')
   for macro in macros:
    if re.search(r'\b'+macro+r'\s*!',sources):features.append(macro+'!')
   for constraint in constraints:
    if re.search(r'\b'+constraint+r'\b',sources):features.append(constraint)
   if any(x in sources for x in ['#[account(', '#[account\n']):features.append('account_constraints')
   if 'bump' in features:features.append('bumps')
   if 'remaining_accounts' in sources:features.append('remaining_accounts')
   if any(x in features for x in ['require_eq!','require_neq!','require_keys_eq!','require_keys_neq!','require_gt!','require_gte!']):features.append('require_family')
   ws=file.parent
   while ws!=root:
    cargo=ws/'Cargo.toml'
    if cargo.exists() and 'workspace' in tomllib.loads(cargo.read_text(encoding='utf-8')):break
    ws=ws.parent
   legacy=known.get(rel) if repo['anchorVersion']=='0.31.1' else None
   key=legacy['id'] if legacy else 'v'+repo['anchorVersion'].replace('.','_')+'-'+rel.removesuffix('/Cargo.toml').replace('/','-')
   found.append({'id':key,'name':name,'kind':'anchor_example' if area=='examples' else 'anchor_test','anchorVersion':repo['anchorVersion'],'workspace':ws.relative_to(root).as_posix(),'manifest':rel,'constructs':sorted(set(features)),'repository':repo['repository'],'revision':repo['revision'],'checkout':repo['checkout'],'complexity':len(sources)})
   environment={}
   for variable in re.findall(r'env!\(\s*"([A-Z][A-Z0-9_]*)"',sources):
    for script in [ws/'test.sh',ws/'generate.sh']:
     if not script.exists():continue
     match=re.search(r'\b'+re.escape(variable)+r'=([A-Za-z0-9_]+)\b',script.read_text())
     if match:environment[variable]=match.group(1)
   if environment:found[-1]['environment']=environment

 # Keep established rows at the front; then cheapest source first.
 order={p['id']:i for i,p in enumerate(old['programs'])}
 found.sort(key=lambda p:(0,order[p['id']]) if p['id'] in order else (1,p['complexity'],p['id']))
 programs.extend(found)
 result={**{k:old[k] for k in ['schemaVersion','repository','revision','tag','checkout']},'repositories':repos,'programs':programs}
(HERE/'manifest.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
print('Programs:',len(programs), {r['anchorVersion']:sum(p['anchorVersion']==r['anchorVersion'] for p in programs) for r in repos})
