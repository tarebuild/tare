//! Source-only account facts omitted from the public IDL.
use crate::{AccountHint, RemainingHint, SourceHints, SplHint};
use anyhow::{bail, Context, Result};
use quote::ToTokens;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
use syn::{GenericArgument, Item, PathArguments, Type};

pub fn source_hints(source: &str) -> Result<SourceHints> {
    hints(
        &syn::parse_file(source)
            .context("parse source for corpus account hints")?
            .items,
    )
}

pub fn source_hints_from_path(path: impl AsRef<Path>) -> Result<SourceHints> {
    let path = path.as_ref();
    let mut file = syn::parse_file(
        &std::fs::read_to_string(path)
            .with_context(|| format!("read source {}", path.display()))?,
    )?;
    load_modules(&mut file.items, path.parent().unwrap_or(Path::new(".")))?;
    hints(&file.items)
}
fn load_modules(items: &mut [Item], directory: &Path) -> Result<()> {
    for item in items {
        let Item::Mod(module) = item else { continue };
        if module.attrs.iter().any(|a| {
            a.path().is_ident("cfg")
                && a.to_token_stream().to_string().replace(' ', "") == "#[cfg(test)]"
        }) {
            continue;
        }
        if let Some((_, items)) = &mut module.content {
            load_modules(items, &directory.join(module.ident.to_string()))?;
            continue;
        }
        let explicit = module.attrs.iter().find_map(|attr| {
            if !attr.path().is_ident("path") {
                return None;
            }
            if let syn::Meta::NameValue(value) = &attr.meta {
                if let syn::Expr::Lit(lit) = &value.value {
                    if let syn::Lit::Str(value) = &lit.lit {
                        return Some(directory.join(value.value()));
                    }
                }
            }
            None
        });
        let name = module.ident.to_string();
        let path = explicit.unwrap_or_else(|| {
            let flat = directory.join(format!("{name}.rs"));
            if flat.exists() {
                flat
            } else {
                directory.join(&name).join("mod.rs")
            }
        });
        let mut file = syn::parse_file(
            &std::fs::read_to_string(&path)
                .with_context(|| format!("read module {name} at {}", path.display()))?,
        )?;
        let parent = path.parent().unwrap();
        let child_directory = if path.file_stem().is_some_and(|s| s == "mod") {
            parent.to_owned()
        } else {
            parent.join(path.file_stem().unwrap())
        };
        load_modules(&mut file.items, &child_directory)?;
        module.content = Some((syn::token::Brace::default(), file.items));
    }
    Ok(())
}
fn hints(items: &[Item]) -> Result<SourceHints> {
    let mut constants = BTreeMap::new();
    fn public_keys(items: &[Item], out: &mut BTreeMap<String, String>) {
        for item in items {
            match item {
                Item::Const(item) => {
                    if let syn::Expr::Macro(value) = &*item.expr {
                        if value
                            .mac
                            .path
                            .segments
                            .last()
                            .is_some_and(|s| s.ident == "pubkey")
                        {
                            if let Ok(text) = syn::parse2::<syn::LitStr>(value.mac.tokens.clone()) {
                                out.insert(item.ident.to_string(), text.value());
                            }
                        }
                    }
                }
                Item::Mod(module) => {
                    if let Some((_, items)) = &module.content {
                        public_keys(items, out)
                    }
                }
                _ => {}
            }
        }
    }
    public_keys(items, &mut constants);
    let mut validation_only=BTreeSet::new();
    fn pure_handlers(items:&[Item],out:&mut BTreeSet<String>){for item in items{match item {
        Item::Fn(f)=>{let body=f.block.to_token_stream().to_string().replace(' ',"");if body=="{Ok(())}"||body=="{returnOk(());}"{out.insert(f.sig.ident.to_string());}},
        Item::Mod(m)=>if let Some((_,items))=&m.content{pure_handlers(items,out)},_=>{}
    }}}
    pure_handlers(items,&mut validation_only);
    let mut spl_operations = BTreeMap::new();
    fn operations(items: &[Item], out: &mut BTreeMap<String, String>) {
        for item in items {
            match item {
                Item::Fn(f) => {
                    let body = f.block.to_token_stream().to_string();
                    if body.contains(":: set_authority") {
                        out.insert(f.sig.ident.to_string(), "set_authority".into());
                    }
                }
                Item::Mod(m) => {
                    if let Some((_, items)) = &m.content {
                        operations(items, out)
                    }
                }
                _ => {}
            }
        }
    }
    operations(items, &mut spl_operations);
    let mut spl_aliases=BTreeMap::new();
    fn aliases(tree:&syn::UseTree,parts:&mut Vec<String>,out:&mut BTreeMap<String,String>){match tree{
        syn::UseTree::Path(p)=>{parts.push(p.ident.to_string());aliases(&p.tree,parts,out);parts.pop();},
        syn::UseTree::Group(g)=>for tree in &g.items{aliases(tree,parts,out)},
        syn::UseTree::Rename(n) if parts.first().is_some_and(|p|p=="anchor_spl")=>{out.insert(n.rename.to_string(),n.ident.to_string());},
        _=>{}
    }}
    fn collect_aliases(items:&[Item],out:&mut BTreeMap<String,String>){for item in items{match item{
        Item::Use(u)=>aliases(&u.tree,&mut Vec::new(),out),
        Item::Mod(m)=>if let Some((_,items))=&m.content{collect_aliases(items,out)},_=>{}
    }}}
    collect_aliases(items,&mut spl_aliases);
    let mut imports = BTreeMap::new();
    let mut local_types = BTreeSet::new();
    fn imported(tree: &syn::UseTree, parts: &mut Vec<String>, out: &mut BTreeMap<String, String>) {
        match tree {
            syn::UseTree::Path(p) => {
                parts.push(p.ident.to_string());
                imported(&p.tree, parts, out);
                parts.pop();
            }
            syn::UseTree::Name(n) => {
                if let Some(owner) = parts.first() {
                    out.insert(n.ident.to_string(), owner.clone());
                }
            }
            syn::UseTree::Rename(n) => {
                if let Some(owner) = parts.first() {
                    out.insert(n.rename.to_string(), owner.clone());
                }
            }
            syn::UseTree::Group(g) => {
                for item in &g.items {
                    imported(item, parts, out)
                }
            }
            _ => {}
        }
    }
    fn declarations(
        items: &[Item],
        imports: &mut BTreeMap<String, String>,
        local: &mut BTreeSet<String>,
    ) {
        for item in items {
            match item {
                Item::Use(u) => imported(&u.tree, &mut Vec::new(), imports),
                Item::Struct(s) => {
                    local.insert(s.ident.to_string());
                }
                Item::Mod(m) => {
                    if let Some((_, items)) = &m.content {
                        declarations(items, imports, local)
                    }
                }
                _ => {}
            }
        }
    }
    declarations(items, &mut imports, &mut local_types);
    let mut structs = BTreeMap::new();
    let mut contexts = Vec::new();
    let mut legacy_idl = false;
    fn visit(
        items: &[Item],
        structs: &mut BTreeMap<String, syn::ItemStruct>,
        contexts: &mut Vec<(String, String)>,
        legacy_idl: &mut bool,
    ) {
        for item in items {
            match item {
                Item::Struct(s)
                    if s.attrs.iter().any(|a| {
                        a.path().is_ident("derive")
                            && a.to_token_stream()
                                .to_string()
                                .split(|c: char| !c.is_alphanumeric() && c != '_')
                                .any(|word| word == "Accounts")
                    }) =>
                {
                    structs.insert(s.ident.to_string(), s.clone());
                }
                Item::Fn(f) => {
                    for arg in &f.sig.inputs {
                        if let syn::FnArg::Typed(arg) = arg {
                            if let Some(name) = generic_type(&arg.ty, "Context") {
                                contexts.push((f.sig.ident.to_string(), name));
                                break;
                            }
                        }
                    }
                }
                Item::Mod(m) => {
                    *legacy_idl |= m.attrs.iter().any(|a| {
                        a.path().is_ident("program")
                            && a.to_token_stream().to_string().contains("legacy_idl")
                    });
                    if let Some((_, items)) = &m.content {
                        visit(items, structs, contexts, legacy_idl)
                    }
                }
                _ => {}
            }
        }
    }
    visit(items, &mut structs, &mut contexts, &mut legacy_idl);
    fn flatten(
        name: &str,
        prefix: &str,
        structs: &BTreeMap<String, syn::ItemStruct>,
        stack: &mut BTreeSet<String>,
        out: &mut BTreeMap<String, AccountHint>,
        imports: &BTreeMap<String, String>,
        local_types: &BTreeSet<String>,
        constants: &BTreeMap<String, String>,
        spl_aliases: &BTreeMap<String,String>,
    ) -> Result<()> {
        if !stack.insert(name.to_owned()) {
            bail!("recursive Accounts composition in {name}")
        }
        if let Some(s) = structs.get(name) {
            for field in &s.fields {
                let Some(ident) = &field.ident else { continue };
                let name = ident.to_string();
                let path = if prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{prefix}.{name}")
                };
                if let Some(nested) = type_name(&field.ty).filter(|n| structs.contains_key(n)) {
                    flatten(
                        &nested,
                        &path,
                        structs,
                        stack,
                        out,
                        imports,
                        local_types,
                        constants,
                        spl_aliases,
                    )?;
                    continue;
                }
                let words: Vec<String> = field
                    .attrs
                    .iter()
                    .filter(|a| a.path().is_ident("account"))
                    .flat_map(|a| {
                        let mut names = Vec::new();
                        let mut first = true;
                        if let syn::Meta::List(list) = &a.meta {
                            for token in list.tokens.clone() {
                                let text = token.to_string();
                                if text == "," {
                                    first = true
                                } else if first {
                                    names.push(text);
                                    first = false
                                }
                            }
                        }
                        names
                    })
                    .collect();
                let hint = AccountHint {
                    init: words.iter().any(|w| w == "init" || w == "init_if_needed"),
                    zero: words.iter().any(|w| w == "zero"),
                    optional: generic_type(&field.ty, "Option").is_some(),
                    account_type: account_type(&field.ty),
                    owner_crate: account_owner(&field.ty, imports, local_types),
                    owner_program: None,
                    spl: spl_hint(&field.ty, &field.attrs, local_types, spl_aliases),
                    executable: is_wrapper(&field.ty, "Program"),
                    program_type: generic_type(&field.ty, "Program"),
                    program_address: explicit_address(&field.attrs, constants),
                    cpi_signer: false,
                    source_account: None,
                    pda: None,
                    writable: false,
                    raw_data_len: None,
                    self_address: false,
                    fixed_address: None,
                    cpi_name: None,
                };
                out.insert(path, hint.clone());
                out.entry(name).or_insert(hint);
            }
        }
        stack.remove(name);
        Ok(())
    }
    let mut instructions = BTreeMap::new();
    let mut scenarios = BTreeMap::new();
    let mut instruction_args = BTreeMap::new();
    let mut event_cpi = BTreeSet::new();
    for (name, context) in contexts {
        if let Some(context) = structs.get(&context) {
            if context
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("event_cpi"))
            {
                event_cpi.insert(name.clone());
            }
            for attribute in &context.attrs {
                if attribute.path().is_ident("instruction") {
                    let args = attribute.parse_args_with(
                        syn::punctuated::Punctuated::<syn::FnArg, syn::Token![,]>::parse_terminated,
                    )?;
                    let names = args
                        .iter()
                        .enumerate()
                        .filter_map(|(i, arg)| {
                            if let syn::FnArg::Typed(arg) = arg {
                                if let syn::Pat::Ident(name) = &*arg.pat {
                                    Some((name.ident.to_string(), i))
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                        .collect();
                    instruction_args.insert(name.clone(), names);
                }
            }
        }

        if structs.contains_key(&context) {
            let mut fields = BTreeMap::new();
            flatten(
                &context,
                "",
                &structs,
                &mut BTreeSet::new(),
                &mut fields,
                &imports,
                &local_types,
                &constants,
                &spl_aliases,
            )?;
            enrich_spl_fields(items, &name, &context, &mut fields);
            scenarios.insert(name.clone(), scenario_hint(items, &name, structs.get(&context).unwrap()));
            instructions.insert(name, fields);
        }
    }
    let mut pubkey_arrays=BTreeSet::new();
    fn accessor_fields(items:&[Item],out:&mut BTreeSet<String>){for item in items{match item{
        Item::Struct(s)=>for field in &s.fields{if field.attrs.iter().any(|a|a.path().is_ident("accessor")&&a.to_token_stream().to_string().replace(' ',"")=="#[accessor(Pubkey)]"){
            if let (Some(name),Type::Array(array))=(&field.ident,&field.ty){if array.elem.to_token_stream().to_string()=="u8"&&array.len.to_token_stream().to_string()=="32"{out.insert(format!("{}.{name}",s.ident));}}
        }},
        Item::Mod(m)=>if let Some((_,items))=&m.content{accessor_fields(items,out)},_=>{}
    }}}
    accessor_fields(items,&mut pubkey_arrays);
    share_spl_roles(&mut instructions);
    Ok(SourceHints {
        scenarios,
        legacy_idl,
        instructions,
        instruction_args,
        spl_operations,
        remaining_accounts: remaining_hints(items, &imports, &local_types, &spl_aliases),
        event_cpi,
        validation_only,
        pubkey_arrays,
    })
}
fn type_name(ty: &Type) -> Option<String> {
    let Type::Path(path) = ty else { return None };
    let last = path.path.segments.last()?;
    if last.ident == "Box" || last.ident == "Option" {
        if let PathArguments::AngleBracketed(args) = &last.arguments {
            for arg in &args.args {
                if let GenericArgument::Type(inner) = arg {
                    return type_name(inner);
                }
            }
        }
    }
    Some(last.ident.to_string())
}
fn generic_type(ty: &Type, wrapper: &str) -> Option<String> {
    let Type::Path(path) = ty else { return None };
    let last = path.path.segments.last()?;
    if last.ident == wrapper {
        if let PathArguments::AngleBracketed(args) = &last.arguments {
            return args
                .args
                .iter()
                .filter_map(|arg| {
                    if let GenericArgument::Type(inner) = arg {
                        type_name(inner)
                    } else {
                        None
                    }
                })
                .last();
        }
        return None;
    }
    if last.ident == "Box" || last.ident == "Option" {
        if let PathArguments::AngleBracketed(args) = &last.arguments {
            for arg in &args.args {
                if let GenericArgument::Type(inner) = arg {
                    return generic_type(inner, wrapper);
                }
            }
        }
    }
    None
}
fn account_type(ty: &Type) -> Option<String> {
    generic_type(ty, "Account")
        .or_else(|| generic_type(ty, "AccountLoader"))
        .or_else(|| generic_type(ty, "InterfaceAccount"))
}

impl SourceHints {
    /// Resolve imported account owners from sibling stock IDL names/addresses.
    pub fn resolve_account_owners(&mut self, programs: &BTreeMap<String, String>) {
        for hint in self
            .remaining_accounts
            .values_mut()
            .flatten()
            .map(|remaining| &mut remaining.account)
        {
            if let Some(owner) = &hint.owner_crate {
                hint.owner_program = programs
                    .iter()
                    .find(|(name, _)| name.replace('-', "_") == owner.replace('-', "_"))
                    .map(|(_, address)| address.clone());
            }
        }
        for fields in self.instructions.values_mut() {
            for hint in fields.values_mut() {
                if hint.program_address.is_none() {
                    if let Some(program) = &hint.program_type {
                        hint.program_address = programs
                            .iter()
                            .find(|(name, _)| crate::normalize(name) == crate::normalize(program))
                            .map(|(_, address)| address.clone());
                    }
                }
                if let Some(owner) = &hint.owner_crate {
                    hint.owner_program = programs
                        .iter()
                        .find(|(name, _)| name.replace('-', "_") == owner.replace('-', "_"))
                        .map(|(_, address)| address.clone());
                }
            }
        }
    }
}

fn account_owner(
    ty: &Type,
    imports: &BTreeMap<String, String>,
    local: &BTreeSet<String>,
) -> Option<String> {
    let Type::Path(path) = ty else { return None };
    let last = path.path.segments.last()?;
    let PathArguments::AngleBracketed(args) = &last.arguments else {
        return None;
    };
    let inner = args
        .args
        .iter()
        .filter_map(|a| {
            if let GenericArgument::Type(t) = a {
                Some(t)
            } else {
                None
            }
        })
        .last()?;
    if last.ident == "Box" || last.ident == "Option" {
        return account_owner(inner, imports, local);
    }
    if !["Account", "AccountLoader", "InterfaceAccount"]
        .iter()
        .any(|n| last.ident == *n)
    {
        return None;
    }
    let Type::Path(inner) = inner else {
        return None;
    };
    let name = inner.path.segments.last()?.ident.to_string();
    let owner = if inner.path.segments.len() > 1 {
        Some(inner.path.segments.first()?.ident.to_string())
    } else if local.contains(&name) {
        None
    } else {
        imports.get(&name).cloned()
    };
    owner.filter(|name| {
        !matches!(
            name.as_str(),
            "crate" | "super" | "self" | "anchor_lang" | "anchor_spl"
        )
    })
}

fn spl_hint(ty: &Type, attrs: &[syn::Attribute], local: &BTreeSet<String>, aliases:&BTreeMap<String,String>) -> Option<SplHint> {
    let original=account_type(ty);
    let imported_alias=original.as_ref().is_some_and(|name|aliases.contains_key(name));
    let name=original.map(|name|aliases.get(&name).cloned().unwrap_or(name));
    let marker = generic_type(ty, "Program").or_else(|| generic_type(ty, "Interface")).map(|name|aliases.get(&name).cloned().unwrap_or(name));
    let kind = match name.as_deref() {
        Some("Mint") if imported_alias || !local.contains("Mint") => "mint",
        Some("TokenAccount") if imported_alias || !local.contains("TokenAccount") => "token",
        _ => match marker.as_deref() {
            Some("Token") => "program-classic",
            Some("Token2022") => "program-2022",
            Some("TokenInterface") => "program-interface",
            Some("AssociatedToken") => "program-associated",
            _ => return None,
        },
    };
    let mut values = BTreeMap::new();
    for attr in attrs.iter().filter(|a| a.path().is_ident("account")) {
        if let syn::Meta::List(list) = &attr.meta {
            let mut item = String::new();
            for token in list.tokens.clone() {
                let t = token.to_string();
                if t == "," {
                    if let Some((key, value)) = item.split_once('=') {
                        values.insert(key.replace(' ', ""), value.replace(' ', ""));
                    }
                    item.clear()
                } else {
                    item.push_str(&t);
                }
            }
            if let Some((key, value)) = item.split_once('=') {
                values.insert(key.replace(' ', ""), value.replace(' ', ""));
            }
        }
    }
    let associated = values.contains_key("associated_token::mint");
    let prefix = if associated {
        "associated_token"
    } else if kind == "mint" {
        "mint"
    } else {
        "token"
    };
    Some(SplHint {
        kind: kind.into(),
        mint: values.get(&format!("{prefix}::mint")).cloned(),
        authority: values.get(&format!("{prefix}::authority")).cloned(),
        token_program: values.get(&format!("{prefix}::token_program")).cloned(),
        decimals: values.get("mint::decimals").and_then(|v| v.parse().ok()),
        associated,
    })
}

fn remaining_hints(
    items: &[Item],
    imports: &BTreeMap<String, String>,
    local: &BTreeSet<String>,
    aliases: &BTreeMap<String,String>,
) -> BTreeMap<String, Vec<RemainingHint>> {
    fn index(
        expr: &syn::Expr,
        variables: &BTreeMap<String, usize>,
        next: &mut usize,
    ) -> Option<usize> {
        match expr {
            syn::Expr::Try(e) => index(&e.expr, variables, next),
            syn::Expr::Reference(e) => index(&e.expr, variables, next),
            syn::Expr::Paren(e) => index(&e.expr, variables, next),
            syn::Expr::Path(p) => p
                .path
                .get_ident()
                .and_then(|n| variables.get(&n.to_string()).copied()),
            syn::Expr::Index(e)
                if e.expr
                    .to_token_stream()
                    .to_string()
                    .contains("remaining_accounts") =>
            {
                if let syn::Expr::Lit(value) = &*e.index {
                    if let syn::Lit::Int(n) = &value.lit {
                        return n.base10_parse().ok();
                    }
                }
                None
            }
            syn::Expr::Call(call) => {
                if call
                    .func
                    .to_token_stream()
                    .to_string()
                    .ends_with("next_account_info")
                {
                    let n = *next;
                    *next += 1;
                    Some(n)
                } else if call
                    .func
                    .to_token_stream()
                    .to_string()
                    .ends_with("try_from")
                {
                    call.args
                        .first()
                        .and_then(|arg| index(arg, variables, next))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    fn converted(expr: &syn::Expr) -> Option<Type> {
        match expr {
            syn::Expr::Try(e) => converted(&e.expr),
            syn::Expr::Paren(e) => converted(&e.expr),
            syn::Expr::Call(call) => {
                let syn::Expr::Path(path) = &*call.func else {
                    return None;
                };
                if path.path.segments.last()?.ident != "try_from" {
                    return None;
                }
                let account = path.path.segments.iter().find(|s| {
                    s.ident == "Account"
                        || s.ident == "AccountLoader"
                        || s.ident == "InterfaceAccount"
                })?;
                let PathArguments::AngleBracketed(args) = &account.arguments else {
                    return None;
                };
                args.args.iter().find_map(|arg| {
                    if let GenericArgument::Type(ty) = arg {
                        Some(ty.clone())
                    } else {
                        None
                    }
                })
            }
            _ => None,
        }
    }
    let mut out = BTreeMap::new();
    for item in items {
        match item {
            Item::Mod(m) => {
                if let Some((_, items)) = &m.content {
                    out.extend(remaining_hints(items, imports, local, aliases))
                }
            }
            Item::Fn(function)
                if function
                    .block
                    .to_token_stream()
                    .to_string()
                    .contains("remaining_accounts") =>
            {
                let mut next = 0;
                let mut variables = BTreeMap::new();
                let mut hints = Vec::new();
                for statement in &function.block.stmts {
                    let syn::Stmt::Local(binding) = statement else {
                        continue;
                    };
                    let syn::Pat::Ident(pattern) = &binding.pat else {
                        continue;
                    };
                    let Some(initializer) = &binding.init else {
                        continue;
                    };
                    let Some(index) = index(&initializer.expr, &variables, &mut next) else {
                        continue;
                    };
                    variables.insert(pattern.ident.to_string(), index);
                    if let Some(ty) = converted(&initializer.expr) {
                        let Some(name) = type_name(&ty) else { continue };
                        let wrapper: Type = syn::parse_quote!(Account<#ty>);
                        hints.push(RemainingHint {
                            index,
                            name: pattern.ident.to_string(),
                            writable: pattern.mutability.is_some(),
                            account: AccountHint {
                                account_type: Some(name),
                                owner_crate: account_owner(&wrapper, imports, local),
                                spl: spl_hint(&wrapper, &[], local, aliases),
                                ..Default::default()
                            },
                        });
                    }
                }
                hints.sort_by_key(|hint| hint.index);
                if !hints.is_empty()
                    && hints
                        .iter()
                        .enumerate()
                        .all(|(index, hint)| hint.index == index)
                {
                    out.insert(function.sig.ident.to_string(), hints);
                }
            }
            _ => {}
        }
    }
    out
}

fn is_wrapper(ty: &Type, wrapper: &str) -> bool {
    let Type::Path(path) = ty else { return false };
    let Some(last) = path.path.segments.last() else {
        return false;
    };
    if last.ident == wrapper {
        return true;
    }
    if last.ident == "Option" || last.ident == "Box" {
        if let PathArguments::AngleBracketed(args) = &last.arguments {
            return args
                .args
                .iter()
                .any(|arg| matches!(arg,GenericArgument::Type(inner)if is_wrapper(inner,wrapper)));
        }
    }
    false
}

fn explicit_address(
    attrs: &[syn::Attribute],
    constants: &BTreeMap<String, String>,
) -> Option<String> {
    for attribute in attrs.iter().filter(|a| a.path().is_ident("account")) {
        if let syn::Meta::List(list) = &attribute.meta {
            let text = list.tokens.to_string();
            for constraint in text.split(',') {
                if let Some((key, value)) = constraint.split_once('=') {
                    if key.trim() == "address" {
                        if let Some(address) = constants.get(value.trim()) {
                            return Some(address.clone());
                        }
                    }
                }
            }
        }
    }
    None
}


// Recover token roles from the ordinary CPI account structs a client mirrors.
fn enrich_spl_fields(items:&[Item], instruction:&str, context:&str, fields:&mut BTreeMap<String,AccountHint>) {
    fn reference(expr:&syn::Expr)->Option<String>{
        let compact=expr.to_token_stream().to_string().replace(' ',"");
        for prefix in ["ctx.accounts.","accounts.","self."] {
            if let Some(start)=compact.find(prefix){
                let suffix=&compact[start+prefix.len()..];
                let name:String=suffix.chars().take_while(|c|c.is_ascii_alphanumeric()||*c=='_').collect();
                if !name.is_empty(){return Some(name)}
            }
        }
        None
    }
    fn fixed_pdas(items:&[Item])->Vec<serde_json::Value>{
        fn constants(items:&[Item],out:&mut BTreeMap<String,Vec<u8>>){for item in items{match item{
            Item::Const(c)=>if let syn::Expr::Lit(v)=&*c.expr{if let syn::Lit::ByteStr(v)=&v.lit{out.insert(c.ident.to_string(),v.value());}},
            Item::Mod(m)=>if let Some((_,items))=&m.content{constants(items,out)},_=>{}
        }}}
        fn byte_value(expr:&syn::Expr,constants:&BTreeMap<String,Vec<u8>>)->Option<Vec<u8>>{match expr{
            syn::Expr::Reference(r)=>byte_value(&r.expr,constants),
            syn::Expr::Lit(v)=>if let syn::Lit::ByteStr(v)=&v.lit{Some(v.value())}else{None},
            syn::Expr::Path(p)=>p.path.get_ident().and_then(|n|constants.get(&n.to_string()).cloned()),_=>None
        }}
        fn block(body:&syn::Block,constants:&BTreeMap<String,Vec<u8>>,out:&mut Vec<serde_json::Value>){
            for statement in &body.stmts {let syn::Stmt::Local(local)=statement else{continue};let Some(init)=&local.init else{continue};let syn::Expr::Call(call)=&*init.expr else{continue};
                if !call.func.to_token_stream().to_string().ends_with("find_program_address"){continue}
                if !call.args.iter().nth(1).is_some_and(|a|a.to_token_stream().to_string().replace(' ',"")=="ctx.program_id"){continue}
                let Some(first)=call.args.first()else{continue};let first=if let syn::Expr::Reference(r)=first{&*r.expr}else{first};let syn::Expr::Array(array)=first else{continue};
                if let Some(seeds)=array.elems.iter().map(|v|byte_value(v,constants)).collect::<Option<Vec<_>>>(){
                    let pda=serde_json::json!({"seeds":seeds.into_iter().map(|value|serde_json::json!({"kind":"const","value":value})).collect::<Vec<_>>()});if !out.contains(&pda){out.push(pda)}
                }
            }
        }
        fn functions(items:&[Item],constants:&BTreeMap<String,Vec<u8>>,out:&mut Vec<serde_json::Value>){for item in items{match item{
            Item::Fn(f)=>block(&f.block,constants,out),Item::Mod(m)=>if let Some((_,items))=&m.content{functions(items,constants,out)},_=>{}
        }}}
        let mut values=BTreeMap::new();constants(items,&mut values);let mut found=Vec::new();functions(items,&values,&mut found);found
    }
    fn source_signers(items:&[Item],context:&str,out:&mut BTreeSet<String>){for item in items{match item{
        Item::Struct(s)if s.ident==context=>for field in &s.fields{if is_wrapper(&field.ty,"Signer")||field.attrs.iter().any(|a|a.path().is_ident("account")&&a.to_token_stream().to_string().split(|c:char|!c.is_ascii_alphanumeric()&&c!='_').any(|w|w=="signer")){if let Some(name)=&field.ident{out.insert(name.to_string());}}},
        Item::Mod(m)=>if let Some((_,items))=&m.content{source_signers(items,context,out)},_=>{}
    }}}
    fn cpi_struct(value:&syn::ExprStruct,direct:bool,signed:bool,fields:&mut BTreeMap<String,AccountHint>,pda:Option<&serde_json::Value>,signers:&BTreeSet<String>){
        let Some(kind)=value.path.segments.last().map(|s|s.ident.to_string())else{return};
        let bindings:BTreeMap<_,_>=value.fields.iter().filter_map(|f|{
            let syn::Member::Named(name)=&f.member else{return None};
            let field=reference(&f.expr).or_else(||{let compact=f.expr.to_token_stream().to_string().replace(' ',"");let local=compact.split('.').next()?;fields.contains_key(local).then(||local.to_owned())});
            field.map(|field|(name.to_string(),field))
        }).collect();
        for (role,name) in &bindings{if let Some(hint)=fields.get_mut(name){if hint.owner_crate.is_some(){hint.cpi_name=Some(role.clone());}}}
        if !matches!(kind.as_str(),"Transfer"|"TransferChecked"|"SetAuthority"|"MintTo"|"MintToChecked"|"Burn"|"BurnChecked"){return}
        let authority=bindings.get("authority").or_else(||bindings.get("current_authority")).cloned();
        if direct&&!signed {if let Some(authority)=&authority {if let Some(hint)=fields.get_mut(authority){hint.cpi_signer=true;}}}
        if let (Some(authority),Some(pda))=(&authority,pda) {if !signers.contains(authority) {if let Some(hint)=fields.get_mut(authority){if !hint.cpi_signer{hint.pda=Some(pda.clone());}}}}
        let mint=bindings.get("mint").cloned();
        for role in ["from","account_or_mint","to"] {
            let Some(name)=bindings.get(role)else{continue};
            let Some(spl)=fields.get_mut(name).and_then(|h|h.spl.as_mut())else{continue};
            if matches!(role,"from"|"account_or_mint")&&spl.authority.is_none(){spl.authority=authority.clone();}
            if spl.mint.is_none(){spl.mint=mint.clone();}
        }
    }
    fn cpi_expr(expr:&syn::Expr,direct:bool,signed:bool,fields:&mut BTreeMap<String,AccountHint>,pda:Option<&serde_json::Value>,signers:&BTreeSet<String>){
        match expr {
            syn::Expr::Struct(value)=>{cpi_struct(value,direct,signed,fields,pda,signers);for field in &value.fields{cpi_expr(&field.expr,direct,signed,fields,pda,signers)}},
            syn::Expr::Call(value)=>for arg in &value.args{cpi_expr(arg,direct,signed,fields,pda,signers)},
            syn::Expr::MethodCall(value)=>{cpi_expr(&value.receiver,direct,signed,fields,pda,signers);for arg in &value.args{cpi_expr(arg,direct,signed,fields,pda,signers)}},
            syn::Expr::Try(value)=>cpi_expr(&value.expr,direct,signed,fields,pda,signers),
            syn::Expr::Reference(value)=>cpi_expr(&value.expr,direct,signed,fields,pda,signers),
            syn::Expr::Paren(value)=>cpi_expr(&value.expr,direct,signed,fields,pda,signers),
            syn::Expr::Group(value)=>cpi_expr(&value.expr,direct,signed,fields,pda,signers),
            _=>{}
        }
    }
    fn apply(body:&syn::Block, direct:bool, fields:&mut BTreeMap<String,AccountHint>,pda:Option<&serde_json::Value>,signers:&BTreeSet<String>){
        let signed=body.to_token_stream().to_string().contains("with_signer") || body.to_token_stream().to_string().contains("new_with_signer");
        for statement in &body.stmts {
            if direct {if let syn::Stmt::Expr(syn::Expr::Assign(assign),_)=statement {
                if let syn::Expr::Index(index)=&*assign.left {
                    let text=index.expr.to_token_stream().to_string().replace(' ',"");
                    if text.contains(".data.borrow_mut()") {if let (Some(name),syn::Expr::Lit(value))=(reference(&index.expr),&*index.index) {if let syn::Lit::Int(value)=&value.lit {if let Ok(index)=value.base10_parse::<usize>() {if let Some(hint)=fields.get_mut(&name){hint.raw_data_len=Some(hint.raw_data_len.unwrap_or(0).max(index+1));hint.writable=true;}}}}}
                }
            }}
            match statement {
                syn::Stmt::Local(local)=>if let Some(init)=&local.init{cpi_expr(&init.expr,direct,signed,fields,pda,signers)},
                syn::Stmt::Expr(expr,_)=>cpi_expr(expr,direct,signed,fields,pda,signers),
                _=>{}
            }
        }
    }
    fn scan(items:&[Item],instruction:&str,context:&str,fields:&mut BTreeMap<String,AccountHint>,pda:Option<&serde_json::Value>,signers:&BTreeSet<String>){
        for item in items {match item{
            Item::Fn(f) if f.sig.ident==instruction => apply(&f.block,true,fields,pda,signers),
            Item::Impl(i) => {
                for item in &i.items {if let syn::ImplItem::Fn(f)=item {
                    let argument=f.sig.inputs.to_token_stream().to_string();
                    let has_context=argument.split(|c:char|!c.is_ascii_alphanumeric()&&c!='_').any(|part|part==context);
                    if type_name(&i.self_ty).as_deref()==Some(context)||has_context {apply(&f.block,false,fields,pda,signers);}
                }}
            }
            Item::Mod(m)=>if let Some((_,items))=&m.content{scan(items,instruction,context,fields,pda,signers)},
            _=>{}
        }}
    }
    let pdas=fixed_pdas(items);let mut signers=BTreeSet::new();source_signers(items,context,&mut signers);
    scan(items,instruction,context,fields,(pdas.len()==1).then(||&pdas[0]),&signers);
    let mut assignments=BTreeMap::new();
    fn assigned(items:&[Item],out:&mut BTreeMap<String,String>){
        for item in items {match item {
            Item::Fn(f)=>for statement in &f.block.stmts {
                if let syn::Stmt::Expr(syn::Expr::Assign(a),_)=statement {
                    let left=a.left.to_token_stream().to_string().replace(' ',"");
                    if let Some((_,field))=left.rsplit_once('.') {if let Some(account)=reference(&a.right){out.insert(field.to_string(),account);}}
                }
            },
            Item::Mod(m)=>if let Some((_,items))=&m.content{assigned(items,out)},_=>{}
        }}
    }
    assigned(items,&mut assignments);
    fn aliases(items:&[Item],context:&str,assignments:&BTreeMap<String,String>,fields:&mut BTreeMap<String,AccountHint>){
        for item in items {match item {
            Item::Struct(s) if s.ident==context => for field in &s.fields {for attribute in &field.attrs {
                if !attribute.path().is_ident("account"){continue}
                let text=attribute.to_token_stream().to_string().replace(' ',"");
                for constraint in text.split(',') {let Some((left,right))=constraint.split_once("==")else{continue};
                    let left=left.trim_start_matches("constraint=");let Some((_,stored))=left.rsplit_once('.')else{continue};
                    let target=right.trim_start_matches(['*','&']).split('.').next().unwrap_or("");
                    if let Some(source)=assignments.get(stored){if source!=target {if let Some(hint)=fields.get_mut(target){hint.source_account=Some(source.clone());}}}
                }
            }},
            Item::Mod(m)=>if let Some((_,items))=&m.content{aliases(items,context,assignments,fields)},_=>{}
        }}
    }
    aliases(items,context,&assignments,fields);
    // Access-control methods sometimes compare a stored authority field to a
    // differently named signer. Use the first explicit authority comparison
    // as one valid fixture scenario; other argument/error variants remain.
    fn control_authorities(items:&[Item],context:&str,assignments:&BTreeMap<String,String>,fields:&mut BTreeMap<String,AccountHint>){
        fn block(block:&syn::Block,assignments:&BTreeMap<String,String>,fields:&mut BTreeMap<String,AccountHint>){for statement in &block.stmts{
            let syn::Stmt::Expr(syn::Expr::If(branch),_)=statement else{continue};
            if let syn::Expr::Binary(binary)=&*branch.cond {if matches!(binary.op,syn::BinOp::Eq(_)|syn::BinOp::Ne(_)){
                let left=binary.left.to_token_stream().to_string().replace(' ',"");
                let right=binary.right.to_token_stream().to_string().replace(' ',"");
                if left.starts_with("ctx.accounts.")&&right.contains(".key") {
                    if let (Some(stored),Some(target))=(left.rsplit('.').next(),reference(&binary.right)){
                        let source=assignments.get(stored).cloned().unwrap_or_else(||stored.into());
                        if let Some(hint)=fields.get_mut(&target){if hint.source_account.is_none(){hint.source_account=Some(source);}}
                    }
                }
            }}
        }}
        for item in items{match item{
            Item::Impl(value)if type_name(&value.self_ty).as_deref()==Some(context)=>for item in &value.items{if let syn::ImplItem::Fn(function)=item{block(&function.block,assignments,fields)}},
            Item::Mod(value)=>if let Some((_,items))=&value.content{control_authorities(items,context,assignments,fields)},_=>{}
        }}
    }
    control_authorities(items,context,&assignments,fields);

    // The legacy Anchor examples pass token_program as AccountInfo. The CPI
    // module is the source of its concrete program ID when no wrapper supplies it.
    let text=items.iter().map(ToTokens::to_token_stream).map(|s|s.to_string()).collect::<String>();
    let kind=if text.contains("token_interface") {"program-interface"} else {"program-classic"};
    if fields.values().any(|h|h.spl.as_ref().is_some_and(|s|s.kind=="token"||s.kind=="mint")) {
        if let Some(field)=fields.get_mut("token_program"){if field.spl.is_none(){field.spl=Some(SplHint{kind:kind.into(),..Default::default()});}}
    }
    // has_one and the common raw owner comparison express SPL authority roles.
    fn constraints(items:&[Item],context:&str,fields:&mut BTreeMap<String,AccountHint>){
        for item in items {match item {
            Item::Struct(s) if s.ident==context=>for f in &s.fields {
                let Some(name)=&f.ident else{continue};let name=name.to_string();
                let names:Vec<_>=fields.keys().cloned().collect();
                let Some(spl)=fields.get_mut(&name).and_then(|h|h.spl.as_mut()) else{continue};
                for a in f.attrs.iter().filter(|a|a.path().is_ident("account")){
                    let compact=a.to_token_stream().to_string().replace(' ',"");
                    if compact.contains("has_one=owner"){spl.authority=Some("owner".into());}
                    for other in names.iter().cloned() {
                        if compact.contains(&format!("{name}.owner=={other}.key")){spl.authority=Some(other);}
                    }
                }
            },
            Item::Mod(m)=>if let Some((_,items))=&m.content{constraints(items,context,fields)},
            _=>{}
        }}
    }
    constraints(items,context,fields);
    fn close_targets(items:&[Item],context:&str,fields:&mut BTreeMap<String,AccountHint>){for item in items{match item{
        Item::Struct(s)if s.ident==context=>for field in &s.fields{for a in field.attrs.iter().filter(|a|a.path().is_ident("account")){
            let text=a.to_token_stream().to_string().replace(' ',"");
            if let Some((_,rest))=text.split_once("close="){let target:String=rest.chars().take_while(|c|c.is_ascii_alphanumeric()||*c=='_').collect();if let Some(hint)=fields.get_mut(&target){hint.writable=true;}}
        }},
        Item::Mod(m)=>if let Some((_,items))=&m.content{close_targets(items,context,fields)},_=>{}
    }}}
    close_targets(items,context,fields);
    fn equal_program(items:&[Item],instruction:&str,context:&str,fields:&mut BTreeMap<String,AccountHint>){
        for item in items {match item {
            Item::Fn(f) if f.sig.ident==instruction => {
                let text=f.block.to_token_stream().to_string().replace(' ',"");
                for (name,hint) in fields.iter_mut() {
                    if text.contains(&format!("require_keys_eq!(ctx.accounts.{name}.key(),*ctx.program_id")) {hint.self_address=true;}
                }
            },
            Item::Struct(s)if s.ident==context=>for field in &s.fields {let Some(name)=&field.ident else{continue};let name=name.to_string();
                for attribute in field.attrs.iter().filter(|a|a.path().is_ident("account")) {let text=attribute.to_token_stream().to_string().replace(' ',"");
                    if text.contains(&format!("constraint=*{name}.key==ID"))||text.contains(&format!("constraint={name}.key()==ID")){if let Some(hint)=fields.get_mut(&name){hint.self_address=true;}}
                }
            },
            Item::Mod(m)=>if let Some((_,items))=&m.content{equal_program(items,instruction,context,fields)},_=>{}
        }}
    }
    equal_program(items,instruction,context,fields);
    // A zero account's Pubkey fields are Pubkey::default(). Keep these
    // per-instruction bindings local: the same name may be a signer elsewhere.
    fn constraint_addresses(items:&[Item],context:&str,fields:&mut BTreeMap<String,AccountHint>){for item in items{match item{
        Item::Struct(s)if s.ident==context=>for field in &s.fields{
            let Some(name)=field.ident.as_ref().map(ToString::to_string)else{continue};
            let zero=fields.get(&name).is_some_and(|h|h.zero);
            for attribute in field.attrs.iter().filter(|a|a.path().is_ident("account")){
                let syn::Meta::List(list)=&attribute.meta else{continue};
                let text=list.tokens.to_string().replace(' ',"");
                for constraint in text.split(','){
                    if zero {if let Some(target)=constraint.strip_prefix("has_one="){
                        let target=target.split('@').next().unwrap_or(target);
                        if let Some(hint)=fields.get_mut(target){hint.fixed_address=Some("11111111111111111111111111111111".into());}
                    }}
                    if let Some(value)=constraint.strip_prefix("address="){
                        if matches!(value,"crate::ID"|"crate::id()"|"ID"|"id()") {if let Some(hint)=fields.get_mut(&name){hint.self_address=true;}}
                        else if let Some((source,stored))=value.split_once('.'){
                            let stored=stored.trim_end_matches("()");
                            if fields.get(source).is_some_and(|h|h.account_type.is_some())&&stored.chars().all(|c|c.is_ascii_alphanumeric()||c=='_'){
                                if let Some(hint)=fields.get_mut(&name){hint.source_account=Some(stored.into());}
                            }
                        }
                    }
                }
            }
        },
        Item::Mod(m)=>if let Some((_,items))=&m.content{constraint_addresses(items,context,fields)},_=>{}
    }}}
    constraint_addresses(items,context,fields);
}


fn share_spl_roles(instructions:&mut BTreeMap<String,BTreeMap<String,AccountHint>>) {
    let mut roles:BTreeMap<(String,&str),BTreeSet<String>>=BTreeMap::new();
    for fields in instructions.values(){for (name,hint) in fields{if let Some(spl)=&hint.spl{
        for (role,value) in [("authority",&spl.authority),("mint",&spl.mint),("token_program",&spl.token_program)] {
            if let Some(value)=value{roles.entry((hint.source_account.clone().unwrap_or_else(||name.clone()),role)).or_default().insert(value.clone());}
        }
    }}}
    for fields in instructions.values_mut(){for (name,hint) in fields{if let Some(spl)=&mut hint.spl{
        for (role,value) in [("authority",&mut spl.authority),("mint",&mut spl.mint),("token_program",&mut spl.token_program)] {
            if value.is_none(){if let Some(values)=roles.get(&(hint.source_account.clone().unwrap_or_else(||name.clone()),role)){if values.len()==1{*value=values.first().cloned();}}}
        }
    }}}
}


impl SourceHints {
    /// Import a unique account PDA definition from the stock IDL of its owner.
    /// Caller account names must cover every seed account; ambiguous definitions
    /// and argument/data-dependent seeds are left unresolved.
    pub fn resolve_external_account_pdas(&mut self,idls:&[serde_json::Value]) -> Result<()> {
        for fields in self.instructions.values_mut() {
            let available:BTreeSet<_>=fields.keys().cloned().collect();
            for (name,hint) in fields.iter_mut() {
                let Some(owner)=&hint.owner_crate else{continue};
                let Some(idl)=idls.iter().find(|idl|idl["metadata"]["name"].as_str().or_else(||idl["name"].as_str()).is_some_and(|name|crate::normalize(name)==crate::normalize(owner)))else{continue};
                let mut candidates=Vec::new();
                for instruction in crate::array(idl,"instructions")? {for account in crate::flat_accounts(instruction)? {
                    if account["name"].as_str()!=Some(hint.cpi_name.as_deref().unwrap_or(name))||!account["pda"].is_object(){continue}
                    let mut pda=account["pda"].clone();
                    let valid=pda["seeds"].as_array().is_some_and(|seeds|seeds.iter().all(|s|match s["kind"].as_str(){Some("const")=>true,Some("account")=>s["path"].as_str().is_some_and(|name|!name.contains('.')&&available.contains(name)),_=>false}));
                    if !valid{continue}
                    if pda.get("program").is_none(){let address:crate::Address=crate::required_str(idl,"address")?.parse()?;pda["program"]=serde_json::json!({"kind":"const","value":address.to_bytes().to_vec()});}
                    if !candidates.contains(&pda){candidates.push(pda);}
                }}
                if candidates.len()==1 {hint.pda=candidates.pop();}
            }
        }
        Ok(())
    }
}

// Client scenario facts are separate from the original corpus: they produce
// additional independent inputs, preserving every previous sequence unchanged.
fn scenario_hint(items:&[Item],instruction:&str,context:&syn::ItemStruct)->crate::ScenarioHint {
    let mut out=crate::ScenarioHint::default();
    fn strip(value:&str)->&str {value.trim_start_matches(['*','&'])}
    for field in &context.fields {
        let Some(name)=field.ident.as_ref().map(ToString::to_string)else{continue};
        for attribute in field.attrs.iter().filter(|a|a.path().is_ident("account")) {
            let syn::Meta::List(list)=&attribute.meta else{continue};
            let mut constraints=Vec::new(); let mut current=String::new();
            for token in list.tokens.clone() {
                if token.to_string()=="," {
                    constraints.push(current.replace(' ',"")); current=String::new();
                } else {current.push_str(&token.to_string());}
            }
            constraints.push(current.to_string().replace(' ',""));
            for raw in &constraints {
                let text=raw.split('@').next().unwrap_or(raw);
                if let Some(payer)=text.strip_prefix("payer="){out.payers.insert(payer.to_owned());}
                if let Some(space)=text.strip_prefix("space=").and_then(|v|v.parse().ok()){out.spaces.insert(name.clone(),space);}
                if let Some(owner)=text.strip_prefix("owner="){let owner=strip(owner).split('.').next().unwrap_or(owner);if context.fields.iter().any(|f|f.ident.as_ref().is_some_and(|id|id==owner)){out.owners.insert(name.clone(),owner.into());}}
                if let Some(value)=text.strip_prefix("mint::decimals="){if value.parse::<u8>().is_err(){out.mint_decimals.insert(name.clone(),value.into());}}
                if let Some(value)=text.strip_prefix("mint::freeze_authority="){out.mint_freeze_authorities.insert(name.clone(),value.into());}
                if let Some(value)=text.strip_prefix("bump="){if value.chars().all(|c|c.is_ascii_alphanumeric()||c=='_'){out.bumps.insert(name.clone(),value.into());}}
                if let Some(value)=text.strip_prefix("seeds=") {if let Ok(syn::Expr::Array(array))=syn::parse_str::<syn::Expr>(value){let seeds:Option<Vec<_>>=array.elems.iter().map(|seed|{let text=seed.to_token_stream().to_string().replace(' ',"");let name=text.trim_start_matches("&[").trim_end_matches(']');(!name.is_empty()&&name.chars().all(|c|c.is_ascii_alphanumeric()||c=='_')).then(||serde_json::json!({"kind":"arg","path":name}))}).collect();if let Some(seeds)=seeds {out.seed_overrides.insert(name.clone(),serde_json::json!({"seeds":seeds}));}}}
                if let Some(eq)=text.strip_prefix("constraint="){if let Some((left,right))=eq.split_once("==") {
                    if let Ok(n)=left.parse::<u64>(){if right.chars().all(|c|c.is_ascii_alphanumeric()||c=='_'){out.arguments.insert(right.into(),n);}}
                    else if let Ok(n)=right.parse::<u64>(){if left.chars().all(|c|c.is_ascii_alphanumeric()||c=='_'){out.arguments.insert(left.into(),n);}}
                }}
            }
        }
    }
    fn inspect(items:&[Item], instruction:&str, out:&mut crate::ScenarioHint) {
        for item in items {
            match item {
                Item::Fn(f) if f.sig.ident == instruction => {
                    let body=f.block.to_token_stream().to_string().replace(' ', "");
                    for part in body.split(".close(").skip(1) {
                        let Some(target)=part.split(')').next() else {continue};
                        for statement in &f.block.stmts {
                            let syn::Stmt::Local(local)=statement else {continue};
                            let (syn::Pat::Ident(id),Some(init))=(&local.pat,&local.init) else {continue};
                            if id.ident!=target {continue}
                            let text=init.expr.to_token_stream().to_string().replace(' ', "");
                            if let Some(rest)=text.strip_prefix("ctx.accounts.") {out.writable.insert(rest.split('.').next().unwrap_or(rest).into());}
                        }
                    }
                    for field in body.split("ctx.accounts.").skip(1) {
                        let name:String=field.chars().take_while(|c|c.is_ascii_alphanumeric()||*c=='_').collect();
                        if let Some((_,rest))=field.split_once("try_borrow_mut_lamports()?-=") {
                            if let Ok(n)=rest.split(';').next().unwrap_or(rest).parse() {out.initial_lamport_debits.insert(name,n);}
                        }
                    }
                }
                Item::Mod(m)=>if let Some((_,items))=&m.content {inspect(items,instruction,out)},
                _=>{}
            }
        }
    }
    inspect(items,instruction,&mut out);out
}
