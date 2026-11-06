use anyhow::Result;
use quote::ToTokens;
use serde_json::{json, Value};
use std::{collections::BTreeMap,path::{Path,PathBuf}};
use syn::{visit::{self,Visit},Type,GenericArgument,PathArguments};
fn files(root:&Path,out:&mut Vec<PathBuf>)->Result<()>{let mut entries=std::fs::read_dir(root)?.map(|e|e.map(|e|e.path())).collect::<std::io::Result<Vec<_>>>()?;entries.sort();for p in entries {if p.is_dir(){files(&p,out)?}else if p.extension().is_some_and(|x|x=="rs"){out.push(p)}}Ok(())}
struct Gather{path:String, structs:BTreeMap<String,Value>, functions:BTreeMap<String,Value>}
fn context(ty:&Type)->Option<String>{if let Type::Path(p)=ty {for seg in &p.path.segments {if seg.ident=="Context" {if let PathArguments::AngleBracketed(a)=&seg.arguments {for g in a.args.iter().rev(){if let GenericArgument::Type(t)=g {if let Type::Path(path)=t{return path.path.segments.last().map(|s|s.ident.to_string())}}}}}}}None}
impl<'a> Visit<'a> for Gather{
 fn visit_item_struct(&mut self,s:&'a syn::ItemStruct){let fields=s.fields.iter().map(|f|json!({"name":f.ident.as_ref().map(ToString::to_string),"type":f.ty.to_token_stream().to_string(),"attributes":f.attrs.iter().map(|a|a.to_token_stream().to_string()).collect::<Vec<_>>()})).collect::<Vec<_>>();self.structs.insert(s.ident.to_string(),json!({"path":self.path,"fields":fields}));visit::visit_item_struct(self,s)}
 fn visit_item_fn(&mut self,f:&'a syn::ItemFn){for a in &f.sig.inputs{if let syn::FnArg::Typed(t)=a{if let Some(ctx)=context(&t.ty){self.functions.insert(f.sig.ident.to_string(),json!({"path":self.path,"context":ctx,"body":f.block.to_token_stream().to_string()}));}}}visit::visit_item_fn(self,f)}
}
fn main()->Result<()>{let source=PathBuf::from(std::env::args().nth(1).expect("source path"));let root=source.parent().unwrap();let mut paths=vec![];files(root,&mut paths)?;let mut g=Gather{path:String::new(),structs:BTreeMap::new(),functions:BTreeMap::new()};for p in paths{g.path=p.strip_prefix(root)?.to_string_lossy().replace('\\',"/");let file=syn::parse_file(&std::fs::read_to_string(&p)?)?;g.visit_file(&file)}println!("{}",json!({"functions":g.functions,"structs":g.structs}));Ok(())}
