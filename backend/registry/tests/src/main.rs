use anyhow::{ensure, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use litesvm::{LiteSVM, types::TransactionMetadata};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use solana_account::Account;
use solana_address::Address;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;
use std::{collections::BTreeSet, path::{Path,PathBuf}};

fn disc(namespace: &str, name: &str) -> Vec<u8> { Sha256::digest(format!("{namespace}:{name}")).as_slice()[..8].to_vec() }
fn bytes(out: &mut Vec<u8>, value: &[u8]) { out.extend((value.len() as u32).to_le_bytes()); out.extend(value); }
fn string(out: &mut Vec<u8>, value: &str) { bytes(out, value.as_bytes()); }
fn key(seed: u8) -> Keypair { Keypair::new_from_array([seed; 32]) }
fn pda(program: &Address, seeds: &[&[u8]]) -> Address { Address::find_program_address(seeds,program).0 }
fn writable(key: Address, signer: bool) -> AccountMeta { AccountMeta::new(key,signer) }
fn readonly(key: Address, signer: bool) -> AccountMeta { AccountMeta::new_readonly(key,signer) }
fn snapshot(svm: &LiteSVM, address: &Address) -> Value { svm.get_account(address).map(|a| json!({"lamports":a.lamports,"owner":a.owner.to_string(),"dataBase64":B64.encode(a.data),"executable":a.executable,"rentEpoch":a.rent_epoch})).unwrap_or(Value::Null) }
fn logs(meta: &TransactionMetadata) -> Vec<String> { meta.logs.iter().filter(|line| !(line.starts_with("Program ") && line.contains(" consumed ") && line.ends_with(" compute units"))).cloned().collect() }
fn events(meta: &TransactionMetadata, message: &Message, program: Address) -> Vec<Vec<u8>> {
    meta.inner_instructions.iter().flatten().filter(|ix| message.account_keys[ix.instruction.program_id_index as usize] == program && ix.instruction.data.starts_with(&0x1d9acb512ea545e4u64.to_le_bytes())).map(|ix|ix.instruction.data[8..].to_vec()).collect()
}
#[derive(Clone)]
struct Claim { target: Address, report: [u8;32], commit: Vec<u8>, location: String }
impl Claim {
 fn encode(&self, event: bool) -> Vec<u8> {
    let mut d=Vec::new();d.extend(self.target.to_bytes());d.extend([1;32]);d.extend([2;32]);string(&mut d,"https://github.com/example/program");
    if event {string(&mut d,&hex::encode(&self.commit));} else {bytes(&mut d,&self.commit);}
    string(&mut d,"0.3.0");d.extend([3;32]);for v in [7u64,11,13,1000,450] {d.extend(v.to_le_bytes());}d.extend(self.report);string(&mut d,&self.location);d
 }
 fn address(&self, program: &Address, attester: &Address)->Address {pda(program,&[b"record",self.target.as_ref(),&self.report,attester.as_ref()])}
 fn record_data(&self,attester:Address)->Vec<u8>{let mut d=disc("account","Record");d.extend(self.encode(false));d.extend(attester.to_bytes());d.extend(42u64.to_le_bytes());d}
 fn event_data(&self,attester:Address,recipient:Address,fee:u64)->Vec<u8>{let mut d=disc("event","RecordAttested");d.extend(self.encode(true));d.extend(attester.to_bytes());d.extend(42u64.to_le_bytes());d.extend(recipient.to_bytes());d.extend(fee.to_le_bytes());d}
}
struct Pair { stock: LiteSVM, tare: LiteSVM, payer: Keypair, program: Address, programdata: Address, watched: BTreeSet<Address>, rows: Vec<Value>, frames: Vec<Value>, output: PathBuf }
impl Pair {
 fn watch(&mut self, keys: impl IntoIterator<Item=Address>) { self.watched.extend(keys); }
 fn send(&mut self,label:&str,accounts:Vec<AccountMeta>,data:Vec<u8>,signer:&Keypair,expected:bool,event:Option<Vec<u8>>)->Result<()> {
    let program=self.program; self.watch(accounts.iter().map(|a|a.pubkey).filter(|k|*k!=program));
    let ix=Instruction{program_id:self.program,accounts,data};let message=Message::new(&[ix],Some(&self.payer.pubkey()));
    let tx=Transaction::new(&[&self.payer,signer],message.clone(),self.stock.latest_blockhash());
    let captured:Vec<_>=self.watched.iter().filter(|a|**a!=self.program).copied().collect();
    let recorded_account=|vm:&LiteSVM,address:&Address|{
        let mut value=snapshot(vm,address);
        if address==&self.programdata {let data=B64.decode(value["dataBase64"].as_str().unwrap()).unwrap();value["dataBase64"]=json!(B64.encode(&data[..45]));}
        json!({"address":address.to_string(),"account":value})
    };
    let pre:Vec<_>=captured.iter().map(|a|recorded_account(&self.stock,a)).collect();
    let wire:solana_transaction::versioned::VersionedTransaction=tx.clone().into();
    let raw=B64.encode(bincode::serialize(&wire)?);
    let (sm,se,error)=match self.stock.send_transaction(tx.clone()){Ok(m)=>(m,None,Value::Null),Err(e)=>{let error=serde_json::to_value(&e.err)?;(e.meta,Some(format!("{:?}",e.err)),error)}};
    let post:Vec<_>=captured.iter().map(|a|recorded_account(&self.stock,a)).collect();
    let accounts:Vec<_>=pre.iter().filter_map(|e|{let a=&e["account"];(!a.is_null()).then(||json!({"address":e["address"],"owner":a["owner"],"lamports":a["lamports"],"executable":a["executable"],"dataBase64":a["dataBase64"]}))}).collect();
    self.frames.push(json!({"label":label,"input":{"kind":"tests","label":label,"transactionBase64":raw,"accounts":accounts,"slot":42},"stage":"landed","transactionBase64":raw,"signature":null,"preSlot":42,"slot":42,"clockExact":true,"preAccounts":pre,"postAccounts":post,"validator":{"error":error,"logs":sm.logs,"returnData":null,"computeUnits":sm.compute_units_consumed}}));
    let (tm,te)=match self.tare.send_transaction(tx){Ok(m)=>(m,None),Err(e)=>(e.meta,Some(format!("{:?}",e.err)))};
    let mut differences=Vec::new();if se!=te{differences.push("error".to_string());}if logs(&sm)!=logs(&tm){differences.push("logs".to_string());}if sm.return_data!=tm.return_data{differences.push("returnData".to_string());}
    let ev=events(&sm,&message,self.program);if ev!=events(&tm,&message,self.program){differences.push("events".to_string());}
    for address in &self.watched { if address==&self.programdata {continue;}if snapshot(&self.stock,address)!=snapshot(&self.tare,address){differences.push(format!("account:{address}"));}}
    let success=se.is_none();let row=json!({"label":label,"success":success,"stockError":se,"tareError":te,"gross":sm.compute_units_consumed,"net":tm.compute_units_consumed,"differences":differences,"stockLogs":sm.logs,"tareLogs":tm.logs,"events":ev.iter().map(|d|B64.encode(d)).collect::<Vec<_>>()});
    self.rows.push(row.clone());
    if !differences.is_empty() {std::fs::write(self.output.join("native-test-first-divergence.json"),serde_json::to_vec_pretty(&row)?)?;}
    ensure!(differences.is_empty(),"{label}: differential failure: {differences:?}");ensure!(success==expected,"{label}: unexpected result: {se:?}");
    if let Some(expected_event)=event {ensure!(ev==[expected_event],"{label}: complete event fields differ from account values");} else {ensure!(ev.is_empty(),"{label}: failed transaction emitted committed event");}
    Ok(())
 }
 fn same_data(&self,address:Address,prefix:&[u8])->Result<()> {for vm in [&self.stock,&self.tare] {let a=vm.get_account(&address).unwrap();ensure!(a.data.starts_with(prefix),"stored account bytes differ");ensure!(a.data[prefix.len()..].iter().all(|b|*b==0),"unexpected account padding");}Ok(())}
 fn lamports(&self,address:Address)->u64 {self.stock.get_account(&address).map_or(0,|a|a.lamports)}
}
fn config_event(name:&str,admin:Address,recipient:Address,fee:u64)->Vec<u8>{let mut d=disc("event",name);d.extend(admin.to_bytes());d.extend(recipient.to_bytes());d.extend(fee.to_le_bytes());d}
fn main()->Result<()> {
 let source=std::env::var("TARE_REGISTRY_SOURCE").unwrap_or_else(|_|"registry".into());let root=Path::new(&source);let idl:Value=serde_json::from_slice(&std::fs::read(root.join("target/idl/tare_registry.json"))?)?;let program:Address=idl["address"].as_str().unwrap().parse()?;
 let stock=std::fs::read(root.join("target/deploy/tare_registry.so"))?;let tare=std::fs::read(root.join("target/tare/tare_registry.so"))?;
 let payer=key(11);let authority=key(12);let admin=key(13);let attester=key(14);let second=key(15);let flagger=key(16);let launch=key(17);
 let loader:Address="BPFLoaderUpgradeab1e11111111111111111111111".parse()?;let system=Address::default();let config=pda(&program,&[b"config"]);let recipient=launch.pubkey();let event_authority=pda(&program,&[b"__event_authority"]);let programdata=pda(&loader,&[program.as_ref()]);
 let setup=|elf:&[u8]|->Result<LiteSVM>{let mut vm=LiteSVM::new().with_blockhash_check(false).with_transaction_history(0);vm.warp_to_slot(42);vm.add_program(program,elf)?;let mut a=vm.get_account(&programdata).unwrap();a.data[12]=1;a.data[13..45].copy_from_slice(authority.pubkey().as_ref());vm.set_account(programdata,a)?;for k in [&payer,&authority,&admin,&attester,&second,&flagger,&launch]{vm.set_account(k.pubkey(),Account{lamports:100_000_000,data:vec![],owner:system,executable:false,rent_epoch:0})?;}Ok(vm)};
 let mut pair=Pair{stock:setup(&stock)?,tare:setup(&tare)?,payer,program,programdata,watched:[authority.pubkey(),admin.pubkey(),attester.pubkey(),second.pubkey(),flagger.pubkey(),key(11).pubkey(),"SysvarC1ock11111111111111111111111111111111".parse()?].into_iter().collect(),rows:vec![],frames:vec![],output:root.join("target")};
 let tail=||vec![readonly(event_authority,false),readonly(program,false)];
 let init_accounts=|a:Address|{let mut v=vec![writable(a,true),readonly(program,false),readonly(programdata,false),writable(config,false),readonly(system,false)];v.extend(tail());v};
 let mut init=disc("global","init_config");init.extend(second.pubkey().to_bytes());init.extend(recipient.to_bytes());init.extend(1250u64.to_le_bytes());
 pair.send("init-wrong-upgrade-authority",init_accounts(attester.pubkey()),init.clone(),&attester,false,None)?;
 ensure!(pair.lamports(config)==0,"unauthorized init created accounts");
 let start=pair.lamports(recipient);let before=pair.lamports(authority.pubkey());
 pair.send("init-config",init_accounts(authority.pubkey()),init.clone(),&authority,true,Some(config_event("ConfigInitialized",second.pubkey(),recipient,1250)))?;
 ensure!(before-pair.lamports(authority.pubkey())==pair.stock.minimum_balance_for_rent_exemption(80),"init charged unexpected lamports");
 pair.send("init-repeat",init_accounts(authority.pubkey()),init,&authority,false,None)?;
 let set_accounts=|a:Address|{let mut v=vec![readonly(a,true),writable(config,false)];v.extend(tail());v};
 let set_data=|next:Address,fee:u64|{let mut d=disc("global","set_config");d.extend(next.to_bytes());d.extend(recipient.to_bytes());d.extend(fee.to_le_bytes());d};
 pair.send("set-config-wrong-admin",set_accounts(attester.pubkey()),set_data(admin.pubkey(),3500),&attester,false,None)?;
 pair.send("set-config-transfer-admin",set_accounts(second.pubkey()),set_data(admin.pubkey(),3500),&second,true,Some(config_event("ConfigUpdated",admin.pubkey(),recipient,3500)))?;
 pair.send("set-config-old-admin",set_accounts(second.pubkey()),set_data(authority.pubkey(),9),&second,false,None)?;
 pair.send("set-config-current-admin",set_accounts(admin.pubkey()),set_data(admin.pubkey(),3500),&admin,true,Some(config_event("ConfigUpdated",admin.pubkey(),recipient,3500)))?;
 let claim=Claim{target:key(44).pubkey(),report:[7;32],commit:vec![6;20],location:"/v1/reports/local-proof".into()};
 let attest_accounts=|who:Address,record:Address,fee_recipient:Address|{let mut v=vec![writable(who,true),readonly(config,false),writable(fee_recipient,false),writable(record,false),readonly(system,false)];v.extend(tail());v};
 let attest_data=|c:&Claim|{let mut d=disc("global","attest");d.extend(c.encode(false));d};
 let record=claim.address(&program,&attester.pubkey());let before=pair.lamports(attester.pubkey());
 pair.send("attest-first",attest_accounts(attester.pubkey(),record,recipient),attest_data(&claim),&attester,true,Some(claim.event_data(attester.pubkey(),recipient,3500)))?;
 pair.same_data(record,&claim.record_data(attester.pubkey()))?;ensure!(pair.lamports(recipient)==start+3500,"attest fee did not reach the launch wallet");ensure!(before-pair.lamports(attester.pubkey())==3500+pair.stock.minimum_balance_for_rent_exemption(1128),"attester did not pay exact rent plus fee");
 pair.send("attest-exact-repeat",attest_accounts(attester.pubkey(),record,recipient),attest_data(&claim),&attester,false,None)?;ensure!(pair.lamports(recipient)==start+3500,"failed duplicate charged fee");
 let mut next=claim.clone();next.report=[8;32];next.commit=vec![9;32];let record2=next.address(&program,&attester.pubkey());
 pair.send("attest-new-report",attest_accounts(attester.pubkey(),record2,recipient),attest_data(&next),&attester,true,Some(next.event_data(attester.pubkey(),recipient,3500)))?;pair.same_data(record2,&next.record_data(attester.pubkey()))?;
 let record3=claim.address(&program,&second.pubkey());pair.send("attest-second-attester",attest_accounts(second.pubkey(),record3,recipient),attest_data(&claim),&second,true,Some(claim.event_data(second.pubkey(),recipient,3500)))?;ensure!(pair.lamports(recipient)==start+10500,"fees did not accumulate");
 let mut bad=claim.clone();bad.report=[10;32];bad.commit=vec![1;3];let badrecord=bad.address(&program,&attester.pubkey());pair.send("attest-invalid-commit",attest_accounts(attester.pubkey(),badrecord,recipient),attest_data(&bad),&attester,false,None)?;ensure!(pair.lamports(badrecord)==0 && pair.lamports(recipient)==start+10500,"invalid attest changed funds");
 let wrongrecord=key(66).pubkey();pair.send("attest-wrong-record-seeds",attest_accounts(attester.pubkey(),wrongrecord,recipient),attest_data(&next),&attester,false,None)?;
 pair.send("attest-wrong-recipient",attest_accounts(attester.pubkey(),badrecord,second.pubkey()),attest_data(&bad),&attester,false,None)?;ensure!(pair.lamports(recipient)==start+10500,"refused attest changed funds");
 let hash=[20u8;32];let flag=pda(&program,&[b"flag",record.as_ref(),&hash]);let location="/v1/flag-inputs/local-input";
 let flag_accounts=|rec:Address,fl:Address|{let mut v=vec![writable(flagger.pubkey(),true),readonly(rec,false),writable(fl,false),readonly(system,false)];v.extend(tail());v};
 let mut flag_data=disc("global","flag");flag_data.extend(hash);string(&mut flag_data,location);
 let mut flag_state=disc("account","Flag");flag_state.extend(record.to_bytes());flag_state.extend(hash);string(&mut flag_state,location);flag_state.extend(flagger.pubkey().to_bytes());flag_state.extend(42u64.to_le_bytes());let mut flag_event=disc("event","RecordFlagged");flag_event.extend_from_slice(&flag_state[8..]);
 pair.send("flag-existing-record",flag_accounts(record,flag),flag_data.clone(),&flagger,true,Some(flag_event))?;pair.same_data(flag,&flag_state)?;
 pair.send("flag-exact-repeat",flag_accounts(record,flag),flag_data.clone(),&flagger,false,None)?;
 let absent=key(67).pubkey();let absentflag=pda(&program,&[b"flag",absent.as_ref(),&hash]);pair.send("flag-nonexistent-record",flag_accounts(absent,absentflag),flag_data,&flagger,false,None)?;ensure!(pair.lamports(absentflag)==0,"missing record created flag");
 let mut config_data=disc("account","Config");config_data.extend(admin.pubkey().to_bytes());config_data.extend(recipient.to_bytes());config_data.extend(3500u64.to_le_bytes());pair.same_data(config,&config_data)?;
 let instructions:Vec<_>=idl["instructions"].as_array().unwrap().iter().map(|ix|json!({"name":ix["name"],"discriminator":hex::encode(ix["discriminator"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap() as u8).collect::<Vec<_>>())})).collect();
 let recorded=json!({"schemaVersion":"1","kind":"tests","targetProgram":program.to_string(),"stockBuildHash":hex::encode(Sha256::digest(&stock)),"captureErrors":[],"sourceCommit":"","programs":[],"captureKind":"native-litesvm","sequences":[{"label":"registry-native-integration","genesisHash":"native-litesvm","transactions":pair.frames}]});
 let replay=json!({"schemaVersion":"1","manifestPath":"programs/tare-registry/Cargo.toml","generated":{"seed":0,"programAliases":[],"executableFixtures":[],"sequences":[],"instructions":instructions},"programs":[],"tests":recorded,"mainnet":null});
 std::fs::write(root.join("target/native-replay.json"),serde_json::to_vec_pretty(&replay)?)?;
 let report=json!({"status":"matched","kind":"native-registry-integration","validatorUsed":false,"stockBuildHash":hex::encode(Sha256::digest(stock)),"tareBuildHash":hex::encode(Sha256::digest(tare)),"transactions":pair.rows.len(),"successful":pair.rows.iter().filter(|r|r["success"]==true).count(),"compared":["account bytes","lamports","logs excluding runtime compute lines","return data","errors","complete self-CPI event payloads"],"feeRecipient":{"address":recipient.to_string(),"attestFeesLamports":10500,"receivedLamports":pair.lamports(recipient)-start},"cases":pair.rows});
 std::fs::write(root.join("target/native-tests.json"),serde_json::to_vec_pretty(&report)?)?;println!("Registry native stock/Tare: {} transactions matched; exact fees, authorization, stored fields and CPI events verified",report["transactions"]);Ok(())
}
