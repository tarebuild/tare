use anyhow::{bail, Result};
use std::{env, fs};
fn main() -> Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() != 3 {
        bail!("usage: opt_control stock.so stock-idl.json source.rs")
    }
    let stock = fs::read(&args[0])?;
    let optimized = tare_opt::optimize(&stock)?;
    let idl: serde_json::Value = serde_json::from_slice(&fs::read(&args[1])?)?;
    let hints = tare_prove::source_hints(&fs::read_to_string(&args[2])?)?;
    let corpus = tare_prove::generate_corpus_with_hints(&idl, &hints, 0x54415245)?;
    let report = tare_prove::run(
        idl["address"].as_str().unwrap(),
        &stock,
        &optimized.bytes,
        &corpus,
    )?;
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"optimization":optimized.stats,"proof":report.proof()})
        )?
    );
    if !report.matched() {
        bail!("stock bytecode control diverged")
    }
    Ok(())
}
