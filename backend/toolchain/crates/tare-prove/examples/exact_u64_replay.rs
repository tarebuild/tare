use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use litesvm::LiteSVM;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use solana_account::Account;
use solana_address::Address;
use solana_instruction::{account_meta::AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::{versioned::VersionedTransaction, Transaction};
use tare_prove::{recorded::*, AccountState, json_u64};

fn snapshots(svm: &LiteSVM, keys: &[Address]) -> Vec<RecordedAccount> {
    keys.iter().map(|key|RecordedAccount {
        address:key.to_string(),
        account:svm.get_account(key).map(|a|json!({"owner":a.owner.to_string(),"lamports":a.lamports,"executable":a.executable,"dataBase64":BASE64.encode(a.data),"rentEpoch":a.rent_epoch})),
    }).collect()
}
fn main() -> Result<()> {
    let args:Vec<_>=std::env::args().skip(1).collect();
    anyhow::ensure!(args.len()==3 || args.len()==4,"exact_u64_replay ELF IDL REPORT.json [REPLAY_INPUT.json]");
    let elf=std::fs::read(&args[0])?;
    let idl:Value=serde_json::from_slice(&std::fs::read(&args[1])?)?;
    let program:Address=idl["address"].as_str().context("IDL address")?.parse()?;
    let payer=Keypair::new_from_array([77;32]);
    let untouched=Address::from([78;32]);
    let state=AccountState{address:untouched.to_string(),owner:Address::default().to_string(),lamports:1_000_000,executable:false,data_base64:BASE64.encode([]),rent_epoch:u64::MAX};
    let state_json=serde_json::to_value(&state)?;
    assert_eq!(state_json["rentEpoch"],"18446744073709551615");
    let decoded:AccountState=serde_json::from_value(state_json)?;
    assert_eq!(decoded.rent_epoch,u64::MAX);
    let large_lamports=AccountState{lamports:u64::MAX,..decoded.clone()};
    assert_eq!(serde_json::from_value::<AccountState>(serde_json::to_value(&large_lamports)?)?.lamports,u64::MAX);
    for bad in [json!("18446744073709551616"),json!(-1),json!(1.25)] {
        let mut value=serde_json::to_value(&state)?;value["rentEpoch"]=bad;
        assert!(serde_json::from_value::<AccountState>(value).is_err());
    }
    let mut svm=LiteSVM::new().with_blockhash_check(false).with_transaction_history(0);
    svm.set_account(payer.pubkey(),Account{lamports:1_000_000_000,owner:Address::default(),executable:false,data:vec![],rent_epoch:u64::MAX})?;
    svm.set_account(untouched,Account{lamports:decoded.lamports,owner:decoded.owner.parse()?,executable:decoded.executable,data:BASE64.decode(&decoded.data_base64)?,rent_epoch:decoded.rent_epoch})?;
    svm.add_program(program,&elf)?;
    let tx:VersionedTransaction=Transaction::new(&[&payer],Message::new(&[Instruction{program_id:program,accounts:vec![AccountMeta::new_readonly(untouched,false)],data:b"TAREBAD!".to_vec()}],Some(&payer.pubkey())),svm.latest_blockhash()).into();
    let keys=[payer.pubkey(),untouched];let pre=snapshots(&svm,&keys);
    let (meta,error)=match svm.send_transaction(tx.clone()){Ok(meta)=>(meta,Value::Null),Err(e)=>(e.meta,serde_json::to_value(e.err)?)};
    let frame=RecordedTransaction{label:"max-rent-epoch".into(),input:None,stage:"landed".into(),transaction_base64:BASE64.encode(bincode::serialize(&tx)?),signature:None,pre_slot:0,slot:0,pre_accounts:pre,post_accounts:snapshots(&svm,&keys),clock_exact:true,validator:ValidatorOutcome{error,logs:meta.logs,return_data:None,compute_units:Some(meta.compute_units_consumed)}};
    let mut corpus=RecordedCorpus{schema_version:"1".into(),kind:"tests".into(),target_program:program.to_string(),stock_build_hash:hex::encode(Sha256::digest(&elf)),capture_errors:vec![],source_commit:String::new(),programs:vec![],sequences:vec![RecordedSequence{label:"exact-u64".into(),genesis_hash:"native".into(),transactions:vec![frame]}]};
    corpus.sequences[0].transactions[0].input=Some(corpus.sequences[0].transactions[0].replay_input());
    let replay_input=corpus.sequences[0].transactions[0].replay_input();
    assert_eq!(replay_input["accounts"][1]["rentEpoch"], "18446744073709551615");
    if let Some(path)=args.get(3) { std::fs::write(path,serde_json::to_vec_pretty(&replay_input)?)?; }
    let wire=serde_json::to_vec(&corpus)?;
    let corpus:RecordedCorpus=serde_json::from_slice(&wire)?;
    assert_eq!(json_u64::integer(&corpus.sequences[0].transactions[0].pre_accounts[1].account.as_ref().unwrap()["rentEpoch"]),Some(u64::MAX));
    let result=run_recorded(&elf,&elf,&corpus)?;
    assert!(result.differential.matched());assert_eq!(result.validator_reproduction.matched,1);
    // Legacy projections omit rentEpoch but retain the exact rich pre-state.
    let mut legacy=corpus.clone();
    let input=legacy.sequences[0].transactions[0].input.as_mut().unwrap();
    input["slot"]=json!("0");
    for account in input["accounts"].as_array_mut().unwrap() {
        account.as_object_mut().unwrap().remove("rentEpoch");
        account["lamports"]=json!(json_u64::integer(&account["lamports"]).unwrap().to_string());
    }
    assert!(run_recorded(&elf,&elf,&legacy)?.differential.matched());
    let mut incorrect=corpus.clone();
    incorrect.sequences[0].transactions[0].input.as_mut().unwrap()["accounts"][1]["rentEpoch"]=json!(0);
    assert!(run_recorded(&elf,&elf,&incorrect).is_err(),"mismatched explicit rentEpoch must be rejected");
    let mut wrong=corpus.clone();wrong.sequences[0].transactions[0].post_accounts[1].account.as_mut().unwrap()["rentEpoch"]=json!(0);
    let mismatch=run_recorded(&elf,&elf,&wrong)?;
    assert_eq!(mismatch.validator_reproduction.failed,1);
    let evidence=serde_json::to_value(&mismatch)?;
    assert!(serde_json::to_string(&evidence)?.contains("18446744073709551615"));
    // A real v0 transaction loads the readonly application account through an
    // address table; its program ID stays static, as Agave requires.
    let table=Address::from([79;32]);
    let mut table_data=vec![0;56];
    table_data[..4].copy_from_slice(&1u32.to_le_bytes());
    table_data[4..12].copy_from_slice(&u64::MAX.to_le_bytes());
    table_data[12..20].copy_from_slice(&0u64.to_le_bytes());
    table_data.extend_from_slice(untouched.as_ref());
    let table_account=Account{lamports:10_000_000,owner:"AddressLookupTab1e1111111111111111111111111".parse()?,executable:false,data:table_data.clone(),rent_epoch:u64::MAX};
    svm.set_account(table,table_account)?;
    // last_extended_slot is zero, so activate the address by moving the bank on.
    svm.warp_to_slot(1);
    let message=solana_message::v0::Message::try_compile(&payer.pubkey(),&[Instruction{program_id:program,accounts:vec![AccountMeta::new_readonly(untouched,false)],data:b"TAREBAD!".to_vec()}],&[solana_message::AddressLookupTableAccount{key:table,addresses:vec![untouched]}],svm.latest_blockhash())?;
    assert!(!message.address_table_lookups.is_empty());
    let v0_tx=VersionedTransaction::try_new(solana_message::VersionedMessage::V0(message),&[&payer])?;
    let mut v0_corpus=corpus.clone();
    let v0_frame=&mut v0_corpus.sequences[0].transactions[0];
    v0_frame.label="v0-loaded-account".into();v0_frame.transaction_base64=BASE64.encode(bincode::serialize(&v0_tx)?);v0_frame.input=None;v0_frame.pre_slot=1;v0_frame.slot=1;
    v0_frame.pre_accounts.extend(snapshots(&svm,&[table,"SysvarC1ock11111111111111111111111111111111".parse()?]));
    let v0_result=run_recorded(&elf,&elf,&v0_corpus)?;
    assert!(v0_result.differential.matched());
    assert!(v0_result.differential.outcomes[0]["instructionGross"].as_u64().is_some(),"v0 loaded-account input was not metered");
    let mut slots=corpus.sequences[0].transactions[0].clone();slots.pre_slot=u64::MAX;slots.slot=u64::MAX;slots.validator.compute_units=Some(u64::MAX);
    let value=serde_json::to_value(&slots)?;assert_eq!(value["preSlot"],"18446744073709551615");assert_eq!(value["slot"],"18446744073709551615");assert_eq!(value["validator"]["computeUnits"],"18446744073709551615");
    let slots:RecordedTransaction=serde_json::from_value(value)?;assert_eq!(slots.pre_slot,u64::MAX);assert_eq!(slots.validator.compute_units,Some(u64::MAX));
    std::fs::write(&args[2],serde_json::to_vec_pretty(&json!({"rentEpoch":json_u64::value(u64::MAX),"replay":result,"mismatchEvidence":mismatch.validator_reproduction,"maxSlotRoundtrip":slots,"replay_input":replay_input,"v0LoadedAccount":v0_result.differential}))?)?;
    println!("Exact u64 JSON and signed replay passed: max rent epoch preserved, mismatch retained, max slots/compute/lamports roundtrip.");
    Ok(())
}
