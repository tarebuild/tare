//! Diagnose newly appended source-resolved scenarios; full proofs remain separate.
use anyhow::{bail,Result};use std::{env,fs};
fn main()->Result<()>{let a:Vec<_>=env::args().skip(1).collect();if a.len()!=8{bail!("scenarios STOCK TARE IDL SOURCE VERSION PREFIX METHODS PAIR")}
 let idl:serde_json::Value=serde_json::from_slice(&fs::read(&a[2])?)?;let hints=tare_prove::source_hints_from_path(&a[3])?;let mut corpus=tare_prove::generate_corpus_for_version(&idl,&hints,0x54415245,&a[4])?;
 fs::write(format!("{}-full-corpus.json",a[5]),serde_json::to_vec_pretty(&corpus)?)?;
 corpus.sequences.retain(|s|s.label.contains("-resolved-") && a[6].split(',').any(|method|s.label.ends_with(&format!("-resolved-{method}"))));
 fs::write(format!("{}-scenarios.json",a[5]),serde_json::to_vec_pretty(&corpus)?)?;
 let stock=fs::read(&a[0])?;let net=if a[7]=="stock" {stock.clone()} else {fs::read(&a[1])?};let r=tare_prove::run(idl["address"].as_str().unwrap(),&stock,&net,&corpus)?;fs::write(format!("{}-report.json",a[5]),serde_json::to_vec_pretty(&r)?)?;println!("matched={} inputs={}",r.matched(),r.comparisons.inputs);if !r.matched(){std::process::exit(1)}Ok(())}
