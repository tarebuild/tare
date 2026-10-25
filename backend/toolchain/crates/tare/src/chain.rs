use super::*;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use reqwest::blocking::Client;
use solana_address::Address;
use solana_instruction::{Instruction, account_meta::AccountMeta};
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_transaction::{Transaction, versioned::VersionedTransaction};
use std::{io::{Read, Write}, time::Duration};

#[derive(Args, Clone)]
pub struct Network {
    #[arg(long, env="TARE_RPC_URL")]
    url: Option<String>,
    #[arg(long, env="TARE_KEYPAIR")]
    keypair: Option<PathBuf>,
    #[arg(long, env="TARE_API_URL")]
    api_url: Option<String>,
    #[arg(long, env="TARE_REGISTRY_PROGRAM_ID")]
    registry_program: Option<String>,
}
#[derive(Args)]
pub struct AttestArgs {
    #[command(flatten)] workspace: Workspace,
    #[command(flatten)] network: Network,
}
#[derive(Args)]
pub struct CheckArgs {
    record: String,
    #[command(flatten)] network: Network,
    #[arg(long)] host: bool,
    #[arg(long)] directory: Option<PathBuf>,
}
#[derive(Args)]
pub struct FlagArgs {
    record: String,
    #[arg(long)] input: PathBuf,
    #[command(flatten)] network: Network,
    #[arg(long)] host: bool,
    #[arg(long)] directory: Option<PathBuf>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct ReplayBundle {
    pub schema_version: String,
    pub manifest_path: String,
    pub generated: tare_prove::Corpus,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub supplied_idl: Option<Value>,
    pub programs: Vec<tare_prove::RecordedProgram>,
    pub tests: Option<tare_prove::RecordedCorpus>,
    pub mainnet: Option<tare_prove::RecordedCorpus>,
}
pub fn pack_replay(bundle: &ReplayBundle) -> Result<Value> {
    let bytes=serde_json::to_vec(bundle)?;
    let mut encoder=flate2::write::GzEncoder::new(Vec::new(),flate2::Compression::best());
    encoder.write_all(&bytes)?;
    Ok(json!({"encoding":"gzip+base64","sha256":digest(&bytes),"data":BASE64.encode(encoder.finish()?)}))
}
fn unpack_replay(report:&Value)->Result<ReplayBundle>{
    let value=&report["replay"];
    if value["encoding"]!="gzip+base64"{bail!("report has no supported replay manifest")}
    let compressed=BASE64.decode(value["data"].as_str().context("replay data")?)?;
    let mut bytes=Vec::new();flate2::read::GzDecoder::new(compressed.as_slice()).take(256*1024*1024+1).read_to_end(&mut bytes)?;
    if bytes.len()>256*1024*1024 {bail!("replay manifest exceeds 256 MiB")}
    if value["sha256"]!=digest(&bytes){bail!("replay manifest hash mismatch")}
    let bundle:ReplayBundle=serde_json::from_slice(&bytes)?;
    if bundle.schema_version!="1"{bail!("unsupported replay manifest version")}
    Ok(bundle)
}
struct Rpc {client:Client,url:String}
impl Rpc {
    fn new(url:Option<&str>)->Result<Self>{Ok(Self{client:Client::builder().timeout(Duration::from_secs(30)).build()?,url:url.context("supply --url or TARE_RPC_URL")?.into()})}
    fn call(&self,method:&str,params:Value)->Result<Value>{
        let response=self.client.post(&self.url).json(&json!({"jsonrpc":"2.0","id":1,"method":method,"params":params})).send().map_err(|e|e.without_url())?.error_for_status().map_err(|e|e.without_url())?;
        let mut bytes=Vec::new();response.take(128*1024*1024+1).read_to_end(&mut bytes)?;
        if bytes.len()>128*1024*1024 {bail!("RPC response exceeds 128 MiB")}
        let value:Value=serde_json::from_slice(&bytes)?;
        if !value["error"].is_null(){bail!("RPC {method}: {}",value["error"])}
        Ok(value["result"].clone())
    }
    fn account(&self,address:&Address)->Result<Value>{
        self.call("getAccountInfo",json!([address.to_string(),{"encoding":"base64","commitment":"confirmed"}]))?["value"].as_object().map(|v|Value::Object(v.clone())).context("account does not exist")
    }
    fn send(&self,ix:Instruction,payer:&Keypair)->Result<String>{
        let latest=self.call("getLatestBlockhash",json!([{"commitment":"confirmed"}]))?;
        let hash=latest["value"]["blockhash"].as_str().context("RPC blockhash")?.parse()?;
        let tx=Transaction::new_signed_with_payer(&[ix],Some(&payer.pubkey()),&[payer],hash);
        let result=self.call("sendTransaction",json!([BASE64.encode(bincode::serialize(&tx)?),{"encoding":"base64","skipPreflight":false,"preflightCommitment":"confirmed","maxRetries":3}]))?;
        let signature=result.as_str().context("RPC signature")?.to_owned();
        for _ in 0..60 {
            let status=self.call("getSignatureStatuses",json!([[signature],{"searchTransactionHistory":true}]))?["value"][0].clone();
            if !status.is_null(){
                if !status["err"].is_null(){bail!("transaction failed: {}",status["err"])}
                if matches!(status["confirmationStatus"].as_str(),Some("confirmed"|"finalized")){return Ok(signature)}
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        bail!("transaction not confirmed within 30 seconds: {signature}")
    }
}
/// The Tare registry on mainnet.
const REGISTRY_PROGRAM_ID:&str="7viqDjx5k8ffnfnZiBv9dMhxzVhcGSYLMb5bKdDP1XtP";
const API_URL:&str="https://api.tare.cx";
fn registry(network:&Network)->Result<Address>{
    Ok(network.registry_program.as_deref().unwrap_or(REGISTRY_PROGRAM_ID).parse()?)
}
fn payer(network:&Network)->Result<Keypair>{
    let path=network.keypair.as_ref().context("supply --keypair or TARE_KEYPAIR")?;
    let bytes:Vec<u8>=serde_json::from_slice(&fs::read(path)?)?;
    Keypair::try_from(bytes.as_slice()).map_err(|e|anyhow::anyhow!("invalid keypair: {e}"))
}
fn api(network:&Network)->Result<&str>{Ok(network.api_url.as_deref().unwrap_or(API_URL).trim_end_matches('/'))}
fn api_get(network:&Network,path:&str)->Result<Value>{
    let response=Client::builder().timeout(Duration::from_secs(30)).build()?.get(format!("{}{path}",api(network)?)).send().map_err(|e|e.without_url())?.error_for_status().map_err(|e|e.without_url())?;
    let mut bytes=Vec::new();response.take(1024*1024+1).read_to_end(&mut bytes)?;if bytes.len()>1024*1024{bail!("API report exceeds 1 MiB")};Ok(serde_json::from_slice(&bytes)?)
}
fn upload(network:&Network,path:&str,value:&Value)->Result<(Vec<u8>,String)>{
    let bytes=serde_json::to_vec(&wire::document(value))?;
    if bytes.len()>1024*1024{bail!("report exceeds the API's 1 MiB limit; compressed replay manifest needs a larger content limit")}
    let response=Client::builder().timeout(Duration::from_secs(30)).build()?.post(format!("{}{path}",api(network)?)).header("Content-Type","application/json").body(bytes.clone()).send().map_err(|e|e.without_url())?;
    let status=response.status();let body=response.text()?;
    if !status.is_success(){bail!("API upload failed ({status}): {body}")}
    let value:Value=serde_json::from_str(&body)?;let hash=hex::decode(value["hash"].as_str().context("upload hash")?)?;
    if hex::encode(&hash)!=digest(&bytes){bail!("API content hash does not match canonical uploaded bytes")}
    Ok((hash,value["location"].as_str().context("upload location")?.into()))
}
fn address(value:&Value)->Result<Address>{value.as_str().context("address")?.parse().map_err(Into::into)}
fn hash(value:&Value)->Result<Vec<u8>>{let v=hex::decode(value.as_str().context("32-byte hash")?)?;if v.len()!=32{bail!("expected 32-byte hash")};Ok(v)}
fn text(out:&mut Vec<u8>,s:&str){out.extend((s.len() as u32).to_le_bytes());out.extend(s.as_bytes())}
fn vector(out:&mut Vec<u8>,v:&[u8]){out.extend((v.len() as u32).to_le_bytes());out.extend(v)}
fn discriminator(name:&str)->Vec<u8>{Sha256::digest(name.as_bytes())[..8].to_vec()}
fn pda(program:&Address,seeds:&[&[u8]])->Address{Address::find_program_address(seeds,program).0}
fn readonly(key:Address)->AccountMeta{AccountMeta::new_readonly(key,false)}
fn event_accounts(program:&Address)->[AccountMeta;2]{[readonly(pda(program,&[b"__event_authority"])),readonly(*program)]}
fn account_bytes(account:&Value,program:&Address)->Result<Vec<u8>>{
    if address(&account["owner"])?!=*program{bail!("registry account owner mismatch")}
    BASE64.decode(account["data"][0].as_str().context("account data")?).map_err(Into::into)
}
struct Decoder<'a>{bytes:&'a[u8],at:usize}
impl<'a> Decoder<'a>{
 fn take(&mut self,n:usize)->Result<&'a[u8]>{let end=self.at.checked_add(n).context("account size overflow")?;let v=self.bytes.get(self.at..end).context("short registry account")?;self.at=end;Ok(v)}
 fn u64(&mut self)->Result<u64>{Ok(u64::from_le_bytes(self.take(8)?.try_into()?))}
 fn bytes(&mut self,max:usize)->Result<Vec<u8>>{let n=u32::from_le_bytes(self.take(4)?.try_into()?) as usize;if n>max{bail!("registry field exceeds declared bound")};Ok(self.take(n)?.to_vec())}
 fn string(&mut self,max:usize)->Result<String>{String::from_utf8(self.bytes(max)?).map_err(Into::into)}
}
#[derive(Serialize)]
struct Record {program:String,stock_build_hash:String,tare_build_hash:String,source_repository:String,source_commit:String,toolchain_version:String,corpus_hash:String,tests_count:u64,generated_count:u64,mainnet_count:u64,gross:u64,net:u64,report_hash:String,report_location:String,attester:String,slot:u64}
fn read_record(rpc:&Rpc,program:&Address,record:&Address)->Result<Record>{
 let bytes=account_bytes(&rpc.account(record)?,program)?;let mut d=Decoder{bytes:&bytes,at:0};
 if d.take(8)?!=discriminator("account:Record"){bail!("not a registry Record")}
 let mut key=||Ok::<String,anyhow::Error>(Address::try_from(d.take(32)?).map_err(|e|anyhow::anyhow!("address: {e}"))?.to_string());
 let program=key()?;drop(key);
 Ok(Record{program,stock_build_hash:hex::encode(d.take(32)?),tare_build_hash:hex::encode(d.take(32)?),source_repository:d.string(256)?,source_commit:hex::encode(d.bytes(32)?),toolchain_version:d.string(64)?,corpus_hash:hex::encode(d.take(32)?),tests_count:d.u64()?,generated_count:d.u64()?,mainnet_count:d.u64()?,gross:d.u64()?,net:d.u64()?,report_hash:hex::encode(d.take(32)?),report_location:d.string(512)?,attester:Address::try_from(d.take(32)?).map_err(|e|anyhow::anyhow!("attester: {e}"))?.to_string(),slot:d.u64()?})
}
fn proof_file(workspace:&Workspace)->Result<(Package,PathBuf,Value)>{let package=discover(&workspace.manifest_path)?;let file=package.root.join("target/tare").join(format!("{}.proof.json",package.lib_name));let proof:Value=serde_json::from_slice(&fs::read(&file)?)?;Ok((package,file,proof))}
pub fn attest(args:AttestArgs)->Result<()>{
 let(package,_,mut proof)=proof_file(&args.workspace)?;
 wire::canonicalize_source(&mut proof["source"]);
 let built:BuildRecord=serde_json::from_slice(&fs::read(package.root.join("target/tare").join(format!("{}.build.json",package.lib_name)))?)?;
 if built.rehearsal_defect.is_some() { bail!("refusing to attest a rehearsal-defect build; unset TARE_REHEARSAL_DEFECT and rebuild") }
 if proof["proof"]["status"]!="matched"{bail!("attestation requires a matching differential proof")}
 let dirty=output(git_command(&package.root).args(["status","--porcelain","--untracked-files=normal"]))?;
 if !dirty.trim().is_empty(){bail!("refusing to attest a dirty source tree")}
 let commit=git(&package.root,&["rev-parse","HEAD"]).context("workspace must be in git")?;
 let origin=git(&package.root,&["remote","get-url","origin"]).context("workspace needs a source origin")?;
 let repository=wire::canonical_repository(&origin);
 let repository_url=reqwest::Url::parse(&repository).context("attestation needs a public source URL")?;
 if !repository_url.username().is_empty() || repository_url.password().is_some() || repository_url.query().is_some(){bail!("refusing to publish credentials or query parameters in source URL")}
 if proof["source"]["commit"]!=commit||proof["source"]["repository"]!=repository{bail!("proof source does not match the current git origin and commit")}
 let rpc=Rpc::new(args.network.url.as_deref())?;let registry=registry(&args.network)?;let signer=payer(&args.network)?;
 let config=pda(&registry,&[b"config"]);let config_data=account_bytes(&rpc.account(&config)?,&registry)?;
 if config_data.get(..8)!=Some(discriminator("account:Config").as_slice()){bail!("registry Config discriminator mismatch")}
 let fee_recipient=Address::try_from(config_data.get(40..72).context("short Config")?).map_err(|e|anyhow::anyhow!("fee recipient: {e}"))?;
 let fee=u64::from_le_bytes(config_data.get(72..80).context("short Config")?.try_into()?);
 println!("attest fee: {fee} lamports");
 for (kind,path) in [("stock",package.root.join("target/deploy").join(format!("{}.so",package.lib_name))),("tare",package.root.join("target/tare").join(format!("{}.so",package.lib_name)))] {if proof["builds"][kind]["hash"]!=digest(&fs::read(path)?){bail!("proof {kind} build hash does not match the current artifact")}}
 let mut report = proof.clone();
 report.as_object_mut().context("proof object")?.remove("optimization");report.as_object_mut().unwrap().remove("recordedCorpus");
 let (report_hash,location)=upload(&args.network,"/v1/reports",&report)?;
 let program=address(&proof["program"]["address"])?;let record=pda(&registry,&[b"record",program.as_ref(),&report_hash,signer.pubkey().as_ref()]);
 let mut data=discriminator("global:attest");data.extend(program.as_ref());data.extend(hash(&proof["builds"]["stock"]["hash"])?);data.extend(hash(&proof["builds"]["tare"]["hash"])?);text(&mut data,&repository);vector(&mut data,&hex::decode(&commit)?);text(&mut data,proof["toolchainVersion"].as_str().context("toolchain version")?);data.extend(hash(&proof["proof"]["corpus"]["hash"])?);
 for field in ["tests","generated","mainnet"]{data.extend(wire::integer(&proof["proof"]["corpus"]["counts"][field]).context("corpus count")?.to_le_bytes())}
 for field in ["gross","net"]{data.extend(wire::integer(&proof["totals"][field]).context("total")?.to_le_bytes())}
 data.extend(&report_hash);text(&mut data,&location);
 let mut accounts=vec![AccountMeta::new(signer.pubkey(),true),readonly(config),AccountMeta::new(fee_recipient,false),AccountMeta::new(record,false),readonly(Address::default())];accounts.extend(event_accounts(&registry));
 let signature=rpc.send(Instruction{program_id:registry,accounts,data},&signer)?;
 let config=api_get(&args.network,"/v1/config")?;
 println!("record: {record}\ntransaction: {signature}");
 if let Some(template)=config["recordUrlTemplate"].as_str(){println!("{}",template.replace("{record}",&record.to_string()))}
 Ok(())
}
fn load_report(network:&Network,record:&Record)->Result<Value>{let report=api_get(network,&format!("/v1/reports/{}",record.report_hash))?;if digest(&serde_json::to_vec(&report)?)!=record.report_hash{bail!("report content hash mismatch")};Ok(report)}
fn rebuild(record_address:&str,network:&Network,host:bool,directory:Option<PathBuf>)->Result<(Record,Workspace,Value)>{
 let rpc=Rpc::new(network.url.as_deref())?;let reg=registry(network)?;let record=read_record(&rpc,&reg,&record_address.parse()?)?;let report=load_report(network,&record)?;
 for (field,actual,expected) in [("program",report["program"]["address"].as_str(),record.program.as_str()),("stock hash",report["builds"]["stock"]["hash"].as_str(),record.stock_build_hash.as_str()),("tare hash",report["builds"]["tare"]["hash"].as_str(),record.tare_build_hash.as_str()),("corpus hash",report["proof"]["corpus"]["hash"].as_str(),record.corpus_hash.as_str()),("source commit",report["source"]["commit"].as_str(),record.source_commit.as_str())] {if actual!=Some(expected){bail!("record/report {field} differs")}}

 if report["source"]["repository"].as_str().map(wire::canonical_repository)!=Some(wire::canonical_repository(&record.source_repository)){bail!("record/report source repository differs")}
 if report["toolchainVersion"]!=record.toolchain_version {bail!("record/report toolchain version differs")}
 for (name,expected) in [("tests",record.tests_count),("generated",record.generated_count),("mainnet",record.mainnet_count)] {if wire::integer(&report["proof"]["corpus"]["counts"][name])!=Some(expected){bail!("record/report {name} count differs")}}
 for (name,expected) in [("gross",record.gross),("net",record.net)] {if wire::integer(&report["totals"][name])!=Some(expected){bail!("record/report {name} total differs")}}
 println!("record: {record_address}\ncommit: {}",record.source_commit);
 if !report["buildEnvironment"].is_null() {
  let current=build_environment(host)?;
  if report["buildEnvironment"]!=current {
   bail!("build environment differs: record {}, current {}; use the recorded environment",environment_label(&report["buildEnvironment"]),environment_label(&current));
  }
 }
 if host&&record.toolchain_version!=VERSION {bail!("record requires Tare {}; this host has {}",record.toolchain_version,VERSION)}

 let bundle=unpack_replay(&report)?;
 if report["proof"]["status"]!="matched"{bail!("record report does not assert a matched proof")}
 let dir=directory.unwrap_or(env::current_dir()?.join("target/tare/check").join(record_address));
 if !dir.exists(){fs::create_dir_all(&dir)?;run(Command::new("git").arg("init").arg(&dir))?;run(Command::new("git").arg("-C").arg(&dir).args(["remote","add","origin",&record.source_repository]))?;run(Command::new("git").arg("-C").arg(&dir).args(["fetch","--depth","1","origin",&record.source_commit]))?;run(Command::new("git").arg("-C").arg(&dir).args(["checkout","--detach","FETCH_HEAD"]))?;run(Command::new("git").arg("-C").arg(&dir).args(["submodule","update","--init","--recursive"]))?;}
 if git(&dir,&["rev-parse","HEAD"]).as_deref()!=Some(&record.source_commit){bail!("check directory commit mismatch")}
 let dirty=output(git_command(&dir).args(["status","--porcelain","--untracked-files=normal"]))?;
 if !dirty.trim().is_empty(){bail!("refusing a dirty check directory")}
 let dir=canonical(&dir)?;
 let relative=Path::new(&bundle.manifest_path);
 if relative.is_absolute()||bundle.manifest_path.contains('\\')||bundle.manifest_path.contains(':')||relative.components().any(|c|matches!(c,std::path::Component::ParentDir)){bail!("unsafe report manifest path")}
 let replay=dir.join("target/tare/replay-bundle.json");fs::create_dir_all(replay.parent().unwrap())?;write_json(&replay,&bundle)?;
 let supplied_idl=if let Some(idl)=&bundle.supplied_idl {let path=dir.join("target/tare/supplied-idl.json");write_json(&path,idl)?;Some(path)}else{None};
 let workspace=Workspace{host,features:bundle.features.clone(),idl:supplied_idl,manifest_path:dir.join(relative),tests_corpus:None,replay:Some(replay),mainnet_corpus:None};
 if !container_with_defect(&workspace,"build",None)?{build_with_defect(&workspace,None)?}
 let package=discover(&workspace.manifest_path)?;let built:BuildRecord=serde_json::from_slice(&fs::read(package.root.join("target/tare").join(format!("{}.build.json",package.lib_name)))?)?;
 if built.toolchain_version!=record.toolchain_version {bail!("built toolchain version differs: expected {}, got {}",record.toolchain_version,built.toolchain_version)}
 if built.stock_hash!=record.stock_build_hash {bail!("stock build hash differs: expected {}, got {}",record.stock_build_hash,built.stock_hash)}
 if built.tare_hash!=record.tare_build_hash {bail!("tare build hash differs: expected {}, got {}",record.tare_build_hash,built.tare_hash)}
 println!("stock hash: {} (matches)\ntare hash: {} (matches)",built.stock_hash,built.tare_hash);
 Ok((record,workspace,report))
}
pub fn check(args:CheckArgs)->Result<()>{
 let(record,workspace,_)=rebuild(&args.record,&args.network,args.host,args.directory)?;env::set_var("TARE_CHECK_SUMMARY","1");let measured=measure(&workspace,true);env::remove_var("TARE_CHECK_SUMMARY");measured?;let(_,_,report)=proof_file(&workspace)?;
 for (field,actual,expected) in [("stock build hash",report["builds"]["stock"]["hash"].as_str(),record.stock_build_hash.as_str()),("tare build hash",report["builds"]["tare"]["hash"].as_str(),record.tare_build_hash.as_str()),("corpus hash",report["proof"]["corpus"]["hash"].as_str(),record.corpus_hash.as_str())]{if actual!=Some(expected){bail!("{field} differs: expected {expected}, got {}",actual.unwrap_or("missing"))}}
 println!("corpus: {} transactions rerun",wire::integer(&report["proof"]["inputs"]).unwrap_or(record.tests_count+record.generated_count+record.mainnet_count));
 println!("matches");Ok(())
}
pub fn flag(args:FlagArgs)->Result<()>{
 let(record,workspace,report)=rebuild(&args.record,&args.network,args.host,args.directory)?;
 if let Some(defect)=rehearsal_defect()? {
  if !container_with_defect(&workspace,"build",Some(defect.clone()))? { build_with_defect(&workspace,Some(defect))?; }
 }
 let input:Value=serde_json::from_slice(&fs::read(&args.input)?)?;
 let input=input.get("divergence").and_then(|d|d.get("input")).unwrap_or(&input);
 let package=discover(&workspace.manifest_path)?;let dir=package.root.join("target/tare");let _build_record:BuildRecord=serde_json::from_slice(&fs::read(dir.join(format!("{}.build.json",package.lib_name)))?)?;
 let stock=fs::read(package.root.join("target/deploy").join(format!("{}.so",package.lib_name)))?;let net=fs::read(dir.join(format!("{}.so",package.lib_name)))?;
 let divergence=run_input(&record.program,&stock,&net,input,unpack_replay(&report)?.programs)?;
 let divergence=divergence.context("input does not diverge; no flag was uploaded or sent")?;
 let value=json!({"schemaVersion":"1","record":args.record,"divergence":divergence});
 let(input_hash,location)=upload(&args.network,"/v1/flag-inputs",&value)?;
 let rpc=Rpc::new(args.network.url.as_deref())?;let reg=registry(&args.network)?;let signer=payer(&args.network)?;let record_key:Address=args.record.parse()?;
 let flag=pda(&reg,&[b"flag",record_key.as_ref(),&input_hash]);let mut data=discriminator("global:flag");data.extend(&input_hash);text(&mut data,&location);
 let mut accounts=vec![AccountMeta::new(signer.pubkey(),true),readonly(record_key),AccountMeta::new(flag,false),readonly(Address::default())];accounts.extend(event_accounts(&reg));
 let signature=rpc.send(Instruction{program_id:reg,accounts,data},&signer)?;println!("flag: {flag}\ntransaction: {signature}");Ok(())
}
fn run_input(program:&str,stock:&[u8],tare:&[u8],input:&Value,programs:Vec<tare_prove::RecordedProgram>)->Result<Option<Value>>{
 let mut accounts=Vec::new();for a in input["accounts"].as_array().context("input accounts")?{let mut value=a.clone();value.as_object_mut().context("input account object")?.entry("rentEpoch").or_insert(json!(0));accounts.push(tare_prove::RecordedAccount{address:a["address"].as_str().context("input address")?.into(),account:Some(value)});}
 let frame=tare_prove::RecordedTransaction{label:input["label"].as_str().unwrap_or("flag").into(),input:None,stage:"landed".into(),transaction_base64:input["transactionBase64"].as_str().context("signed transaction input")?.into(),signature:None,pre_slot:wire::integer(&input["slot"]).unwrap_or(0),slot:wire::integer(&input["slot"]).unwrap_or(0),pre_accounts:accounts,post_accounts:Vec::new(),clock_exact:false,validator:tare_prove::ValidatorOutcome{error:Value::Null,logs:Vec::new(),return_data:None,compute_units:None}};
 let corpus=tare_prove::RecordedCorpus{schema_version:"1".into(),kind:"tests".into(),target_program:program.into(),stock_build_hash:digest(stock),capture_errors:Vec::new(),source_commit:String::new(),programs,sequences:vec![tare_prove::RecordedSequence{label:"flag".into(),genesis_hash:String::new(),transactions:vec![frame]}]};
 Ok(tare_prove::run_recorded(stock,tare,&corpus)?.differential.first_divergence)
}
pub fn prove_mainnet(workspace:&Workspace,program:&str,samples:u32,url:Option<&str>)->Result<()>{
 let url=url.map(str::to_owned).or_else(||env::var("TARE_MAINNET_RPC_URL").ok());let rpc=Rpc::new(url.as_deref())?;
 let package=discover(&workspace.manifest_path)?;let dir=package.root.join("target/tare");let record:BuildRecord=serde_json::from_slice(&fs::read(dir.join(format!("{}.build.json",package.lib_name)))?)?;
 if record.manifest_path!=package.manifest { bail!("build metadata belongs to another program"); }
 if record.toolchain_version!=VERSION { bail!("build was produced by Tare {}; run tare build with {} before proving",record.toolchain_version,VERSION); }
 for (path,hash) in [(&record.stock,&record.stock_hash),(&record.tare,&record.tare_hash)] { if digest(&fs::read(path)?)!=*hash { bail!("build artifacts changed since tare build"); } }
 let idl:Value=serde_json::from_slice(&fs::read(&record.idl)?)?;
 if address(&idl["address"])?!=program.parse::<Address>()? { bail!("mainnet program does not match the built program"); }
 let corpus=sample_mainnet(&rpc,program,samples,&record.stock_hash)?;write_json(&dir.join("mainnet-corpus.json"),&corpus)?;
 let mut workspace=workspace.clone();workspace.mainnet_corpus=Some(dir.join("mainnet-corpus.json"));measure(&workspace,true)
}
fn sample_mainnet(rpc:&Rpc,program:&str,samples:u32,stock_hash:&str)->Result<tare_prove::RecordedCorpus>{
 let program_key:Address=program.parse()?;
 let signatures=rpc.call("getSignaturesForAddress",json!([program,{"limit":samples,"commitment":"confirmed"}]))?;
 let mut sequences=Vec::new();let mut external=std::collections::BTreeMap::new();
 for row in signatures.as_array().context("signature rows")?{
  let signature=row["signature"].as_str().context("signature")?;
  let tx=rpc.call("getTransaction",json!([signature,{"encoding":"base64","maxSupportedTransactionVersion":1,"commitment":"confirmed"}]))?;
  if tx.is_null(){continue}
  let encoded=tx["transaction"][0].as_str().context("base64 transaction")?;
  let raw:VersionedTransaction=bincode::deserialize(&BASE64.decode(encoded)?)?;
  let mut keys:Vec<String>=raw.message.static_account_keys().iter().map(ToString::to_string).collect();
  for kind in ["writable","readonly"]{if let Some(values)=tx["meta"]["loadedAddresses"][kind].as_array(){for value in values{keys.push(value.as_str().context("loaded address")?.into())}}}
  let direct=raw.message.instructions().iter().any(|ix|keys.get(ix.program_id_index as usize).map(String::as_str)==Some(program));
  let inner=tx["meta"]["innerInstructions"].as_array().into_iter().flatten().flat_map(|group|group["instructions"].as_array().into_iter().flatten()).any(|ix|ix["programIdIndex"].as_u64().and_then(|i|keys.get(i as usize)).map(String::as_str)==Some(program));
  if !direct&&!inner {continue}
  if let Some(lookups)=raw.message.address_table_lookups(){for lookup in lookups {let key=lookup.account_key.to_string();if !keys.contains(&key){keys.push(key)}}}
  for sysvar in ["SysvarC1ock11111111111111111111111111111111","SysvarS1otHashes111111111111111111111111111"] {let key=sysvar.to_owned();if !keys.contains(&key){keys.push(key)}}
  let mut values=Vec::new();let mut slot=0;
  for group in keys.chunks(100){let snapshot=rpc.call("getMultipleAccounts",json!([group,{"encoding":"base64","commitment":"confirmed","minContextSlot":slot}]))?;slot=wire::integer(&snapshot["context"]["slot"]).context("snapshot slot")?;values.extend(snapshot["value"].as_array().context("accounts")?.iter().cloned());}
  let mut accounts=Vec::new();
  for (key,value) in keys.iter().zip(&values){
   let data=if value.is_null(){None}else{Some(json!({"owner":value["owner"],"lamports":value["lamports"],"executable":value["executable"],"dataBase64":value["data"][0],"rentEpoch":value["rentEpoch"]}))};
   if value["executable"]==true&&key!=program&&value["owner"]!="NativeLoader1111111111111111111111111111111"&&!external.contains_key(key){
    let bytes=BASE64.decode(value["data"][0].as_str().context("program data")?)?;
    let elf=if bytes.starts_with(b"\x7fELF"){bytes}else if bytes.get(..4)==Some(&[2,0,0,0])&&bytes.len()>=36{let pd=Address::try_from(&bytes[4..36]).map_err(|e|anyhow::anyhow!("programdata: {e}"))?;let a=rpc.account(&pd)?;let b=BASE64.decode(a["data"][0].as_str().context("programdata")?)?;b.get(45..).context("short ProgramData")?.to_vec()}else{bail!("unsupported executable format at {key}")};
    external.insert(key.clone(),tare_prove::RecordedProgram{address:key.clone(),sha256:digest(&elf),elf_base64:BASE64.encode(elf)});
   }
   accounts.push(tare_prove::RecordedAccount{address:key.clone(),account:data});
  }
  // Current account snapshots are intentionally not the historical execution bank.
  let frame=tare_prove::RecordedTransaction{label:signature.into(),input:None,stage:"landed".into(),transaction_base64:encoded.into(),signature:Some(signature.into()),pre_slot:slot,slot,pre_accounts:accounts,post_accounts:Vec::new(),clock_exact:false,validator:tare_prove::ValidatorOutcome{error:tx["meta"]["err"].clone(),logs:Vec::new(),return_data:None,compute_units:None}};
  sequences.push(tare_prove::RecordedSequence{label:signature.into(),genesis_hash:String::new(),transactions:vec![frame]});
 }
 if sequences.is_empty(){bail!("no replayable transactions found in the bounded sample")}
 let _=program_key;
 Ok(tare_prove::RecordedCorpus{schema_version:"1".into(),kind:"mainnet".into(),target_program:program.into(),stock_build_hash:stock_hash.into(),capture_errors:Vec::new(),source_commit:String::new(),programs:external.into_values().collect(),sequences})
}