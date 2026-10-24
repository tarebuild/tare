//! Execute one synthetic transaction-v1 input through the production recorder replay.
use anyhow::{ensure,Result};
use base64::{engine::general_purpose::STANDARD as B64,Engine};
use serde_json::json;
use sha2::{Digest,Sha256};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_message::{VersionedMessage,v1};
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;
use std::{env,fs};
fn main()->Result<()> {
 let a:Vec<String>=env::args().skip(1).collect();ensure!(a.len()==4,"v1_replay STOCK TARE IDL OUTPUT");
 let stock=fs::read(&a[0])?;let tare=fs::read(&a[1])?;let idl:serde_json::Value=serde_json::from_slice(&fs::read(&a[2])?)?;
 let program=idl["address"].as_str().unwrap().parse()?;let signer=Keypair::new_from_array([21;32]);
 let ix=Instruction{program_id:program,accounts:vec![],data:idl["instructions"][0]["discriminator"].as_array().unwrap().iter().map(|n|n.as_u64().unwrap() as u8).collect()};
 let message=v1::Message::try_compile_with_config(&signer.pubkey(),&[ix],Default::default(),v1::TransactionConfig::empty().with_compute_unit_limit(200_000).with_loaded_accounts_data_size_limit(2 * 1024 * 1024))?;
 let tx=VersionedTransaction::try_new(VersionedMessage::V1(message),&[&signer])?;
 let frame=tare_prove::RecordedTransaction{label:"synthetic-v1-initialize".into(),input:None,stage:"landed".into(),transaction_base64:B64.encode(bincode::serialize(&tx)?),signature:Some(tx.signatures[0].to_string()),pre_slot:42,slot:42,pre_accounts:vec![tare_prove::RecordedAccount{address:signer.pubkey().to_string(),account:Some(json!({"owner":"11111111111111111111111111111111","lamports":5000000000u64,"executable":false,"dataBase64":"","rentEpoch":0}))}],post_accounts:vec![],clock_exact:false,validator:tare_prove::ValidatorOutcome{error:serde_json::Value::Null,logs:vec![],return_data:None,compute_units:None}};
 let corpus=tare_prove::RecordedCorpus{schema_version:"1".into(),kind:"mainnet".into(),target_program:program.to_string(),stock_build_hash:hex::encode(Sha256::digest(&stock)),capture_errors:vec![],source_commit:String::new(),programs:vec![],sequences:vec![tare_prove::RecordedSequence{label:"synthetic-v1".into(),genesis_hash:String::new(),transactions:vec![frame]}]};
 let run=tare_prove::run_recorded(&stock,&tare,&corpus)?;
 fs::write(&a[3],serde_json::to_vec_pretty(&json!({"kind":"synthetic-transaction-v1","sampledFromMainnet":false,"corpus":corpus,"run":run.differential}))?)?;
 ensure!(run.differential.matched(),"v1 input diverged");
 ensure!(run.differential.outcomes[0]["success"]==true,"v1 input did not execute successfully: {}",run.differential.outcomes[0]["grossError"]);
 println!("synthetic transaction v1: successful, stock/Tare matched");Ok(())
}
