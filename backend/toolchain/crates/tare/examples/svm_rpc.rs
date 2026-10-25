//! Native test-only RPC/report-store bridge. Every transaction executes in LiteSVM.
use anyhow::{bail,Context,Result};
use base64::{engine::general_purpose::STANDARD as B64,Engine};
use litesvm::LiteSVM;
use serde_json::{json,Value};
use sha2::{Digest,Sha256};
use solana_account::Account;
use solana_address::Address;
use solana_instruction::{Instruction,AccountMeta};
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_transaction::{Transaction,versioned::VersionedTransaction};
use std::{fs,io::{Read,Write},net::TcpListener,path::PathBuf,collections::BTreeMap};
fn disc(s:&str)->Vec<u8>{Sha256::digest(s)[..8].to_vec()}
fn main()->Result<()>{
 let a:Vec<String>=std::env::args().skip(1).collect();if a.len()!=5{bail!("svm_rpc ELF PROGRAM KEYPAIR FEE OUTPUT_DIR")}
 let program:Address=a[1].parse()?;let fee:u64=a[3].parse()?;let folder=PathBuf::from(&a[4]);fs::create_dir_all(&folder)?;
 let key=Keypair::try_from(serde_json::from_slice::<Vec<u8>>(&fs::read(&a[2])?)?.as_slice()).map_err(|e|anyhow::anyhow!("keypair: {e}"))?;
 let mut vm=LiteSVM::new().with_blockhash_check(false).with_transaction_history(0);
 vm.warp_to_slot(42);vm.add_program(program,&fs::read(&a[0])?)?;
 vm.set_account(key.pubkey(),Account{lamports:10_000_000_000,data:vec![],owner:Address::default(),executable:false,rent_epoch:0})?;
 let loader:Address="BPFLoaderUpgradeab1e11111111111111111111111".parse()?;let pd=Address::find_program_address(&[program.as_ref()],&loader).0;
 let mut account=vm.get_account(&pd).context("ProgramData")?;account.data[12]=1;account.data[13..45].copy_from_slice(key.pubkey().as_ref());vm.set_account(pd,account)?;
 let pda=|seed:&[u8]|Address::find_program_address(&[seed],&program).0;
 let mut data=disc("global:init_config");data.extend(key.pubkey().as_ref());data.extend(fee.to_le_bytes());
 let ix=Instruction{program_id:program,accounts:vec![AccountMeta::new(key.pubkey(),true),AccountMeta::new_readonly(program,false),AccountMeta::new_readonly(pd,false),AccountMeta::new(pda(b"config"),false),AccountMeta::new(pda(b"fee_vault"),false),AccountMeta::new_readonly(Address::default(),false),AccountMeta::new_readonly(pda(b"__event_authority"),false),AccountMeta::new_readonly(program,false)],data};
 let tx=Transaction::new_signed_with_payer(&[ix],Some(&key.pubkey()),&[&key],vm.latest_blockhash());vm.send_transaction(tx).map_err(|e|anyhow::anyhow!("init: {:?}",e))?;
 let listener=TcpListener::bind("127.0.0.1:7813")?;println!("Native LiteSVM RPC and report store: http://127.0.0.1:7813");std::io::stdout().flush()?;
 let mut signatures=BTreeMap::<String,Value>::new();
 for stream in listener.incoming(){
  let mut stream=stream?;stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
  let mut bytes=Vec::new();let mut buffer=[0;4096];let (head_end,length)=loop{let n=stream.read(&mut buffer)?;if n==0{bail!("short HTTP request")};bytes.extend_from_slice(&buffer[..n]);if bytes.len()>2*1024*1024{bail!("request too large")};if let Some(i)=bytes.windows(4).position(|w|w==b"\r\n\r\n"){let header=String::from_utf8_lossy(&bytes[..i]);let length=header.lines().find_map(|l|l.split_once(':').filter(|(k,_)|k.eq_ignore_ascii_case("content-length")).map(|(_,v)|v.trim().parse::<usize>())).transpose()?.unwrap_or(0);break(i+4,length)}};
  while bytes.len()<head_end+length{let n=stream.read(&mut buffer)?;if n==0{bail!("short HTTP body")};bytes.extend_from_slice(&buffer[..n]);}
  let header=String::from_utf8_lossy(&bytes[..head_end]);let first=header.lines().next().unwrap();let parts:Vec<_>=first.split_whitespace().collect();let method=parts[0];let path=parts[1];
  let mut http=200;let response=if path=="/v1/config" {json!({"siteUrl":"http://127.0.0.1:7813","recordUrlTemplate":"http://127.0.0.1:7813/builds/{record}"})}
  else if method=="POST"&&(path=="/v1/reports"||path=="/v1/flag-inputs"){
   let value:Value=serde_json::from_slice(&bytes[head_end..head_end+length])?;let encoded=serde_json::to_vec(&value)?;let hash=hex::encode(Sha256::digest(&encoded));fs::write(folder.join(format!("{hash}.json")),encoded)?;http=201;json!({"hash":hash,"location":format!("{path}/{hash}")})
  } else if method=="GET"&&path.starts_with("/v1/") {
   let hash=path.rsplit('/').next().unwrap();serde_json::from_slice(&fs::read(folder.join(format!("{hash}.json")))?)?
  } else {
   let request:Value=serde_json::from_slice(&bytes[head_end..head_end+length])?;let p=&request["params"];let mut error=None;
   let result=match request["method"].as_str().unwrap_or(""){
    "getVersion"=>json!({"solana-core":"LiteSVM 0.18.2","feature-set":0}),
    "getSlot"=>json!(42),
    "getLatestBlockhash"=>json!({"context":{"slot":42},"value":{"blockhash":vm.latest_blockhash().to_string(),"lastValidBlockHeight":999999}}),
    "getAccountInfo"=>{let key:Address=p[0].as_str().context("address")?.parse()?;let value=vm.get_account(&key).map(|a|json!({"owner":a.owner.to_string(),"lamports":a.lamports,"executable":a.executable,"rentEpoch":a.rent_epoch,"space":a.data.len(),"data":[B64.encode(a.data),"base64"]}));json!({"context":{"slot":42},"value":value})},
    "getSignatureStatuses"=>json!({"context":{"slot":42},"value":p[0].as_array().unwrap().iter().map(|s|signatures.get(s.as_str().unwrap()).cloned().unwrap_or(Value::Null)).collect::<Vec<_>>()}),
    "sendTransaction"=>{let tx:VersionedTransaction=bincode::deserialize(&B64.decode(p[0].as_str().unwrap())?)?;let signature=tx.signatures[0].to_string();match vm.send_transaction(tx){Ok(meta)=>{signatures.insert(signature.clone(),json!({"slot":42,"confirmations":null,"err":null,"confirmationStatus":"confirmed"}));fs::write(folder.join(format!("{signature}.transaction.json")),serde_json::to_vec_pretty(&json!({"logs":meta.logs,"computeUnits":meta.compute_units_consumed}))?)?;json!(signature)},Err(e)=>{error=Some(json!({"code":-32002,"message":"SVM transaction failed","data":{"err":e.err,"logs":e.meta.logs}}));Value::Null}}},
    _=>{error=Some(json!({"code":-32601,"message":"unsupported test RPC method"}));Value::Null}
   };
   if let Some(error)=error{json!({"jsonrpc":"2.0","id":request["id"],"error":error})}else{json!({"jsonrpc":"2.0","id":request["id"],"result":result})}
  };
  let body=serde_json::to_vec(&response)?;write!(stream,"HTTP/1.1 {http} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len())?;stream.write_all(&body)?;
 }
 Ok(())
}