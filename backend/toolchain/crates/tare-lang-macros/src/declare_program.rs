//! IDL-based external program bindings. No Anchor implementation is linked into the result.
use heck::{CamelCase, SnakeCase};
use proc_macro2::TokenStream as Tokens;
use quote::{format_ident, quote};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, path::PathBuf};
use syn::{Expr, Ident, Type};

type Result<T> = syn::Result<T>;

pub(super) fn expand(input: Tokens) -> Result<Tokens> {
    let name = syn::parse2::<Ident>(input)?;
    let manifest = std::env::var_os("CARGO_MANIFEST_DIR")
        .ok_or_else(|| error(&name, "CARGO_MANIFEST_DIR is missing"))?;
    let manifest = PathBuf::from(manifest);
    let directory = manifest.ancestors().map(|p| p.join("idls")).find(|p| p.is_dir())
        .ok_or_else(|| error(&name, "idls directory not found"))?;
    let path = directory.join(format!("{}.json", name));
    let bytes = std::fs::read(&path).map_err(|e| error(&name, format!("cannot read {}: {e}", path.display())))?;
    let raw: Value = serde_json::from_slice(&bytes).map_err(|e| error(&name, format!("invalid IDL {}: {e}", path.display())))?;
    let model = Model::parse(raw, &name)?;
    model.generate(&name)
}
fn error(at: &impl quote::ToTokens, text: impl std::fmt::Display) -> syn::Error {
    syn::Error::new_spanned(at, format!("Tare declare_program: {text}"))
}
fn string<'a>(v: &'a Value, key: &str, at: &Ident) -> Result<&'a str> {
    v.get(key).and_then(Value::as_str).ok_or_else(|| error(at, format!("IDL field `{key}` must be a string")))
}
fn list<'a>(v: &'a Value, key: &str, at: &Ident) -> Result<&'a [Value]> {
    match v.get(key) {None=>Ok(&[]),Some(Value::Array(v))=>Ok(v),_=>Err(error(at,format!("IDL field `{key}` must be an array")))}
}
fn ident(text: &str, at: &Ident) -> Result<Ident> {
    syn::parse_str(text).map_err(|_|error(at, format!("unsupported IDL identifier `{text}`")))
}
fn discriminator(v:&Value, namespace:&str, name:&str, at:&Ident)->Result<Vec<u8>> {
    match v.get("discriminator") {
        Some(Value::Array(bytes))=>bytes.iter().map(|b|b.as_u64().and_then(|b|u8::try_from(b).ok()).ok_or_else(||error(at,"IDL discriminator bytes must be in 0..255"))).collect(),
        Some(_)=>Err(error(at,"IDL discriminator must be a byte array")),
        None=>Ok(Sha256::digest(format!("{namespace}:{name}")).as_slice()[..8].to_vec()),
    }
}
fn docs(v:&Value)->Tokens {
    let lines=v.get("docs").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str);
    quote!{#(#[doc=#lines])*}
}
fn normalize_type(v:&mut Value) {
    match v {
        Value::String(s) if s=="publicKey"=>*s="pubkey".into(),
        Value::Object(map)=>{
            if let Some(Value::String(s))=map.get("defined") {let name=s.clone();map.insert("defined".into(),json!({"name":name}));}
            for value in map.values_mut(){normalize_type(value);}
        }
        Value::Array(values)=>for value in values {normalize_type(value);},
        _=>{}
    }
}
fn normalize_fields(fields:&mut Value) {
    if let Some(fields)=fields.as_array_mut() {
        for field in fields {
            if let Some(name)=field.get("name").and_then(Value::as_str) {
                field["name"]=Value::String(name.to_snake_case());
            }
            normalize_type(field);
        }
    }
}
fn normalize_accounts(accounts:&mut Value) {
    if let Some(accounts)=accounts.as_array_mut() {
        for acc in accounts {
            if let Some(name)=acc.get("name").and_then(Value::as_str) {acc["name"]=Value::String(name.to_snake_case());}
            if acc.get("accounts").is_some() {normalize_accounts(&mut acc["accounts"]);}
            else {
                acc["writable"]=acc.get("isMut").cloned().unwrap_or(Value::Bool(false));
                acc["signer"]=acc.get("isSigner").cloned().unwrap_or(Value::Bool(false));
                acc["optional"]=acc.get("isOptional").cloned().unwrap_or(Value::Bool(false));
                if let Some(map)=acc.as_object_mut(){map.remove("isMut");map.remove("isSigner");map.remove("isOptional");}
            }
        }
    }
}
fn normalize_definition(v:&mut Value) {
    if v["type"]["kind"]=="alias" {v["type"]["kind"]=Value::String("type".into());v["type"]["alias"]=v["type"]["value"].clone();}

    if let Some(fields)=v.get_mut("type").and_then(|t|t.get_mut("fields")){normalize_fields(fields);}
    if let Some(variants)=v.get_mut("type").and_then(|t|t.get_mut("variants")).and_then(Value::as_array_mut){
        for variant in variants {if let Some(fields)=variant.get_mut("fields"){normalize_fields(fields);}}
    }
    normalize_type(v);
}
struct Model {
    id: Vec<u8>,
    program_name:String,
    types:Vec<Value>,
    accounts:Vec<Value>,
    events:Vec<Value>,
    instructions:Vec<Value>,
    constants:Vec<Value>,
    layouts:Vec<Layout>,
}
struct Layout {name:String,accounts:Vec<Value>}
fn accounts_use_lifetime(accounts:&[Value])->bool {
    accounts.iter().any(|account|match account.get("accounts").and_then(Value::as_array) {
        Some(children)=>accounts_use_lifetime(children),
        None=>true,
    })
}
impl Model {
    fn parse(mut v:Value, at:&Ident)->Result<Self> {
        let legacy=v.get("metadata").and_then(|m|m.get("spec")).is_none();
        if !legacy && v["metadata"]["spec"]!="0.1.0" {return Err(error(at,"unsupported IDL spec"));}
        let address=if legacy {string(&v["metadata"],"address",at)?}else{string(&v,"address",at)?};
        let id=bs58::decode(address).into_vec().map_err(|_|error(at,"invalid IDL program address"))?;
        if id.len()!=32 {return Err(error(at,"IDL program address must decode to 32 bytes"));}
        let program_name=if legacy{string(&v,"name",at)?}else{string(&v["metadata"],"name",at)?}.to_owned();
        if legacy {
            if let Some(ixs)=v.get_mut("instructions").and_then(Value::as_array_mut) {
                for ix in ixs {
                    let name=string(ix,"name",at)?.to_snake_case();ix["name"]=Value::String(name);
                    normalize_accounts(&mut ix["accounts"]);normalize_fields(&mut ix["args"]);
                    if let Some(ret)=ix.get_mut("returns"){normalize_type(ret);}
                }
            }
        }
        let instructions=list(&v,"instructions",at)?.to_vec();
        let mut types=list(&v,"types",at)?.to_vec();
        let mut accounts=list(&v,"accounts",at)?.to_vec();
        let mut events=list(&v,"events",at)?.to_vec();
        if legacy {
            for ty in &mut types {normalize_definition(ty);}
            for account in &mut accounts {
                normalize_definition(account);types.push(account.clone());
            }
            for event in &mut events {
                normalize_fields(&mut event["fields"]);
                types.push(json!({"name":event["name"],"docs":event.get("docs").cloned().unwrap_or(json!([])),"type":{"kind":"struct","fields":event["fields"]}}));
            }
        }
        let mut layouts=Vec::new();
        for ix in &instructions {layouts.push(Layout{name:string(ix,"name",at)?.to_camel_case(),accounts:list(ix,"accounts",at)?.to_vec()});}
        let instruction_fields=layouts.iter().map(|layout|layout.accounts.clone()).collect::<Vec<_>>();
        for fields in instruction_fields {
            for field in fields {Self::collect_layouts(&field,&mut layouts,at)?;}
        }
        Ok(Self{id,program_name,types,accounts,events,instructions,constants:list(&v,"constants",at)?.to_vec(),layouts})
    }
    fn collect_layouts(field:&Value, layouts:&mut Vec<Layout>,at:&Ident)->Result<()> {
        if let Some(accounts)=field.get("accounts") {
            let fields=accounts.as_array().ok_or_else(||error(at,"composite IDL accounts must be an array"))?;
            if layouts.iter().any(|layout|layout.accounts==*fields) {return Ok(());}
            for child in fields {Self::collect_layouts(child,layouts,at)?;}
            let base=string(field,"name",at)?.to_camel_case();
            let mut name=base.clone();
            let mut suffix=2;
            while layouts.iter().any(|layout|layout.name==name) {
                name=format!("{base}{suffix}");
                suffix+=1;
            }
            layouts.push(Layout{name,accounts:fields.clone()});
        }
        Ok(())
    }
    fn generate(&self,module:&Ident)->Result<Tokens>{
        let id=&self.id;let program=ident(&self.program_name.to_camel_case(),module)?;
        let account_names=self.accounts.iter().map(|a|string(a,"name",module).map(str::to_owned)).collect::<Result<BTreeSet<_>>>()?;
        let event_names=self.events.iter().map(|a|string(a,"name",module).map(str::to_owned)).collect::<Result<BTreeSet<_>>>()?;
        let mut accounts=Vec::new();let mut events=Vec::new();let mut types=Vec::new();
        for ty in &self.types {
            let name=string(ty,"name",module)?;
            let generated=self.definition(ty,module)?;
            if account_names.contains(name) {
                let account=self.accounts.iter().find(|a|a["name"]==name).unwrap();
                let name=ident(name,module)?;let disc=discriminator(account,"account",&name.to_string(),module)?;
                let format=ty.get("serialization").and_then(Value::as_str).unwrap_or("borsh");
                let parse=if format=="borsh" {quote!{anchor_lang::AnchorDeserialize::deserialize(&mut &buf[Self::DISCRIMINATOR.len()..]).map_err(|_|anchor_lang::ErrorCode::AccountDidNotDeserialize.into())}}else{quote!{Ok(*anchor_lang::__private::bytemuck::from_bytes(&buf[Self::DISCRIMINATOR.len()..]))}};
                let ser=if format=="borsh" {quote!{impl anchor_lang::AccountSerialize for #name {
                    #[inline(always)] fn try_serialize<W:std::io::Write>(&self,writer:&mut W)->anchor_lang::Result<()>{
                        writer.write_all(Self::DISCRIMINATOR).map_err(|_|anchor_lang::ErrorCode::AccountDidNotSerialize)?;
                        anchor_lang::AnchorSerialize::serialize(self,writer).map_err(|_|anchor_lang::ErrorCode::AccountDidNotSerialize.into())
                    }
                }}}else{quote!{impl anchor_lang::ZeroCopy for #name {}}};
                accounts.push(quote!{
                    #generated
                    impl anchor_lang::Discriminator for #name {const DISCRIMINATOR:&'static[u8]=&[#(#disc),*];}
                    impl anchor_lang::Owner for #name {fn owner()->Pubkey{super::ID}}
                    #ser
                    impl anchor_lang::AccountDeserialize for #name {
                        #[inline(always)] fn try_deserialize(buf:&mut &[u8])->anchor_lang::Result<Self>{
                            if buf.len()<Self::DISCRIMINATOR.len(){return Err(anchor_lang::ErrorCode::AccountDiscriminatorNotFound.into());}
                            if &buf[..Self::DISCRIMINATOR.len()]!=Self::DISCRIMINATOR{return Err(anchor_lang::error!(anchor_lang::ErrorCode::AccountDiscriminatorMismatch).with_account_name(stringify!(#name)));}
                            Self::try_deserialize_unchecked(buf)
                        }
                        #[inline(always)] fn try_deserialize_unchecked(buf:&mut &[u8])->anchor_lang::Result<Self>{#parse}
                    }
                });
            }else if event_names.contains(name){
                let event=self.events.iter().find(|e|e["name"]==name).unwrap();let name=ident(name,module)?;let disc=discriminator(event,"event",&name.to_string(),module)?;
                events.push(quote!{#generated impl anchor_lang::Discriminator for #name {const DISCRIMINATOR:&'static[u8]=&[#(#disc),*];}
                    impl anchor_lang::Event for #name {fn data(&self)->Vec<u8>{let mut data=Vec::with_capacity(256);data.extend_from_slice(Self::DISCRIMINATOR);anchor_lang::AnchorSerialize::serialize(self,&mut data).unwrap();data}}
                });
            }else{types.push(generated);}
        }
        for name in account_names.union(&event_names){if !self.types.iter().any(|t|t["name"]==*name){return Err(error(module,format!("IDL type `{name}` is missing")));}}
        let constants=self.constants.iter().map(|c|{
            let name=ident(string(c,"name",module)?,module)?;let doc=docs(c);
            let value=syn::parse_str::<Expr>(string(c,"value",module)?).map_err(|e|error(module,format!("IDL constant `{name}`: {e}")))?;
            let ty=self.constant_ty(&c["type"],module)?;
            let value=match c["type"].as_str() {
                Some("bytes")=>quote!{&#value},
                Some("pubkey"|"publicKey")=>{
                    let bytes=bs58::decode(string(c,"value",module)?).into_vec().map_err(|_|error(module,"invalid constant public key"))?;
                    if bytes.len()!=32{return Err(error(module,"constant public key must decode to32 bytes"));}
                    quote!{Pubkey::new_from_array([#(#bytes),*])}
                },
                _=>quote!{#value},
            };
            Ok(quote!{#doc pub const #name:#ty=#value;})
        }).collect::<Result<Vec<_>>>()?;
        let args=self.instructions.iter().map(|ix|self.args(ix,module)).collect::<Result<Vec<_>>>()?;
        let cpi_accounts=self.layouts.iter().map(|l|self.layout(l,true,module)).collect::<Result<Vec<_>>>()?;
        let client_accounts=self.layouts.iter().map(|l|self.layout(l,false,module)).collect::<Result<Vec<_>>>()?;
        let cpi_functions=self.instructions.iter().map(|ix|self.cpi(ix,module)).collect::<Result<Vec<_>>>()?;
        let account_utils=self.utilities(&self.accounts,false,module)?;
        let event_utils=self.utilities(&self.events,true,module)?;
        let account_imports=account_names.iter().map(|name|ident(name,module)).collect::<Result<Vec<_>>>()?;
        let event_imports=event_names.iter().map(|name|ident(name,module)).collect::<Result<Vec<_>>>()?;
        let type_imports=self.types.iter().filter(|ty|!account_names.contains(ty["name"].as_str().unwrap_or("")) && !event_names.contains(ty["name"].as_str().unwrap_or(""))).map(|ty|ident(string(ty,"name",module)?,module)).collect::<Result<Vec<_>>>()?;
        Ok(quote!{
            pub mod #module {
                use anchor_lang::prelude::*;
                use accounts::{#(#account_imports),*};
                use events::{#(#event_imports),*};
                use types::{#(#type_imports),*};
                pub static ID:Pubkey=Pubkey::new_from_array([#(#id),*]);
                pub const ID_CONST:Pubkey=Pubkey::new_from_array([#(#id),*]);
                pub mod program {use super::*;#[derive(Clone)]pub struct #program;impl anchor_lang::Id for #program {fn id()->Pubkey{super::ID}}}
                pub mod constants {use super::*;#(#constants)*}
                pub mod accounts {use super::*;#(#accounts)*}
                pub mod events {use super::*;#(#events)*}
                pub mod types {use super::*;#(#types)*}
                pub mod client {use super::*;pub mod args {use super::*;#(#args)*}pub mod accounts {use super::*;#(#client_accounts)*}}
                pub mod cpi {use super::*;pub use anchor_lang::cpi::Return;pub mod accounts {use super::*;#(#cpi_accounts)*}#(#cpi_functions)*}
                pub mod utils {use super::*;#account_utils #event_utils}
            }
        })
    }
    fn constant_ty(&self,v:&Value,at:&Ident)->Result<Tokens> {
        if v=="string" {return Ok(quote!{&str});}
        if v=="bytes" {return Ok(quote!{&[u8]});}
        if let Some(inner)=v.get("option") {let inner=self.constant_ty(inner,at)?;return Ok(quote!{Option<#inner>});}
        if let Some(inner)=v.get("vec") {let inner=self.constant_ty(inner,at)?;return Ok(quote!{Vec<#inner>});}
        if let Some(array)=v.get("array").and_then(Value::as_array) {
            let parsed=syn::parse2::<Type>(self.ty(v,at)?).map_err(|e|error(at,e))?;
            let Type::Array(parsed)=parsed else {unreachable!()};let length=parsed.len;
            let inner=self.constant_ty(&array[0],at)?;
            return Ok(quote!{[#inner;#length]});
        }
        self.ty(v,at)
    }
    fn ty(&self,v:&Value,at:&Ident)->Result<Tokens>{
        if let Some(text)=v.as_str(){
            return match text {
                "bool"|"u8"|"i8"|"u16"|"i16"|"u32"|"i32"|"u64"|"i64"|"u128"|"i128"|"f32"|"f64"=>{let name=ident(text,at)?;Ok(quote!{#name})},
                "pubkey"|"publicKey"=>Ok(quote!{Pubkey}),"bytes"=>Ok(quote!{Vec<u8>}),"string"=>Ok(quote!{String}),
                _=>Err(error(at,format!("unsupported IDL type `{text}`")))
            };
        }
        if let Some(t)=v.get("option"){let ty=self.ty(t,at)?;return Ok(quote!{Option<#ty>});}
        if let Some(t)=v.get("vec"){let ty=self.ty(t,at)?;return Ok(quote!{Vec<#ty>});}
        if let Some(a)=v.get("array").and_then(Value::as_array){
            if a.len()!=2{return Err(error(at,"IDL array requires type and length"));}
            let ty=self.ty(&a[0],at)?;
            let length=if let Some(n)=a[1].as_u64(){syn::parse_str::<Expr>(&n.to_string())}else if let Some(s)=a[1].as_str().or_else(||a[1].get("generic").and_then(Value::as_str)){syn::parse_str::<Expr>(s)}else{return Err(error(at,"unsupported IDL array length"));}.map_err(|e|error(at,e))?;
            return Ok(quote!{[#ty;#length]});
        }
        if let Some(t)=v.get("generic").and_then(Value::as_str){let name=ident(t,at)?;return Ok(quote!{#name});}
        if let Some(d)=v.get("defined"){
            let name=if let Some(n)=d.as_str(){n}else{string(d,"name",at)?};
            let name=syn::parse_str::<Type>(name).map_err(|_|error(at,"unsupported defined IDL type name"))?;
            let mut args=Vec::new();
            for arg in list(d,"generics",at)? {
                if arg["kind"]=="type"{args.push(self.ty(&arg["type"],at)?);}else if arg["kind"]=="const"{let value=syn::parse_str::<Expr>(string(arg,"value",at)?).map_err(|e|error(at,e))?;args.push(quote!{#value});}else{return Err(error(at,"unsupported IDL generic argument"));}
            }
            return if args.is_empty(){Ok(quote!{#name})}else{Ok(quote!{#name<#(#args),*>})};
        }
        Err(error(at,format!("unsupported IDL type `{v}`")))
    }
    fn fields(&self,v:Option<&Value>,visibility:bool,at:&Ident)->Result<Tokens>{
        let fields=match v {None|Some(Value::Null)=>return Ok(quote!{;}),Some(Value::Array(a))=>a,_=>return Err(error(at,"IDL fields must be an array"))};
        if fields.is_empty(){return Ok(quote!{;});}
        let public=visibility.then(||quote!{pub});
        if fields[0].get("name").is_some(){
            let fields=fields.iter().map(|f|{let name=ident(string(f,"name",at)?,at)?;let ty=self.ty(&f["type"],at)?;let doc=docs(f);Ok(quote!{#doc #public #name:#ty})}).collect::<Result<Vec<_>>>()?;
            Ok(quote!{{#(#fields),*}})
        }else{let fields=fields.iter().map(|t|self.ty(t,at).map(|ty|quote!{#public #ty})).collect::<Result<Vec<_>>>()?;Ok(quote!{(#(#fields),*);})}
    }
    fn definition(&self,v:&Value,at:&Ident)->Result<Tokens>{
        let name=ident(string(v,"name",at)?,at)?;let doc=docs(v);
        let mut generic_defs=Vec::new();let mut generic_names=Vec::new();
        for g in list(v,"generics",at)? {let name=ident(string(g,"name",at)?,at)?;generic_names.push(quote!{#name});
            match string(g,"kind",at)? {"type"=>generic_defs.push(quote!{#name}),"const"=>{let ty=syn::parse_str::<Type>(string(g,"type",at)?).map_err(|e|error(at,e))?;generic_defs.push(quote!{const #name:#ty});},_=>return Err(error(at,"unsupported IDL type generic"))}
        }
        let generics=if generic_defs.is_empty(){quote!{}}else{quote!{<#(#generic_defs),*>}};
        let generic_args=if generic_names.is_empty(){quote!{}}else{quote!{<#(#generic_names),*>}};
        let serial=v.get("serialization").and_then(Value::as_str).unwrap_or("borsh");
        let attrs=match serial {
            "borsh"=>{
                let copy=self.can_derive(v,true,&mut BTreeSet::new());let default=self.can_derive(v,false,&mut BTreeSet::new());
                let copy=copy.then(||quote!{#[derive(Copy)]});let default=default.then(||quote!{#[derive(Default)]});
                quote!{#[derive(Debug,Clone,anchor_lang::AnchorSerialize,anchor_lang::AnchorDeserialize)]#copy #default}
            },
            "bytemuck"=>quote!{#[derive(Debug)]#[anchor_lang::zero_copy]},
            "bytemuckunsafe"|"bytemuckUnsafe"=>quote!{#[derive(Debug)]#[anchor_lang::zero_copy(unsafe)]},
            _=>return Err(error(at,format!("unsupported IDL serialization `{serial}`")))
        };
        let repr=match v.get("repr") {None|Some(Value::Null)=>quote!{},Some(r)=>{
            let kind=string(r,"kind",at)?;let kind=match kind{"rust"=>quote!{Rust},"c"=>quote!{C},"transparent"=>quote!{transparent},_=>return Err(error(at,"unsupported IDL representation"))};
            let packed=(r["packed"]==true).then(||quote!{,packed});let align=r.get("align").and_then(Value::as_u64).map(|n|{let n=proc_macro2::Literal::u64_unsuffixed(n);quote!{,align(#n)}});
            quote!{#[repr(#kind #packed #align)]}
        }};
        let kind=string(&v["type"],"kind",at)?;
        let body=match kind {
            "struct"=>{let fields=self.fields(v["type"].get("fields"),true,at)?;quote!{pub struct #name #generics #fields}},
            "enum"=>{
                let variants=list(&v["type"],"variants",at)?.iter().map(|variant|{let name=ident(string(variant,"name",at)?,at)?;let fields=self.fields(variant.get("fields"),false,at)?;let fields=fields.to_string().trim_end_matches(';').parse::<Tokens>().map_err(|e|error(at,e))?;Ok(quote!{#name #fields})}).collect::<Result<Vec<_>>>()?;
                quote!{pub enum #name #generics {#(#variants),*}}
            },
            "type"=>{let ty=self.ty(&v["type"]["alias"],at)?;return Ok(quote!{#doc pub type #name #generics=#ty;});},
            _=>return Err(error(at,format!("unsupported IDL definition kind `{kind}`")))
        };
        let unsafe_impl=if serial=="bytemuckunsafe"||serial=="bytemuckUnsafe"{quote!{unsafe impl #generics anchor_lang::__private::bytemuck::Pod for #name #generic_args {}unsafe impl #generics anchor_lang::__private::bytemuck::Zeroable for #name #generic_args {}}}else{quote!{}};
        Ok(quote!{#doc #attrs #repr #body #unsafe_impl})
    }
    fn can_derive(&self,v:&Value,copy:bool,seen:&mut BTreeSet<String>)->bool {
        let Some(name)=v["name"].as_str()else{return false};if !seen.insert(name.into()){return false;}
        let result=match v["type"]["kind"].as_str(){
            Some("struct")=>self.can_fields(v["type"].get("fields"),copy,seen),
            Some("enum")=>copy && v["type"]["variants"].as_array().is_some_and(|vs|vs.iter().all(|v|self.can_fields(v.get("fields"),copy,seen))),
            Some("type")=>self.can_type(&v["type"]["alias"],copy,seen),_=>false
        };seen.remove(name);result
    }
    fn can_fields(&self,v:Option<&Value>,copy:bool,seen:&mut BTreeSet<String>)->bool {
        v.and_then(Value::as_array).map_or(true,|fs|fs.iter().all(|f|self.can_type(f.get("type").unwrap_or(f),copy,seen)))
    }
    fn can_type(&self,v:&Value,copy:bool,seen:&mut BTreeSet<String>)->bool {
        if let Some(s)=v.as_str(){return !copy || !matches!(s,"bytes"|"string");}
        if let Some(v)=v.get("generic"){let _=v;return false;}
        if let Some(v)=v.get("option"){return self.can_type(v,copy,seen);}
        if let Some(v)=v.get("vec"){return !copy && self.can_type(v,copy,seen);}
        if let Some(a)=v.get("array").and_then(Value::as_array){return a.len()==2 && self.can_type(&a[0],copy,seen) && a[1].as_u64().is_some_and(|n|copy||n<=32);}
        if let Some(d)=v.get("defined"){let name=d.as_str().or_else(||d["name"].as_str());return name.and_then(|name|self.types.iter().find(|v|v["name"]==name)).is_some_and(|v|self.can_derive(v,copy,seen));}
        false
    }
    fn args(&self,ix:&Value,at:&Ident)->Result<Tokens>{
        let name=ident(&string(ix,"name",at)?.to_camel_case(),at)?;let disc=discriminator(ix,"global",string(ix,"name",at)?,at)?;
        let fields=self.fields(ix.get("args"),true,at)?;
        Ok(quote!{#[derive(anchor_lang::AnchorSerialize,anchor_lang::AnchorDeserialize)]pub struct #name #fields
            impl anchor_lang::Discriminator for #name{const DISCRIMINATOR:&'static[u8]=&[#(#disc),*];}
            impl anchor_lang::InstructionData for #name{}
            impl anchor_lang::Owner for #name{fn owner()->Pubkey{super::super::ID}}
        })
    }
    fn layout(&self,layout:&Layout,cpi:bool,at:&Ident)->Result<Tokens>{
        let name=ident(&layout.name,at)?;let generics=if cpi && accounts_use_lifetime(&layout.accounts){quote!{<'info>}}else{quote!{}};
        let mut fields=Vec::new();let mut infos=Vec::new();let mut metas=Vec::new();
        for field in &layout.accounts {
            let id=ident(string(field,"name",at)?,at)?;
            if let Some(children)=field.get("accounts").and_then(Value::as_array){
                let child=self.layouts.iter().find(|layout|layout.accounts==*children).ok_or_else(||error(at,"missing composite IDL account group"))?;
                let child_id=ident(&child.name,at)?;let life=if cpi && accounts_use_lifetime(&child.accounts){quote!{<'info>}}else{quote!{}};
                fields.push(quote!{pub #id:#child_id #life});
                infos.push(quote!{infos.extend(anchor_lang::ToAccountInfos::to_account_infos(&self.#id));});
                metas.extend(self.layout_metas(children,cpi,quote!{self.#id},at)?);
            }else{
                let mut ty=if cpi{quote!{anchor_lang::AccountInfo<'info>}}else{quote!{Pubkey}};
                let optional=field["optional"]==true;if optional{ty=quote!{Option<#ty>};}
                let signer=field["signer"]==true;let new=if field["writable"]==true{quote!{anchor_lang::solana_program::instruction::AccountMeta::new}}else{quote!{anchor_lang::solana_program::instruction::AccountMeta::new_readonly}};
                fields.push(quote!{pub #id:#ty});
                if optional{
                    let key=if cpi{quote!{*value.key}}else{quote!{*value}};
                    metas.push(quote!{if let Some(value)=&self.#id{metas.push(#new(#key,#signer));}else{metas.push(anchor_lang::solana_program::instruction::AccountMeta::new_readonly(super::super::ID,false));}});
                    infos.push(quote!{if let Some(value)=&self.#id{infos.push(value.clone());}});
                }else{
                    let key=if cpi{quote!{*self.#id.key}}else{quote!{self.#id}};
                    metas.push(quote!{metas.push(#new(#key,#signer));});infos.push(quote!{infos.push(self.#id.clone());});
                }
            }
        }
        let derive=if cpi{quote!{}}else{quote!{#[derive(anchor_lang::AnchorSerialize)]}};
        let info_impl=if cpi{quote!{impl<'info> anchor_lang::ToAccountInfos<'info> for #name #generics{fn to_account_infos(&self)->Vec<anchor_lang::AccountInfo<'info>>{let mut infos=Vec::new();#(#infos)*infos}}}}else{quote!{}};
        Ok(quote!{#derive pub struct #name #generics{#(#fields),*}
            impl #generics anchor_lang::ToAccountMetas for #name #generics{fn to_account_metas(&self,_is_signer:Option<bool>)->Vec<anchor_lang::solana_program::instruction::AccountMeta>{let mut metas=Vec::new();#(#metas)*metas}}
            #info_impl
        })
    }
    fn layout_metas(&self,fields:&[Value],cpi:bool,base:Tokens,at:&Ident)->Result<Vec<Tokens>> {
        let mut result=Vec::new();
        for field in fields {
            let name=ident(string(field,"name",at)?,at)?;
            let access=quote!{#base.#name};
            if let Some(children)=field.get("accounts").and_then(Value::as_array) {
                result.extend(self.layout_metas(children,cpi,access,at)?);
            } else {
                let signer=field["signer"]==true;
                let new=if field["writable"]==true {quote!{anchor_lang::solana_program::instruction::AccountMeta::new}} else {quote!{anchor_lang::solana_program::instruction::AccountMeta::new_readonly}};
                if field["optional"]==true {
                    let key=if cpi{quote!{*value.key}}else{quote!{*value}};
                    result.push(quote!{if let Some(value)=&#access {metas.push(#new(#key,#signer));} else {metas.push(anchor_lang::solana_program::instruction::AccountMeta::new_readonly(super::super::ID,false));}});
                } else {
                    let key=if cpi{quote!{*#access.key}}else{access};
                    result.push(quote!{metas.push(#new(#key,#signer));});
                }
            }
        }
        Ok(result)
    }
    fn cpi(&self,ix:&Value,at:&Ident)->Result<Tokens>{
        let name=ident(string(ix,"name",at)?,at)?;let ty=ident(&name.to_string().to_camel_case(),at)?;
        let life=if accounts_use_lifetime(list(ix,"accounts",at)?){quote!{<'info>}}else{quote!{}};
        let mut params=Vec::new();let mut names=Vec::new();
        for arg in list(ix,"args",at)? {let name=ident(string(arg,"name",at)?,at)?;let ty=self.ty(&arg["type"],at)?;params.push(quote!{#name:#ty});names.push(name);}
        let data=if names.is_empty(){quote!{super::client::args::#ty}}else{quote!{super::client::args::#ty{#(#names),*}}};
        let (ret,result)=if let Some(ret)=ix.get("returns").filter(|v|!v.is_null()){let ty=self.ty(ret,at)?;(quote!{anchor_lang::Result<Return<#ty>>},quote!{Ok(Return::new())})}else{(quote!{anchor_lang::Result<()>},quote!{Ok(())})};
        Ok(quote!{pub fn #name<'a,'b,'c,'info>(ctx:anchor_lang::CpiContext<'a,'b,'c,'info,accounts::#ty #life>,#(#params),*)->#ret{
            let __tare_args=#data;let mut __tare_data=Vec::with_capacity(256);
            __tare_data.extend_from_slice(<super::client::args::#ty as anchor_lang::Discriminator>::DISCRIMINATOR);
            anchor_lang::AnchorSerialize::serialize(&__tare_args,&mut __tare_data).map_err(|_|anchor_lang::ErrorCode::InstructionDidNotSerialize)?;
            anchor_lang::cpi::invoke_dynamic_with_id(ctx.program.key,&ctx.to_account_infos(),&ctx.to_account_metas(None),&__tare_data,ctx.signer_seeds)?;
            #result
        }})
    }
    fn utilities(&self,entries:&[Value],event:bool,at:&Ident)->Result<Tokens>{
        let enum_name=if event{format_ident!("Event")}else{format_ident!("Account")};
        let mut variants=Vec::new();let mut checks=Vec::new();
        for entry in entries {let name=ident(string(entry,"name",at)?,at)?;variants.push(quote!{#name(#name)});
            let parse=if event{quote!{anchor_lang::AnchorDeserialize::try_from_slice(&value[<#name as anchor_lang::Discriminator>::DISCRIMINATOR.len()..]).map(Self::#name).map_err(Into::into)}}else{quote!{<#name as anchor_lang::AccountDeserialize>::try_deserialize_unchecked(&mut &value[..]).map(Self::#name).map_err(Into::into)}};
            checks.push(quote!{if value.starts_with(<#name as anchor_lang::Discriminator>::DISCRIMINATOR){return #parse;}});
        }
        Ok(quote!{pub enum #enum_name{#(#variants),*}impl #enum_name{#[inline(always)] pub fn try_from_bytes(bytes:&[u8])->anchor_lang::Result<Self>{Self::try_from(bytes)}}
            impl TryFrom<&[u8]> for #enum_name{type Error=anchor_lang::Error;#[inline(always)] fn try_from(value:&[u8])->anchor_lang::Result<Self>{#(#checks)*Err(anchor_lang::solana_program::program_error::ProgramError::InvalidArgument.into())}}
        })
    }
}
