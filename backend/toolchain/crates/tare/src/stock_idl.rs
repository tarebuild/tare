//! Stock Anchor IDL build for a manifest without its optional idl-build alias.
//! Marker assembly follows anchor-lang-idl 0.1.4; the program source is untouched.
use anyhow::{anyhow,bail,Context,Result};
use serde_json::{json,Value};
use std::{collections::BTreeMap,env,path::Path,process::{Command,Stdio}};

pub fn build(manifest:&Path,features:&[String])->Result<Value>{
    let source=std::fs::read_to_string(manifest)?;
    let package:toml::Value=toml::from_str(&source)?;
    let mut enabled=features.to_vec();
    let has_alias=package.get("features").and_then(|f|f.get("idl-build")).is_some();
    if has_alias { enabled.push("idl-build".into()); } else { enabled.push("anchor-lang/idl-build".into()); }
    if package.get("dependencies").and_then(|v|v.get("anchor-spl")).is_some(){enabled.push("anchor-spl/idl-build".into());}
    let manifest=manifest.canonicalize()?;
    let directory=manifest.parent().context("program manifest parent")?;
    let mut command=Command::new("cargo");
    // Match the pinned image's native compiler; do not inherit a workspace's SBF-era pin.
    let toolchain=env::var("RUSTUP_TOOLCHAIN").unwrap_or_else(|_|"1.99.0".into());
    command.arg(format!("+{toolchain}")).env("RUSTUP_TOOLCHAIN",&toolchain);
    command.args(["test","__anchor_private_print_idl","--manifest-path"]).arg(&manifest)
        .args(["--lib","--features",&enabled.join(","),"--","--show-output","--quiet"])
        .env("ANCHOR_IDL_BUILD_NO_DOCS","FALSE")
        .env("ANCHOR_IDL_BUILD_RESOLUTION","TRUE")
        .env("ANCHOR_IDL_BUILD_SKIP_LINT","TRUE")
        .env("ANCHOR_IDL_BUILD_PROGRAM_PATH",directory)
        .env("RUSTFLAGS","-A warnings --cfg feature=\"idl-build\"")
        .current_dir(directory).stderr(Stdio::piped());
    let output=command.output().context("run stock Anchor IDL tests")?;
    let stdout=String::from_utf8(output.stdout).context("stock IDL output UTF-8")?;
    if crate::verbose() || !output.status.success() { eprint!("{}",String::from_utf8_lossy(&output.stderr)); }
    if env::var_os("ANCHOR_LOG").is_some(){eprintln!("{stdout}");}
    if !output.status.success(){bail!("stock IDL build failed without a package idl-build alias; enable ANCHOR_LOG for macro output");}
    assemble(&stdout)
}

fn assemble(stdout:&str)->Result<Value>{
    let mut section:Option<&str>=None;let mut lines=Vec::new();let mut address=String::new();let mut constants=Vec::new();let mut events=Vec::new();let mut errors=None;let mut types=BTreeMap::new();let mut program=None;
    for line in stdout.lines(){
        if let Some(kind)=line.strip_prefix("--- IDL begin ").and_then(|s|s.strip_suffix(" ---")){
            if section.is_some(){bail!("nested stock IDL marker");}
            if kind=="errors"&&errors.is_some(){bail!("multiple error definitions are not allowed");}
            section=Some(match kind{"address"=>"address","const"=>"const","event"=>"event","errors"=>"errors","program"=>"program",_=>continue});lines.clear();continue
        }
        if section==Some("address"){
            address=line.chars().filter(|c|c.is_alphanumeric()).collect();section=None;continue
        }
        if let Some(kind)=section{
            if line==format!("--- IDL end {kind} ---"){
                let value:Value=serde_json::from_str(&lines.join("\n")).with_context(||format!("decode stock IDL {kind}"))?;
                match kind{
                    "const"=>constants.push(value),
                    "event"=>{events.push(value["event"].clone());for ty in value["types"].as_array().context("event IDL types")?{types.insert(ty["name"].as_str().context("event type name")?.to_owned(),ty.clone());}},
                    "errors"=>errors=Some(value),"program"=>program=Some(value),_=>{}
                }
                section=None;lines.clear();
            }else{lines.push(line.to_owned());}
        }
    }
    if section.is_some(){bail!("unterminated stock IDL marker");}
    let mut idl=program.ok_or_else(||anyhow!("stock macro output contains no IDL program"))?;
    if address.is_empty(){bail!("stock macro output contains no program address");}
    for ty in idl["types"].as_array().map(Vec::as_slice).unwrap_or(&[]){types.insert(ty["name"].as_str().context("program type name")?.to_owned(),ty.clone());}
    idl["address"]=json!(address);idl["constants"]=json!(constants);idl["events"]=json!(events);idl["errors"]=errors.unwrap_or(json!([]));idl["types"]=json!(types.into_values().collect::<Vec<_>>());
    simplify_paths(&mut idl);
    for field in ["accounts","constants","events","instructions","types"]{if let Some(values)=idl.get_mut(field).and_then(Value::as_array_mut){values.sort_by(|a,b|a["name"].as_str().cmp(&b["name"].as_str()));}}
    // Deserialize the public stock type, then perform the stock discriminator checks.
    let parsed:anchor_lang_idl::types::Idl=serde_json::from_value(idl.clone()).context("stock IDL schema")?;
    for field in ["accounts","events","instructions"]{
        let values=idl[field].as_array().map(Vec::as_slice).unwrap_or(&[]);
        for outer in values{
            let discriminator=outer["discriminator"].as_array().context("IDL discriminator")?;
            if discriminator.is_empty(){bail!("empty {field} discriminator: {}",outer["name"]);}
            for inner in values{if outer["name"]!=inner["name"]{let other=inner["discriminator"].as_array().context("IDL discriminator")?;if discriminator.starts_with(other){bail!("ambiguous {field} discriminators: {} {}",outer["name"],inner["name"]);}}}
        }
    }
    let accounts=idl["accounts"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    for account in accounts{let d=account["discriminator"].as_array().unwrap();let zeros=d.iter().take_while(|v|v.as_u64()==Some(0)).count();if zeros==d.len(){bail!("all-zero account discriminator: {}",account["name"]);}if accounts.iter().any(|other|other["discriminator"].as_array().unwrap().len()<=zeros){bail!("account discriminator conflicts with zero constraint: {}",account["name"]);}}
    Ok(serde_json::to_value(parsed)?)
}
fn simplify_paths(idl:&mut Value){
    // Match the stock builder's ordered replacement over JSON string values:
    // names simplify only when no other still-qualified path has that suffix.
    fn collect(value:&Value,paths:&mut Vec<String>){match value{
        Value::String(s)if s.contains("::")&&s.split("::").all(|p|!p.is_empty()&&p.chars().all(|c|c.is_ascii_alphanumeric()||c=='_'))=>{if !paths.contains(s){paths.push(s.clone());}},
        Value::Array(v)=>for v in v{collect(v,paths)},Value::Object(v)=>for v in v.values(){collect(v,paths)},_=>{}
    }}
    fn replace(value:&mut Value,path:&str,name:&str){match value{Value::String(s)if s==path=>*s=name.into(),Value::Array(v)=>for v in v{replace(v,path,name)},Value::Object(v)=>for v in v.values_mut(){replace(v,path,name)},_=>{}}}
    let mut paths=Vec::new();collect(idl,&mut paths);
    for path in paths {let name=path.rsplit("::").next().unwrap();let mut current=Vec::new();collect(idl,&mut current);if !current.iter().any(|other|other!=&path&&other.rsplit("::").next()==Some(name)){replace(idl,&path,name);}}
}
