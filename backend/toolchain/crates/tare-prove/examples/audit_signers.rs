//! Diagnose generated signer identities without changing corpus transactions.
use anyhow::{Context, Result};
use solana_signer::Signer;
use std::collections::BTreeSet;
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let idl: serde_json::Value = serde_json::from_slice(&std::fs::read(
        args.first().context("audit_signers IDL SOURCE [VERSION]")?,
    )?)?;
    let hints = tare_prove::source_hints_from_path(&args[1])?;
    let corpus = tare_prove::generate_corpus_for_version(
        &idl,
        &hints,
        0x54415245,
        args.get(2).map(String::as_str).unwrap_or("0.31.1"),
    )?;
    let mut missing = Vec::new();
    let mut inputs = 0;
    for sequence in &corpus.sequences {
        let keys: BTreeSet<_> = sequence
            .signer_seeds
            .iter()
            .map(|seed| {
                solana_keypair::Keypair::new_from_array(*seed)
                    .pubkey()
                    .to_string()
            })
            .collect();
        for case in &sequence.cases {
            inputs += 1;
            for instruction in &case.instructions {
                for account in &instruction.accounts {
                    if account.signer && !keys.contains(&account.address) {
                        missing.push(serde_json::json!({"sequence":sequence.label,"case":case.label,"account":account.address}));
                    }
                }
            }
        }
    }
    println!(
        "{}",
        serde_json::json!({"inputs":inputs,"missingCount":missing.len(),"firstMissing":missing.iter().take(12).collect::<Vec<_>>()})
    );
    Ok(())
}
