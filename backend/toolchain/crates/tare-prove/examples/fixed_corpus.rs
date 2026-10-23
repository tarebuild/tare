//! Compare two builds on an unchanged, stored corpus and pinned external programs.
use anyhow::{bail, ensure, Context, Result};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
struct ExternalProgram {
    address: String,
    path: PathBuf,
    sha256: String,
}
fn hash(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
fn main() -> Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() != 6 {
        bail!("usage: fixed_corpus ADDRESS BEFORE.so AFTER.so CORPUS.json PROGRAMS.json OUT.json");
    }
    let before = fs::read(&args[1]).with_context(|| format!("read {}", args[1]))?;
    let after = fs::read(&args[2]).with_context(|| format!("read {}", args[2]))?;
    let corpus_bytes = fs::read(&args[3]).with_context(|| format!("read {}", args[3]))?;
    let corpus: tare_prove::Corpus =
        serde_json::from_slice(&corpus_bytes).context("read stored Corpus")?;
    let list_path = Path::new(&args[4]);
    let programs: Vec<ExternalProgram> =
        serde_json::from_slice(&fs::read(list_path)?).context("read external program manifest")?;
    let mut external = Vec::new();
    let mut provenance = Vec::new();
    for program in programs {
        let path = if program.path.is_absolute() {
            program.path
        } else {
            list_path
                .parent()
                .unwrap_or(Path::new("."))
                .join(program.path)
        };
        let bytes = fs::read(&path).with_context(|| format!("read external {}", path.display()))?;
        let actual = hash(&bytes);
        ensure!(
            actual == program.sha256,
            "external {} hash mismatch: expected {}, got {}",
            program.address,
            program.sha256,
            actual
        );
        provenance.push(json!({"address":program.address,"path":path,"sha256":actual}));
        external.push((program.address, bytes));
    }
    let report = tare_prove::run_with_programs(&args[0], &before, &after, &corpus, &external)?;
    let regressions: Vec<_> = report.instructions.iter().filter_map(|row| match (row.gross, row.net) {
        (Some(before),Some(after)) if row.inputs > 0 && after > before => Some(json!({"instruction":row.name,"discriminator":row.discriminator,"inputs":row.inputs,"before":before,"after":after,"increase":after-before})),
        _ => None,
    }).collect();
    let mut output = serde_json::to_value(&report)?;
    output["fixedCorpus"] = json!({"address":args[0],"beforeSha256":hash(&before),"afterSha256":hash(&after),"corpusFileSha256":hash(&corpus_bytes),"externalPrograms":provenance,"medianRegressions":regressions});
    let out_path = Path::new(&args[5]);
    if let Some(parent) = out_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(out_path, serde_json::to_vec_pretty(&output)?)?;
    println!(
        "matched={}, inputs={}, median CU regressions={}",
        report.matched(),
        report.comparisons.inputs,
        regressions.len()
    );
    if let Some(divergence) = &report.first_divergence {
        eprintln!(
            "first divergence: {}",
            serde_json::to_string_pretty(divergence)?
        );
    }
    for regression in &regressions {
        eprintln!("median CU regression: {regression}");
    }
    if !report.matched() || !regressions.is_empty() {
        std::process::exit(1);
    }
    Ok(())
}
