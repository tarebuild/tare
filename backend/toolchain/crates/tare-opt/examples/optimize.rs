//! Optimize an existing ELF independently of language compilation.
use anyhow::{bail, Result};
use std::{env, fs};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 2 {
        bail!("usage: optimize <input.so> <output.so>");
    }
    let optimized = tare_opt::optimize(&fs::read(&args[0])?)?;
    fs::write(&args[1], &optimized.bytes)?;
    println!("{}", serde_json::to_string_pretty(&optimized.stats)?);
    Ok(())
}
