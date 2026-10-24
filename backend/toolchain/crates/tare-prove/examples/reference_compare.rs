use anyhow::{bail, Result};
use sha2::{Digest, Sha256};
use std::{env, fs, path::PathBuf};
fn main() -> Result<()> {
    let a: Vec<_> = env::args().skip(1).collect();
    if a.len() != 6 {
        bail!("reference_compare STOCK TARE HANDWRITTEN IDL SOURCE OUTPUT_DIR")
    }
    let idl: serde_json::Value = serde_json::from_slice(&fs::read(&a[3])?)?;
    let hints = tare_prove::source_hints_from_path(&a[4])?;
    let mut corpus = tare_prove::generate_corpus_with_hints(&idl, &hints, 0x54415245)?;
    // These handwritten references implement the user instructions only. The full
    // Tare proof still exercises built-in IDL lifecycle independently.
    for sequence in &mut corpus.sequences {
        sequence
            .cases
            .retain(|case| !case.label.starts_with("idl:"));
    }
    if idl["metadata"]["name"] == "basic_4" {
        use base64::Engine;
        let program: solana_address::Address = idl["address"].as_str().unwrap().parse()?;
        let (_, bump) = solana_address::Address::find_program_address(&[b"counter"], &program);
        for sequence in &mut corpus.sequences {
            if let Some(index) = sequence.cases.iter().position(|case| case.label.starts_with("increment:seed-")) {
                let mut overflow = sequence.cases[index].clone();
                overflow.label = "increment:overflow".into();
                overflow.commit = false;
                let instruction = &overflow.instructions[0];
                let authority: solana_address::Address = instruction.accounts[1].address.parse()?;
                let mut data = Sha256::digest(b"account:Counter")[..8].to_vec();
                data.extend_from_slice(authority.as_ref());
                data.extend_from_slice(&u64::MAX.to_le_bytes());
                data.push(bump);
                overflow.mutations.push(tare_prove::AccountMutation::Data { address: instruction.accounts[0].address.clone(), data_base64: base64::engine::general_purpose::STANDARD.encode(data) });
                let mut unauthorized = sequence.cases[index].clone();
                unauthorized.label = "increment:unauthorized".into();
                unauthorized.commit = false;
                unauthorized.instructions[0].accounts[1].address = sequence.payer.clone();
                sequence.cases.insert(index, unauthorized);
                sequence.cases.insert(index, overflow);
            }
        }
    }
    let out = PathBuf::from(&a[5]);
    fs::create_dir_all(&out)?;
    fs::write(
        out.join("reference-corpus.json"),
        serde_json::to_vec_pretty(&corpus)?,
    )?;
    let program = idl["address"].as_str().unwrap();
    let stock = fs::read(&a[0])?;
    let mut artifacts = Vec::new();
    for (name, path) in [("stock", &a[0]), ("tare", &a[1]), ("handwritten", &a[2])] {
        let bytes = fs::read(path)?;
        fs::create_dir_all(out.join("artifacts"))?;
        fs::write(out.join("artifacts").join(format!("{name}.so")), &bytes)?;
        artifacts.push(serde_json::json!({"name":name,"sourcePath":path,"sha256":hex::encode(Sha256::digest(&bytes)),"sizeBytes":bytes.len()}));
    }
    fs::write(
        out.join("artifacts.json"),
        serde_json::to_vec_pretty(&artifacts)?,
    )?;
    let mut all_match = true;
    for (name, path) in [("tare", &a[1]), ("handwritten", &a[2])] {
        let _ = path;
        let bytes = fs::read(out.join("artifacts").join(format!("{name}.so")))?;
        let (report, traces) = tare_prove::run_traced(program, &stock, &bytes, &corpus)?;
        all_match &= report.matched();
        fs::write(
            out.join(format!("{name}-report.json")),
            serde_json::to_vec_pretty(&report)?,
        )?;
        fs::write(
            out.join(format!("{name}-traces.json")),
            serde_json::to_vec_pretty(&traces)?,
        )?;
        println!(
            "{name}: matched={}, inputs={}, measurements={}",
            report.matched(),
            report.comparisons.inputs,
            serde_json::to_string(&report.instructions)?
        );
    }
    if !all_match {
        std::process::exit(1)
    }
    Ok(())
}
