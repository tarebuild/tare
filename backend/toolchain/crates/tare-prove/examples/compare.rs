//! Direct differential runner for diagnosing a pair of already-built ELFs.
use anyhow::{bail, Context, Result};
use std::{env, fs};
fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 4 && args.len() != 5 {
        bail!("usage: compare <stock.so> <tare.so> <stock-idl.json> <source.rs> [anchor-version]");
    }
    let idl: serde_json::Value = serde_json::from_slice(&fs::read(&args[2])?)?;
    let hints = tare_prove::source_hints_from_path(&args[3])?;
    let corpus = tare_prove::generate_corpus_for_version(
        &idl,
        &hints,
        0x54415245,
        args.get(4).map(String::as_str).unwrap_or("0.31.1"),
    )?;
    let program = idl["address"].as_str().context("IDL program address")?;
    let report = tare_prove::run(program, &fs::read(&args[0])?, &fs::read(&args[1])?, &corpus)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if !report.matched() {
        std::process::exit(1);
    }
    Ok(())
}
