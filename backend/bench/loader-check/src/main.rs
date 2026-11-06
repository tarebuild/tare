use anyhow::{Context,Result};
use std::io::Read;
use solana_program_runtime::{
    deploy::deploy_program,
    execution_budget::SVMTransactionExecutionBudget,
    invoke_context::InvokeContext,
    loaded_programs::{ProgramCacheForTxBatch, ProgramRuntimeEnvironment},
    program_cache_entry::{ProgramCacheEntry,ProgramCacheEntryType},
};
use solana_sbpf::{elf::Executable,verifier::RequisiteVerifier};
use solana_svm_feature_set::SVMFeatureSet;
use solana_pubkey::Pubkey;
fn main()->Result<()> {
 let features=SVMFeatureSet {disable_sbpf_v0_v1_v2_deployment:false,..SVMFeatureSet::all_enabled()};
 let budget=SVMTransactionExecutionBudget::new_with_defaults(features.raise_cpi_nesting_limit_to_8);
 let load_runtime=solana_syscalls::create_program_runtime_environment(&features,&budget,false,false).map_err(|e|anyhow::anyhow!(e.to_string()))?;
 let preload_runtime=solana_syscalls::create_program_runtime_environment(&features,&budget,true,false).map_err(|e|anyhow::anyhow!(e.to_string()))?;
 let mut rows=Vec::new();
 let mut names=std::env::args().skip(1).collect::<Vec<_>>();
 if names==["--stdin"] {
  let mut input=String::new();std::io::stdin().read_to_string(&mut input)?;
  names=input.lines().filter(|s|!s.is_empty()).map(str::to_owned).collect();
 }
 for name in names {
  let bytes=std::fs::read(&name)?;
  let mut cache=ProgramCacheForTxBatch::new(1);
  let deployment_log=solana_svm_log_collector::LogCollector::new_ref();
  let deployed=deploy_program(Some(deployment_log.clone()),&mut cache,ProgramRuntimeEnvironment::clone(&load_runtime),features.disable_sbpf_v0_v1_v2_deployment,&Pubkey::default(),&solana_sdk_ids::bpf_loader_upgradeable::id(),&bytes,0);
  rows.push(serde_json::json!({"file":name,"phase":"deploy","passed":deployed.is_ok(),"error":deployed.err().map(|e|e.to_string()),"logs":deployment_log.borrow().get_recorded_content()}));
  let preload=Executable::<InvokeContext>::from_elf(&bytes,(*preload_runtime).clone()).and_then(|executable|executable.verify::<RequisiteVerifier>());
  rows.push(serde_json::json!({"file":name,"phase":"test-validator-preload","passed":preload.is_ok(),"error":preload.err().map(|e|e.to_string())}));
  let loaded=ProgramCacheEntry::load(&solana_sdk_ids::bpf_loader_upgradeable::id(),ProgramRuntimeEnvironment::clone(&load_runtime),0,&bytes).and_then(|entry| {
   #[cfg(target_arch="x86_64")]
   if let solana_program_runtime::program_cache_entry::ProgramCacheEntryType::Loaded(executable)=&entry.program { executable.jit_compile()?; }
   Ok(std::sync::Arc::new(entry))
  });
  let mut visibility=serde_json::Value::Null;
  if let Ok(entry)=&loaded {
   let key=Pubkey::default();let entry=entry.clone();
   let mut slot0=ProgramCacheForTxBatch::new(0);slot0.replenish(key,entry.clone());
   let mut slot1=ProgramCacheForTxBatch::new(1);slot1.replenish(key,entry.clone());
   let label=|entry:Option<std::sync::Arc<ProgramCacheEntry>>| match entry.map(|x|match &x.program {
    ProgramCacheEntryType::DelayVisibility=>"DelayVisibility",ProgramCacheEntryType::Loaded(_)=>"Loaded",_=>"Other",
   }) {Some(label)=>label,None=>"Missing"};
   visibility=serde_json::json!({"deploymentSlot":0,"effectiveSlot":entry.effective_slot(),"atSlot0":label(slot0.find(&key)),"atSlot1":label(slot1.find(&key))});
  }
  rows.push(serde_json::json!({"file":name,"phase":"program-cache-load","passed":loaded.is_ok(),"error":loaded.err().map(|e|e.to_string()),"jit":cfg!(target_arch="x86_64"),"visibility":visibility}));
 }
 let passed=rows.iter().all(|row|row["passed"]==true);
 println!("{}",serde_json::to_string_pretty(&rows).context("encode results")?);
 if !passed {std::process::exit(1)}
 Ok(())
}
