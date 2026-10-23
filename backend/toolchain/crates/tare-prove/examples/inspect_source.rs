//! Inspect the source-only facts used by the corpus generator.
use anyhow::{Context,Result};
fn main()->Result<()>{let path=std::env::args().nth(1).context("inspect_source SOURCE_RS")?;let hints=tare_prove::source_hints_from_path(path)?;println!("{}",serde_json::to_string_pretty(&hints)?);Ok(())}
