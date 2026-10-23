//! Trace an explicitly selected stored corpus without regenerating inputs.
use anyhow::{bail,Result};
use std::{env,fs};
fn main()->Result<()>{
 let a:Vec<_>=env::args().skip(1).collect();if a.len()!=5{bail!("trace_corpus ADDRESS BEFORE AFTER CORPUS OUTPUT_PREFIX")}
 let corpus:tare_prove::Corpus=serde_json::from_slice(&fs::read(&a[3])?)?;
 let (report,traces)=tare_prove::run_traced(&a[0],&fs::read(&a[1])?,&fs::read(&a[2])?,&corpus)?;
 fs::write(format!("{}-report.json",a[4]),serde_json::to_vec_pretty(&report)?)?;
 fs::write(format!("{}-traces.json",a[4]),serde_json::to_vec_pretty(&traces)?)?;
 println!("matched={}, inputs={}",report.matched(),report.comparisons.inputs);if !report.matched(){std::process::exit(1)}Ok(())
}
