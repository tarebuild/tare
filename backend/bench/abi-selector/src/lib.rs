//! Conservative source detector for observable account representation.
use anyhow::{Context, Result};
use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
use syn::{
    visit::{self, Visit},
    GenericArgument, Item, PathArguments, Type,
};

#[derive(Clone)]
enum Definition {
    Fields(Vec<Type>),
    GenericFields(Vec<String>, Vec<Type>),
    Alias(Type),
    ProvenIndependent,
}
#[derive(Clone)]
struct Types {
    definitions: BTreeMap<String, Option<Definition>>,
    observations: BTreeMap<String, String>,
    external_glob: bool,
    payload_imports: BTreeMap<String,Type>,
}
impl Types {
    fn collect(files: &[(PathBuf, syn::File)]) -> Self {
        fn collect_items(items: &[Item], definitions: &mut BTreeMap<String, Option<Definition>>) {
            for item in items {
                let named = match item {
                    Item::Struct(s) => Some((
                        s.ident.to_string(),
                        if s.generics.type_params().next().is_some() {
                            Definition::GenericFields(s.generics.type_params().map(|p|p.ident.to_string()).collect(), s.fields.iter().map(|f|f.ty.clone()).collect())
                        } else { Definition::Fields(s.fields.iter().map(|f| f.ty.clone()).collect()) },
                    )),
                    Item::Enum(e) => Some((
                        e.ident.to_string(),
                        Definition::Fields(
                            e.variants
                                .iter()
                                .flat_map(|v| v.fields.iter().map(|f| f.ty.clone()))
                                .collect(),
                        ),
                    )),
                    Item::Union(u) => Some((
                        u.ident.to_string(),
                        Definition::Fields(u.fields.named.iter().map(|f| f.ty.clone()).collect()),
                    )),
                    Item::Type(t) => {
                        Some((t.ident.to_string(), Definition::Alias((*t.ty).clone())))
                    }
                    _ => None,
                };
                if let Some((name, definition)) = named {
                    if definitions.contains_key(&name) {
                        definitions.insert(name, None);
                    } else {
                        definitions.insert(name, Some(definition));
                    }
                }
                if let Item::Mod(m) = item {
                    definitions.entry(m.ident.to_string()).or_insert(None);
                    if let Some((_, content)) = &m.content {
                        collect_items(content, definitions)
                    }
                }
            }
        }
        let mut definitions = BTreeMap::new();
        for (_, file) in files {
            collect_items(&file.items, &mut definitions)
        }
        struct Integers<'a>(&'a mut BTreeMap<String, Option<Definition>>);
        impl<'ast> Visit<'ast> for Integers<'_> {
            fn visit_item_macro(&mut self, node: &'ast syn::ItemMacro) {
                if node.mac.path.segments.last().is_some_and(|s|s.ident=="construct_uint") {
                    struct IntegerDecl { name:syn::Ident, limbs:syn::LitInt }
                    impl syn::parse::Parse for IntegerDecl {
                        fn parse(input:syn::parse::ParseStream)->syn::Result<Self> {
                            let _visibility: syn::Visibility=input.parse()?;
                            input.parse::<syn::Token![struct]>()?;
                            let name=input.parse()?;let content;syn::parenthesized!(content in input);let limbs=content.parse()?;
                            input.parse::<syn::Token![;]>()?;
                            if !input.is_empty(){return Err(input.error("one integer declaration expected"))}
                            Ok(Self{name,limbs})
                        }
                    }
                    if let Ok(decl)=syn::parse2::<IntegerDecl>(node.mac.tokens.clone()) {
                        if let Ok(limbs)=decl.limbs.base10_parse::<usize>() {
                            if limbs>0 {
                                let ty:Type=syn::parse_quote!([u64; #limbs]);
                                self.0.entry(decl.name.to_string()).or_insert(Some(Definition::Fields(vec![ty])));
                            }
                        }
                    }
                }
                visit::visit_item_macro(self,node);
            }
        }
        // Only the canonical uint macro has this representation. A local macro
        // with the same name remains unknown.
        let local_uint_macro=files.iter().any(|(_,f)|f.items.iter().any(|i|matches!(i,Item::Macro(m) if m.ident.as_ref().is_some_and(|id|id=="construct_uint"))));
        let canonical_uint=!local_uint_macro && files.iter().any(|(_,f)|f.items.iter().any(|i|matches!(i,Item::Use(u) if u.to_token_stream().to_string().replace(' ',"")=="useuint::construct_uint;")));
        if canonical_uint { for (_,file) in files {Integers(&mut definitions).visit_file(file)} }
        struct BlockDefinitions<'a>(&'a mut BTreeMap<String, Option<Definition>>);
        impl<'ast> Visit<'ast> for BlockDefinitions<'_> {
            fn visit_block(&mut self, block: &'ast syn::Block) {
                for statement in &block.stmts {
                    if let syn::Stmt::Item(item) = statement {
                        collect_items(std::slice::from_ref(item), self.0);
                    }
                }
                visit::visit_block(self, block);
            }
        }
        for (_, file) in files {
            BlockDefinitions(&mut definitions).visit_file(file);
        }
        fn aliases(tree: &syn::UseTree, out: &mut BTreeMap<String, String>) {
            match tree {
                syn::UseTree::Path(p) => aliases(&p.tree, out),
                syn::UseTree::Group(g) => {
                    for t in &g.items {
                        aliases(t, out)
                    }
                }
                syn::UseTree::Rename(r) => {
                    let name = r.ident.to_string();
                    if [
                        "size_of",
                        "align_of",
                        "type_name",
                        "TypeId",
                        "transmute",
                        "transmute_copy",
                        "size_of_val",
                        "align_of_val",
                        "addr_of",
                        "addr_of_mut",
                        "Any",
                        "Rc",
                        "RefCell",
                    ]
                    .contains(&name.as_str())
                    {
                        out.insert(r.rename.to_string(), name);
                    }
                }
                _ => {}
            }
        }
        fn type_imports(
            tree: &syn::UseTree,
            prefix: &mut Vec<String>,
            names: &mut BTreeSet<String>,
            external_glob: &mut bool,
        ) {
            match tree {
                syn::UseTree::Path(p) => {
                    prefix.push(p.ident.to_string());
                    type_imports(&p.tree, prefix, names, external_glob);
                    prefix.pop();
                }
                syn::UseTree::Group(g) => {
                    for t in &g.items {
                        type_imports(t, prefix, names, external_glob)
                    }
                }
                syn::UseTree::Name(n) => {
                    if !prefix
                        .first()
                        .is_some_and(|p| p == "crate" || p == "self" || p == "super")
                    {
                        names.insert(n.ident.to_string());
                    }
                }
                syn::UseTree::Rename(n) => {
                    // Renames can shadow a built-in payload name even through a local module.
                    // Resolving the target needs full Rust name resolution, so retain SDK.
                    names.insert(n.rename.to_string());
                }
                syn::UseTree::Glob(_) => {
                    if !prefix.first().is_some_and(|p| {
                        ["crate", "self", "super", "anchor_lang", "std", "core", "pinocchio"].contains(&p.as_str())
                    }) {
                        *external_glob = true
                    }
                }
            }
        }
        fn all_type_imports(items: &[Item], names: &mut BTreeSet<String>, glob: &mut bool) {
            for item in items {
                match item {
                    Item::Use(u) => type_imports(&u.tree, &mut vec![], names, glob),
                    Item::Mod(m) => {
                        if let Some((_, content)) = &m.content {
                            all_type_imports(content, names, glob)
                        }
                    }
                    _ => {}
                }
            }
        }
        let mut imported = BTreeSet::new();
        let mut external_glob = false;
        for (_, file) in files {
            all_type_imports(&file.items, &mut imported, &mut external_glob)
        }
        for name in imported {
            definitions.insert(name, None);
        }
        struct GenericNames(BTreeSet<String>);
        impl<'ast> Visit<'ast> for GenericNames {
            fn visit_type_param(&mut self, p: &'ast syn::TypeParam) {
                self.0.insert(p.ident.to_string());
            }
        }
        let mut generic_names = GenericNames(BTreeSet::new());
        for (_, file) in files {
            generic_names.visit_file(file)
        }
        for name in generic_names.0 {
            definitions.insert(name, None);
        }
        struct Uses {
            observations: BTreeMap<String, String>,
            names: BTreeSet<String>,
            external_glob: bool,
        }
        impl<'ast> Visit<'ast> for Uses {
            fn visit_item_use(&mut self, u: &'ast syn::ItemUse) {
                aliases(&u.tree, &mut self.observations);
                type_imports(
                    &u.tree,
                    &mut vec![],
                    &mut self.names,
                    &mut self.external_glob,
                );
            }
        }
        let mut uses = Uses {
            observations: BTreeMap::new(),
            names: BTreeSet::new(),
            external_glob,
        };
        for (_, file) in files {
            uses.visit_file(file)
        }
        for name in uses.names {
            definitions.insert(name, None);
        }
        Self {
            definitions,
            observations: uses.observations,
            external_glob: uses.external_glob,
            payload_imports: BTreeMap::new(),
        }
    }
    fn independent(&self, ty: &Type, stack: &mut BTreeSet<String>) -> bool {
        match ty {
            Type::Array(a) => self.independent(&a.elem, stack),
            Type::Slice(a) => self.independent(&a.elem, stack),
            Type::Reference(a) => self.independent(&a.elem, stack),
            Type::Ptr(a) => self.independent(&a.elem, stack),
            Type::Paren(a) => self.independent(&a.elem, stack),
            Type::Group(a) => self.independent(&a.elem, stack),
            Type::Tuple(t) => t.elems.iter().all(|t| self.independent(t, stack)),
            Type::Never(_) => true,
            Type::Path(p) if p.qself.is_none() => {
                let Some(last) = p.path.segments.last() else {
                    return false;
                };
                let name = last.ident.to_string();
                if let Some(payload)=self.payload_imports.get(&name) {
                    if p.path.segments.len()==1 {return self.independent(payload,stack)}
                }
                if [
                    "AccountInfo",
                    "AccountData",
                    "AccountLamports",
                    "DataRef",
                    "LamportsRef",
                    "Account",
                    "AccountLoader",
                    "InterfaceAccount",
                    "Signer",
                    "Program",
                    "Interface",
                    "SystemAccount",
                    "UncheckedAccount",
                    "Sysvar",
                    "Context",
                    "CpiContext",
                    "Any",
                    "Ref",
                    "RefMut",
                ]
                .contains(&name.as_str())
                {
                    return false;
                }
                let arguments = match &last.arguments {
                    PathArguments::None => vec![],
                    PathArguments::AngleBracketed(a) => {
                        let mut out = vec![];
                        for arg in &a.args {
                            match arg {
                                GenericArgument::Type(t) => out.push(t),
                                GenericArgument::Lifetime(_) | GenericArgument::Const(_) => {}
                                _ => return false,
                            }
                        }
                        out
                    }
                    _ => return false,
                };
                if !arguments.iter().all(|t| self.independent(t, stack)) {
                    return false;
                }
                let known_path = p.path.segments.len() == 1
                    || p.path.segments.first().is_some_and(|s| {
                        !self.definitions.contains_key(&s.ident.to_string())
                            && ["std", "core", "alloc", "solana_program", "anchor_lang"]
                                .contains(&s.ident.to_string().as_str())
                    });
                if known_path
                    && !self.definitions.contains_key(&name)
                    && [
                        "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64",
                        "i128", "isize", "f32", "f64", "bool", "char", "str", "String", "Pubkey",
                        "Duration",
                    ]
                    .contains(&name.as_str())
                {
                    return true;
                }
                if known_path
                    && !self.definitions.contains_key(&name)
                    && [
                        "Vec",
                        "Option",
                        "Result",
                        "Box",
                        "PhantomData",
                        "MaybeUninit",
                        "ManuallyDrop",
                        "Cell",
                        "RefCell",
                        "Rc",
                        "Arc",
                        "BTreeMap",
                        "BTreeSet",
                        "HashMap",
                        "HashSet",
                    ]
                    .contains(&name.as_str())
                {
                    return !arguments.is_empty();
                }
                // Local lookup accepts only the unqualified name or Self-module form.
                // Other module paths require name resolution and remain conservative.
                if p.path.segments.len() > 1
                    && !(p.path.segments.len() == 2 && p.path.segments[0].ident == "self")
                {
                    return false;
                }
                // Ambiguous local names, recursive or generic types remain conservative.
                if self.external_glob || (!arguments.is_empty() && !matches!(self.definitions.get(&name), Some(Some(Definition::GenericFields(..))))) || !stack.insert(name.clone()) {
                    return false;
                }
                let answer = match self.definitions.get(&name).and_then(Option::as_ref) {
                    Some(Definition::Fields(fields)) => {
                        fields.iter().all(|t| self.independent(t, stack))
                    }
                    Some(Definition::Alias(ty)) => self.independent(ty, stack),
                    Some(Definition::ProvenIndependent) => true,
                    Some(Definition::GenericFields(names, fields)) if names.len()==arguments.len() => {
                        let bindings:BTreeMap<String,Type>=names.iter().cloned().zip(arguments.iter().map(|t|(*t).clone())).collect();
                        fields.iter().all(|ty|self.independent(&substitute(ty,&bindings),stack))
                    }
                    _ => false,
                };
                stack.remove(&name);
                answer
            }
            _ => false,
        }
    }
}
fn substitute(ty:&Type, bindings:&BTreeMap<String,Type>)->Type {
    if let Type::Path(p)=ty {
        if p.qself.is_none()&&p.path.segments.len()==1 {
            if let Some(value)=bindings.get(&p.path.segments[0].ident.to_string()){return value.clone()}
        }
    }
    let mut out=ty.clone();
    match &mut out {
        Type::Array(t)=>t.elem=Box::new(substitute(&t.elem,bindings)),
        Type::Slice(t)=>t.elem=Box::new(substitute(&t.elem,bindings)),
        Type::Reference(t)=>t.elem=Box::new(substitute(&t.elem,bindings)),
        Type::Ptr(t)=>t.elem=Box::new(substitute(&t.elem,bindings)),
        Type::Paren(t)=>t.elem=Box::new(substitute(&t.elem,bindings)),
        Type::Group(t)=>t.elem=Box::new(substitute(&t.elem,bindings)),
        Type::Tuple(t)=>{for elem in &mut t.elems{*elem=substitute(elem,bindings)}},
        Type::Path(t)=>{for seg in &mut t.path.segments{if let PathArguments::AngleBracketed(a)=&mut seg.arguments{for arg in &mut a.args{if let GenericArgument::Type(t)=arg{*t=substitute(t,bindings)}}}}},
        _=>{}
    }
    out
}

fn observation(stream: TokenStream, types: &Types) -> Option<String> {
    // Inspect tokens in attributes/macros too; comments and string contents
    // never become observation identifiers.
    let tokens: Vec<_> = stream.into_iter().collect();
    for (index, token) in tokens.iter().enumerate() {
        if let TokenTree::Group(g) = token {
            if let Some(reason) = observation(g.stream(), types) {
                return Some(reason);
            }
        }
        let TokenTree::Ident(ident) = token else {
            continue;
        };
        let local = ident.to_string();
        let name = types.observations.get(&local).cloned().unwrap_or(local);
        if ![
            "size_of",
            "align_of",
            "type_name",
            "TypeId",
            "transmute",
            "transmute_copy",
            "size_of_val",
            "align_of_val",
            "addr_of",
            "addr_of_mut",
            "downcast",
            "downcast_ref",
            "downcast_mut",
            "Any",
            "Rc",
            "RefCell",
        ]
        .contains(&name.as_str())
        {
            continue;
        }
        if [
            "addr_of",
            "addr_of_mut",
            "downcast",
            "downcast_ref",
            "downcast_mut",
            "Any",
            "size_of_val",
            "align_of_val",
            "Rc",
            "RefCell",
        ]
        .contains(&name.as_str())
        {
            return Some(format!(
                "representation observation {name} with unresolved value/type"
            ));
        }
        let mut at = index + 1;
        if name == "TypeId" {
            while at < tokens.len() && !matches!(&tokens[at],TokenTree::Ident(id)if id=="of") {
                at += 1
            }
            if at == tokens.len() {
                return Some("TypeId observation with unresolved type".into());
            }
            at += 1;
        }
        while at < tokens.len() && matches!(&tokens[at],TokenTree::Punct(p)if p.as_char()==':') {
            at += 1
        }
        if !matches!(tokens.get(at),Some(TokenTree::Punct(p))if p.as_char()=='<') {
            return Some(format!(
                "representation observation {name} with unresolved type"
            ));
        }
        let mut depth = 0;
        let mut generic = TokenStream::new();
        let mut end = at;
        for (offset, token) in tokens[at..].iter().enumerate() {
            match token {
                TokenTree::Punct(p) if p.as_char() == '<' => depth += 1,
                TokenTree::Punct(p) if p.as_char() == '>' => {
                    depth -= 1;
                    if depth == 0 {
                        end = at + offset;
                        break;
                    }
                }
                _ => {}
            }
            generic.extend([token.clone()]);
        }
        if depth != 0 {
            return Some(format!(
                "representation observation {name} with unparsed type"
            ));
        }
        generic.extend([tokens[end].clone()]);
        let arguments: syn::AngleBracketedGenericArguments = match syn::parse2(generic) {
            Ok(a) => a,
            Err(_) => {
                return Some(format!(
                    "representation observation {name} with unparsed type"
                ))
            }
        };
        let mut seen = false;
        for arg in arguments.args {
            if let GenericArgument::Type(ty) = arg {
                seen = true;
                if !types.independent(&ty, &mut BTreeSet::new()) {
                    return Some(format!(
                        "representation observation {name}<{}>",
                        ty.to_token_stream()
                    ));
                }
            }
        }
        if !seen {
            return Some(format!(
                "representation observation {name} with unresolved type"
            ));
        }
    }
    None
}
struct Inspect<'a> {
    types: &'a Types,
    reason: Option<String>,
}
impl<'ast> Visit<'ast> for Inspect<'_> {
    fn visit_item_mod(&mut self,node:&'ast syn::ItemMod) {
        let only_test=node.attrs.iter().any(|a|a.path().is_ident("cfg")&&a.to_token_stream().to_string().replace(' ',"")=="#[cfg(test)]");
        if !only_test {visit::visit_item_mod(self,node)}
    }
    fn visit_item_fn(&mut self,node:&'ast syn::ItemFn) {
        let only_test=node.attrs.iter().any(|a|a.path().is_ident("test")||(a.path().is_ident("cfg")&&a.to_token_stream().to_string().replace(' ',"")=="#[cfg(test)]"));
        if !only_test {visit::visit_item_fn(self,node)}
    }

    fn visit_item_impl(&mut self, node:&'ast syn::ItemImpl) {
        let mut local=self.types.clone();
        local.definitions.insert("Self".into(),Some(Definition::Alias((*node.self_ty).clone())));
        let mut inner=Inspect {types:&local,reason:None};
        visit::visit_item_impl(&mut inner,node);
        if self.reason.is_none(){self.reason=inner.reason}
    }
    fn visit_attribute(&mut self, node: &'ast syn::Attribute) {
        if self.reason.is_none() {
            self.reason = observation(node.meta.to_token_stream(), self.types)
        }
        visit::visit_attribute(self, node)
    }
    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        if self.reason.is_none() {
            self.reason = observation(node.to_token_stream(), self.types)
        }
        visit::visit_expr_path(self, node)
    }
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if self.reason.is_none() {
            self.reason = observation(node.func.to_token_stream(), self.types)
        }
        visit::visit_expr_call(self, node)
    }
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if self.reason.is_none() && node.method.to_string().starts_with("downcast") {
            self.reason = Some("representation observation downcast".into())
        }
        visit::visit_expr_method_call(self, node)
    }
    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if self.reason.is_none()
            && node
                .path
                .segments
                .last()
                .is_some_and(|s| s.ident == "addr_of" || s.ident == "addr_of_mut")
        {
            self.reason = Some("representation observation addr_of".into())
        }
        if self.reason.is_none() {
            self.reason = observation(node.tokens.clone(), self.types)
        }
        visit::visit_macro(self, node)
    }
    fn visit_type_path(&mut self, node: &'ast syn::TypePath) {
        if self.reason.is_none() && node.path.segments.last().is_some_and(|s| s.ident == "Any") {
            self.reason = Some("representation observation dyn Any".into())
        }
        visit::visit_type_path(self, node)
    }
}
fn dependency_metadata(source:&Path)->Option<serde_json::Value> {
    static CACHE:std::sync::OnceLock<std::sync::Mutex<BTreeMap<PathBuf,Option<serde_json::Value>>>>=std::sync::OnceLock::new();
    let manifest=source.ancestors().map(|p|p.join("Cargo.toml")).find(|p|p.exists())?;
    let cache=CACHE.get_or_init(||std::sync::Mutex::new(BTreeMap::new()));
    if let Some(value)=cache.lock().ok()?.get(&manifest).cloned(){return value}
    let output=std::process::Command::new("cargo").args(["metadata","--offline","--locked","--format-version","1","--manifest-path"]).arg(&manifest).output().ok()?;
    let value=if output.status.success(){serde_json::from_slice(&output.stdout).ok()}else{None};
    cache.lock().ok()?.insert(manifest,value.clone());value
}

fn resolve_local_payload_imports(source:&Path, files:&[(PathBuf,syn::File)],types:&mut Types) {
    let uses_pinocchio=files.iter().any(|(_,file)|file.items.iter().any(|i|matches!(i,Item::Use(u) if u.to_token_stream().to_string().replace(' ',"")=="pubusepinocchio::pubkey::Pubkey;")));
    if !uses_pinocchio {return}
    let Some(metadata)=dependency_metadata(source) else{return};
    let Some(packages)=metadata["packages"].as_array() else{return};
    let packages:Vec<_>=packages.iter().filter(|p|p["name"].as_str()==Some("pinocchio")).collect();
    if packages.len()!=1{return}
    let Some(manifest)=packages[0]["manifest_path"].as_str() else{return};
    let Some(root)=Path::new(manifest).parent() else{return};
    let Ok(text)=fs::read_to_string(root.join("src/pubkey.rs")) else{return};
    let compact=text.split_whitespace().collect::<String>();
    if !compact.contains("pubtypePubkey=[u8;PUBKEY_BYTES];")||!compact.contains("PUBKEY_BYTES:usize=32;"){return}
    let no_local_key=files.iter().all(|(_,f)|f.items.iter().all(|i|!matches!(i,Item::Struct(x) if x.ident=="Pubkey")&&!matches!(i,Item::Enum(x) if x.ident=="Pubkey")&&!matches!(i,Item::Type(x) if x.ident=="Pubkey")));
    if no_local_key {types.payload_imports.insert("Pubkey".into(),syn::parse_quote!([u8;32]));}
}

// Resolve this exact public reexport through the workspace's locked packages.
// Unknown imports remain unknown; no bare type-name exemption is installed.
fn resolve_external_payloads(source:&Path, files:&[(PathBuf,syn::File)], types:&mut Types) {
    let uses_stake=files.iter().any(|(_,f)|f.items.iter().any(|i|matches!(i,Item::Use(u) if u.to_token_stream().to_string().replace(' ',"")=="useanchor_lang::solana_program::stake::state::StakeState;")));
    if !uses_stake {return}
    let Some(metadata)=dependency_metadata(source) else{return};
    let Some(packages)=metadata["packages"].as_array() else{return};
    let package=|name:&str| -> Option<PathBuf> {
        let matches:Vec<_>=packages.iter().filter(|p|p["name"].as_str()==Some(name)).collect();
        if matches.len()!=1{return None}
        Some(PathBuf::from(matches[0]["manifest_path"].as_str()?).parent()?.to_path_buf())
    };
    let (Some(anchor),Some(program),Some(stake),Some(clock))=(package("anchor-lang"),package("solana-program"),package("solana-stake-interface"),package("solana-clock")) else{return};
    let (Ok(anchor_text),Ok(program_text),Ok(stake_text),Ok(clock_text))=(fs::read_to_string(anchor.join("src/lib.rs")),fs::read_to_string(program.join("src/stake.rs")),fs::read_to_string(stake.join("src/state.rs")),fs::read_to_string(clock.join("src/lib.rs"))) else{return};
    if !anchor_text.replace(' ',"").contains("pubusesolana_program;") || !program_text.contains("solana_stake_interface") || !program_text.contains("state") {return}
    let compact_clock=clock_text.split_whitespace().collect::<String>();
    if !compact_clock.contains("pubtypeEpoch=u64;")||!compact_clock.contains("pubtypeUnixTimestamp=i64;"){return}
    let Ok(file)=syn::parse_file(&stake_text) else{return};
    let mut external=Types::collect(&[(stake.join("src/state.rs"),file)]);
    external.definitions.remove("Pubkey");
    external.definitions.insert("Epoch".into(),Some(Definition::Alias(syn::parse_quote!(u64))));
    external.definitions.insert("UnixTimestamp".into(),Some(Definition::Alias(syn::parse_quote!(i64))));
    if external.independent(&syn::parse_quote!(StakeState),&mut BTreeSet::new()) {
        // Refuse ambiguity with any local declaration, even if another module imports the SDK type.
        if !files.iter().any(|(_,f)|f.items.iter().any(|i|matches!(i,Item::Struct(x) if x.ident=="StakeState")||matches!(i,Item::Enum(x) if x.ident=="StakeState")||matches!(i,Item::Type(x) if x.ident=="StakeState"))) {
            types.definitions.insert("StakeState".into(),Some(Definition::ProvenIndependent));
        }
    }
}

/// Independent local payload observations allow the raw type-check probe.
/// Unknown/generic types stay SDK; real SDK type boundaries are still decided
/// by the existing compile-and-check step after this source scan.
pub fn account_layout_observation(source: &Path) -> Result<Option<String>> {
    fn collect(path:&Path, files:&mut Vec<(PathBuf,syn::File)>)->Result<()> {
        let text=fs::read_to_string(path)?;
        let file=syn::parse_file(&text).with_context(||format!("parse {}",path.file_name().unwrap().to_string_lossy()))?;
        fn modules(items:&[Item],directory:&Path,files:&mut Vec<(PathBuf,syn::File)>)->Result<()> {
            for item in items {
                if let Item::Mod(module)=item {
                    if module.attrs.iter().any(|a|a.path().is_ident("cfg")&&a.to_token_stream().to_string().replace(' ',"")=="#[cfg(test)]") {continue}
                    if let Some((_,inline))=&module.content {modules(inline,&directory.join(module.ident.to_string()),files)?;continue}
                    let explicit=module.attrs.iter().find_map(|a|if a.path().is_ident("path"){if let syn::Meta::NameValue(n)=&a.meta{if let syn::Expr::Lit(e)=&n.value{if let syn::Lit::Str(p)=&e.lit{return Some(directory.join(p.value()))}}}None}else{None});
                    let stem=directory.join(module.ident.to_string());
                    let path=explicit.unwrap_or_else(||if stem.with_extension("rs").exists(){stem.with_extension("rs")}else{stem.join("mod.rs")});
                    if path.exists(){collect(&path,files)?}
                }
            }
            Ok(())
        }
        let directory=if path.file_name().is_some_and(|n|n=="lib.rs"||n=="main.rs"||n=="mod.rs"){path.parent().unwrap().to_path_buf()}else{path.with_extension("")};
        modules(&file.items,&directory,files)?;
        files.push((path.to_path_buf(),file));Ok(())
    }
    let mut files=vec![];
    let root=source.parent().context("program source directory")?;
    collect(source,&mut files)?;
    let mut types = Types::collect(&files);
    resolve_external_payloads(source,&files,&mut types);
    resolve_local_payload_imports(source,&files,&mut types);
    for (path, file) in files {
        // An unrelated module's glob import cannot obscure this module's bindings.
        let mut local=types.clone();
        local.external_glob=Types::collect(&[(path.clone(),file.clone())]).external_glob;
        let mut inspect = Inspect {
            types: &local,
            reason: None,
        };
        inspect.visit_file(&file);
        if let Some(reason) = inspect.reason {
            return Ok(Some(format!(
                "{reason} at {}",
                path.strip_prefix(root).unwrap_or(&path).display()
            )));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sdk(source: &str) -> bool {
        let file = syn::parse_file(source).unwrap();
        let files = vec![(PathBuf::from("lib.rs"), file)];
        let types = Types::collect(&files);
        let mut inspect = Inspect {
            types: &types,
            reason: None,
        };
        inspect.visit_file(&files[0].1);
        inspect.reason.is_some()
    }
    #[test]
    fn payload_layout_is_independent() {
        assert!(!sdk(
            "struct MyAccount { field:u8 } fn f(){let _=core::mem::size_of::<MyAccount>();}"
        ));
        assert!(!sdk("type Key=Pubkey; struct Inner{a:[u64;8],key:Key} struct Outer{v:Option<Vec<Inner>>} fn f(){let _=std::mem::align_of::<Outer>();}"));
    }
    #[test]
    fn account_layout_stays_sdk() {
        assert!(sdk(
            "struct Wrapper<'a>{a:AccountInfo<'a>} fn f(){let _=core::mem::size_of::<Wrapper>();}"
        ));
        assert!(sdk(
            "type Alias<'a>=AccountInfo<'a>; fn f(){let _=core::mem::size_of::<Alias>();}"
        ));
        assert!(sdk("fn f<T>(){let _=core::mem::size_of::<T>();}"));
    }
    #[test]
    fn imported_observer_alias_is_detected() {
        assert!(sdk(
            "use core::mem::size_of as size; fn f(){let _=size::<AccountInfo>();}"
        ));
        assert!(!sdk(
            "use core::mem::size_of as size; struct Data{a:u8} fn f(){let _=size::<Data>();}"
        ));
        assert!(sdk(
            "fn f(){use core::mem::size_of as s;let _=s::<AccountInfo>();}"
        ));
    }
    #[test]
    fn attribute_layout_is_scanned() {
        assert!(!sdk(
            "#[account(space=8+size_of::<Data>())] struct Context{} struct Data{value:u64}"
        ));
        assert!(sdk(
            "#[account(space=8+size_of::<AccountInfo>())] struct Context{}"
        ));
    }
    #[test]
    fn unknown_paths_stay_sdk() {
        assert!(sdk("fn f(){let _=size_of::<external::u8>();}"));
        assert!(sdk(
            "struct Generic<T>{x:T} fn f<T>(){let _=size_of::<Generic<T>>();}"
        ));
        assert!(sdk("fn f(v:&u8){let _=size_of_val(v);}"));
    }
    #[test]
    fn comments_and_strings_are_not_observations() {
        assert!(!sdk("// size_of AccountInfo RefCell
fn f(){let _=\"size_of::<AccountInfo>()\";}"));
    }
}

#[cfg(test)]
mod shadow_tests {
    use super::*;
    fn sdk(s: &str) -> bool {
        let f = syn::parse_file(s).unwrap();
        let types = Types::collect(&[(PathBuf::from("x.rs"), f.clone())]);
        let mut v = Inspect {
            types: &types,
            reason: None,
        };
        v.visit_file(&f);
        v.reason.is_some()
    }
    #[test]
    fn local_known_names_are_not_assumed_builtin() {
        assert!(sdk(
            "type Pubkey=AccountInfo; fn f(){let _=size_of::<Pubkey>();}"
        ));
        assert!(sdk(
            "type Vec<T>=AccountInfo; fn f(){let _=size_of::<Vec<u8>>();}"
        ));
    }
    #[test]
    fn unrelated_qualified_local_name_is_unknown() {
        assert!(sdk(
            "struct Payload{a:u8} fn f(){let _=size_of::<other::Payload>();}"
        ));
    }
    #[test]
    fn refcount_and_borrow_operations_are_conservative() {
        assert!(sdk("fn f(x:Rc<AccountInfo>){let _=Rc::strong_count(&x);}"));
        assert!(sdk("fn f(){let _=RefCell::new(0);}"));
        assert!(sdk(
            "use std::rc::Rc as R;fn f(x:R<u8>){let _=R::strong_count(&x);}"
        ));
    }
}

#[cfg(test)]
mod observer_value_tests {
    use super::*;
    fn sdk(s: &str) -> bool {
        let file = syn::parse_file(s).unwrap();
        let types = Types::collect(&[(PathBuf::from("x.rs"), file.clone())]);
        let mut v = Inspect {
            types: &types,
            reason: None,
        };
        v.visit_file(&file);
        v.reason.is_some()
    }
    #[test]
    fn observer_function_values_are_scanned() {
        assert!(sdk("fn f(){let s=std::mem::size_of::<AccountInfo>;s();}"));
        assert!(!sdk(
            "struct Data{a:u8} fn f(){let s=std::mem::size_of::<Data>;s();}"
        ));
    }
    #[test]
    fn address_macros_are_scanned() {
        assert!(sdk("fn f(x:AccountInfo){let _=core::ptr::addr_of!(x);}"));
    }
    #[test]
    fn generic_payloads_and_cycles_are_unknown() {
        assert!(sdk(
            "struct Empty<T>(std::marker::PhantomData<T>);fn f<T>(){let _=size_of::<Empty<T>>();}"
        ));
        assert!(sdk("type Cycle=Cycle;fn f(){let _=size_of::<Cycle>();}"));
    }
}

#[cfg(test)]
mod import_tests {
    use super::*;
    fn sdk(s: &str) -> bool {
        let f = syn::parse_file(s).unwrap();
        let types = Types::collect(&[(PathBuf::from("x.rs"), f.clone())]);
        let mut v = Inspect {
            types: &types,
            reason: None,
        };
        v.visit_file(&f);
        v.reason.is_some()
    }
    #[test]
    fn local_block_and_module_shadowing_is_unknown() {
        assert!(sdk(
            "fn f(){type Pubkey=AccountInfo;let _=size_of::<Pubkey>();}"
        ));
        assert!(sdk("mod std{pub mod mem{pub type u8=AccountInfo;}}fn f(){let _=size_of::<std::mem::u8>();}"));
    }
    #[test]
    fn external_import_collision_is_unknown() {
        assert!(sdk("mod a{pub struct Payload{value:u8}} mod b{use external::Payload;fn f(){let _=size_of::<Payload>();}}"));
        assert!(sdk(
            "use external::*;struct Payload{value:u8}fn f(){let _=size_of::<Payload>();}"
        ));
        assert!(sdk("use evil::Pubkey;fn f(){let _=size_of::<Pubkey>();}"));
        assert!(sdk(
            "use crate::m::Thing as Pubkey;fn f(){let _=size_of::<Pubkey>();}"
        ));
        assert!(sdk("fn f<String>(){let _=size_of::<String>();}"));
    }
}

#[cfg(test)]
mod resolved_payload_tests {
 use super::*;
 fn sdk(text:&str)->bool{let file=syn::parse_file(text).unwrap();let types=Types::collect(&[(PathBuf::from("lib.rs"),file.clone())]);let mut inspect=Inspect{types:&types,reason:None};inspect.visit_file(&file);inspect.reason.is_some()}
 #[test] fn concrete_self(){assert!(!sdk("struct P([u64;4]);impl P{fn bytes(){let _=size_of::<Self>();}}"));assert!(sdk("struct P(AccountInfo);impl P{fn bytes(){let _=size_of::<Self>();}}"));assert!(sdk("struct P<T>(T);impl<T> P<T>{fn bytes(){let _=size_of::<Self>();}}"));}
 #[test] fn uint_payload(){assert!(!sdk("use uint::construct_uint;construct_uint!{pub struct U256(4);} impl U256{fn f(){let _=size_of::<Self>();}}"));assert!(sdk("macro_rules! construct_uint{()=>{}} construct_uint!{pub struct U256(4);} impl U256{fn f(){let _=size_of::<Self>();}}"));}
}

#[cfg(test)]
mod generic_container_tests {
 use super::*;
 fn sdk(text:&str)->bool{let file=syn::parse_file(text).unwrap();let types=Types::collect(&[(PathBuf::from("lib.rs"),file.clone())]);let mut inspect=Inspect{types:&types,reason:None};inspect.visit_file(&file);inspect.reason.is_some()}
 #[test] fn closed_payload_container(){assert!(!sdk("struct Boxed<T>(Vec<T>); fn f(){let _=size_of::<Boxed<u64>>();}"));assert!(sdk("struct Boxed<T>(Vec<T>);fn f(){let _=size_of::<Boxed<AccountInfo>>();}"));assert!(sdk("struct Boxed<T>(Vec<T>);fn f<T>(){let _=size_of::<Boxed<T>>();}"));}
 #[test] fn local_macro_stays_unknown(){assert!(sdk("use uint::construct_uint; macro_rules! construct_uint{()=>{}} construct_uint!{pub struct U256(4);} impl U256{fn f(){let _=size_of::<Self>();}}"));}
}
