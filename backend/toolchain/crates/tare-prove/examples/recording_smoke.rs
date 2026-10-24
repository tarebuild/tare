//! Native replay integration check; does not represent an upstream validator recording.
use anyhow::Result;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use litesvm::LiteSVM;
use serde_json::json;
use sha2::{Digest, Sha256};
use solana_account::Account;
use solana_address::Address;
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::{versioned::VersionedTransaction, Transaction};
use tare_prove::recorded::*;
fn accounts(svm: &LiteSVM, keys: &[Address]) -> Vec<RecordedAccount> {
    keys.iter().map(|key|RecordedAccount{address:key.to_string(),account:svm.get_account(key).map(|a|json!({"owner":a.owner.to_string(),"lamports":a.lamports,"dataBase64":B64.encode(a.data),"executable":a.executable,"rentEpoch":a.rent_epoch}))}).collect()
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let elf = std::fs::read(&args[0])?;
    let idl: serde_json::Value = serde_json::from_slice(&std::fs::read(&args[1])?)?;
    let program: Address = idl["address"].as_str().unwrap().parse()?;
    let payer = Keypair::new_from_array([77; 32]);
    let mut svm = LiteSVM::new()
        .with_blockhash_check(false)
        .with_transaction_history(0);
    svm.set_account(
        payer.pubkey(),
        Account {
            lamports: 1_000_000_000,
            data: vec![],
            owner: Address::default(),
            executable: false,
            rent_epoch: 0,
        },
    )?;
    svm.add_program(program, &elf)?;
    let keys = vec![
        payer.pubkey(),
        "SysvarC1ock11111111111111111111111111111111".parse()?,
        "SysvarRent111111111111111111111111111111111".parse()?,
    ];
    let mut frames = Vec::new();
    for n in 0..3u8 {
        let mut data: Vec<u8> = idl["instructions"][0]["discriminator"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_u64().unwrap() as u8)
            .collect();
        if n == 2 {
            data = vec![255; 8];
        } else {
            data.push(n);
        }
        let ix = solana_instruction::Instruction {
            program_id: program,
            accounts: vec![],
            data,
        };
        let tx: VersionedTransaction = Transaction::new(
            &[&payer],
            Message::new(&[ix], Some(&payer.pubkey())),
            svm.latest_blockhash(),
        )
        .into();
        let pre = accounts(&svm, &keys);
        let (meta, error) = match svm.send_transaction(tx.clone()) {
            Ok(meta) => (meta, serde_json::Value::Null),
            Err(failure) => (failure.meta, serde_json::to_value(failure.err)?),
        };
        let post = accounts(&svm, &keys);
        frames.push(RecordedTransaction {
            label: format!("native-smoke-{n}"),
            input: None,
            stage: "landed".into(),
            transaction_base64: B64.encode(bincode::serialize(&tx)?),
            signature: None,
            pre_slot: 0,
            slot: 0,
            clock_exact: true,
            pre_accounts: pre,
            post_accounts: post,
            validator: ValidatorOutcome {
                error,
                logs: meta.logs,
                return_data: None,
                compute_units: Some(meta.compute_units_consumed),
            },
        });
    }
    for frame in &mut frames {
        frame.input = Some(frame.replay_input());
    }
    let corpus = RecordedCorpus {
        schema_version: "1".into(),
        kind: "tests".into(),
        target_program: program.to_string(),
        stock_build_hash: hex::encode(Sha256::digest(&elf)),
        capture_errors: vec![],
        source_commit: String::new(),
        programs: vec![],
        sequences: vec![RecordedSequence {
            label: "native-smoke".into(),
            genesis_hash: "native".into(),
            transactions: frames,
        }],
    };
    let result = run_recorded(&elf, &elf, &corpus)?;
    assert!(result.differential.matched());
    assert_eq!(result.validator_reproduction.matched, 3);
    assert!(result.differential.outcomes[0]["instructionGross"]
        .as_u64()
        .is_some());
    assert_eq!(result.differential.outcomes[2]["success"], false);
    let generated_corpus = tare_prove::generate_corpus(&idl, 0x54415245)?;
    let mut generated = tare_prove::run(&program.to_string(), &elf, &elf, &generated_corpus)?;
    let successful = generated.instructions[0].inputs;
    let metadata = tare_prove::merge_recorded(&mut generated, result)?;
    assert_eq!(generated.counts.tests, 3);
    assert_eq!(generated.instructions[0].inputs, successful + 2);
    assert_eq!(metadata.validator_reproduction.matched, 3);
    println!("Native recorded replay: 3 ordered signed transactions (including failure) reproduced; differential and sample merge matched");
    Ok(())
}
