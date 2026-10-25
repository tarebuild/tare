use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

pub struct Plan {
    pub root: PathBuf,
    pub config: PathBuf,
}

pub fn framework(name: &str) -> bool {
    matches!(
        name,
        "anchor-lang"
            | "anchor-spl"
            | "anchor-syn"
            | "anchor-lang-idl"
            | "anchor-lang-idl-spec"
            | "anchor-lang-error"
            | "anchor-client"
    ) || name.starts_with("anchor-attribute-")
        || name.starts_with("anchor-derive-")
}

pub fn prepare(metadata: &Value, toolchain: &Path, out: &Path) -> Result<Plan> {
    fs::create_dir_all(out)?;
    let config_out = out.to_owned();
    let shared = toolchain.join("target/adapters");
    fs::create_dir_all(&shared)?;
    let out = shared.as_path();
    let packages: Vec<&Value> = metadata["packages"]
        .as_array()
        .context("Cargo packages")?
        .iter()
        .filter(|p| framework(p["name"].as_str().unwrap_or("")))
        .collect();
    let mut versions: BTreeMap<&str, &str> = BTreeMap::new();
    for package in &packages {
        let name = package["name"].as_str().unwrap();
        let version = package["version"].as_str().unwrap();
        if matches!(name, "anchor-lang" | "anchor-spl" | "anchor-syn")
            || name.starts_with("anchor-attribute-") || name.starts_with("anchor-derive-")
        {
            if !matches!(version, "0.31.1" | "0.32.1" | "0.32.3" | "1.2.2") {
                bail!("unsupported Anchor version {version} for {name} at {}", package["manifest_path"].as_str().unwrap());
            }
        }
        if let Some(previous) = versions.insert(name, version) {
            if previous != version {
                bail!("unsupported construct mixed Anchor versions for {name}: {previous} and {version}");
            }
        }
    }
    let mut config = toml::map::Map::new();
    let mut patches: BTreeMap<String, toml::map::Map<String, toml::Value>> = BTreeMap::new();
    let macros = fs::read_to_string(toolchain.join("crates/tare-lang-macros/src/lib.rs"))?;
    let graph_identity = resolved_graph_identity(metadata)?;
    let destinations: BTreeMap<String, PathBuf> = packages
        .iter()
        .map(|p| {
            (
                p["name"].as_str().unwrap().to_owned(),
                out.join(format!(
                    "{}-{}-{}",
                    p["name"].as_str().unwrap(),
                    p["version"].as_str().unwrap(),
                    &crate::digest(format!("{}:{}:{graph_identity}",p["name"].as_str().unwrap(),p["version"].as_str().unwrap()).as_bytes())[..8]
                )),
            )
        })
        .collect();
    for (index, package) in packages.iter().enumerate() {
        let name = package["name"].as_str().unwrap();
        let version = package["version"].as_str().unwrap();
        let original = Path::new(package["manifest_path"].as_str().unwrap());
        let original_dir = original.parent().unwrap();
        let dest = destinations.get(name).unwrap().clone();
        fs::create_dir_all(dest.join("src"))?;
        let mut manifest: toml::Value = toml::from_str(&fs::read_to_string(original)?)?;
        expand_workspace(&mut manifest, original_dir)?;
        absolutize_dependencies(&mut manifest, original_dir, &destinations)?;
        version_dependencies(name, version, &mut manifest, metadata)?;
        if name.starts_with("anchor-attribute-") || name.starts_with("anchor-derive-") {
            let syn = destinations
                .get("anchor-syn")
                .context("Anchor syntax adapter")?;
            manifest["dependencies"].as_table_mut().unwrap().insert(
                "anchor-syn".into(),
                toml::Value::Table(toml::map::Map::from_iter([(
                    "path".into(),
                    toml::Value::String(syn.to_string_lossy().replace('\\', "/")),
                )])),
            );
        }

        let table = manifest.as_table_mut().context("package manifest")?;
        table.remove("patch");
        table.remove("replace");
        table.insert("workspace".into(), toml::Value::Table(Default::default()));
        let p = table
            .get_mut("package")
            .and_then(toml::Value::as_table_mut)
            .context("package table")?;
        for field in ["name", "version", "edition"] {
            if let Some(value) = package[field].as_str() {
                p.insert(field.into(), toml::Value::String(value.into()));
            }
        }
        p.remove("workspace");
        p.insert("build".into(), toml::Value::Boolean(false));
        // Preserve stock dependency declarations, adding the runtime compatibility imports above.
        let lib = table
            .entry("lib")
            .or_insert_with(|| toml::Value::Table(Default::default()))
            .as_table_mut()
            .unwrap();
        lib.insert("path".into(), toml::Value::String("src/lib.rs".into()));
        write_changed(dest.join("Cargo.toml"), toml::to_string(&manifest)?)?;
        match name {
            "anchor-lang" => {
                copy_tree(&toolchain.join("crates/tare-lang/src"), &dest.join("src"))?;
                let file = dest.join("src/lib.rs");
                let mut code = fs::read_to_string(&file)?
                    .replace("tare_lang_macros", "anchor_attribute_program");
                if version.starts_with("1.") {
                    code=code.replace("pub use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};", "pub use borsh::{de::BorshDeserialize as AnchorDeserialize, ser::BorshSerialize as AnchorSerialize};\npub use anchor_derive_serde::{AnchorSerialize,AnchorDeserialize,__erase};");
                }
                write_changed(&file, code)?;
                if version.starts_with("1.") {
                    version_runtime(&dest.join("src"), original_dir)?;
                }
            }
            "anchor-syn" => {
                write_changed(dest.join("src/lib.rs"), "pub mod tare;\n")?;
                write_changed(dest.join("src/tare.rs"), version_macros(&macros, version))?;
                let mut helper = fs::read_to_string(
                    toolchain.join("crates/tare-lang-macros/src/declare_program.rs"),
                )?;
                if version.starts_with("1.") {
                    helper = helper.replace("ctx.program.key", "&ctx.program_id");
                    helper = helper.replace("pub use anchor_lang::cpi::Return;", cpi_return_type());
                    helper = helper.replace("Ok(Return::new())", "Ok(Return::new(ctx.program_id))");
                    helper.push_str("\nuse syn_legacy as syn;use heck_legacy as heck;\n");
                }
                write_changed(dest.join("src/declare_program.rs"), helper)?;
            }
            "anchor-lang-idl" | "anchor-lang-idl-spec" => {
                // Pinned protocol representation, also used by the stock IDL producer.
                copy_tree(&original_dir.join("src"), &dest.join("src"))?;
            }
            name if name.starts_with("anchor-attribute-") || name.starts_with("anchor-derive-") => {
                write_changed(
                    dest.join("src/lib.rs"),
                    if name == "anchor-derive-serde" && version.starts_with("1.") {
                        borsh1_wrappers().into()
                    } else {
                        wrappers(name, &macros)
                    },
                )?;
            }
            "anchor-lang-error" => write_changed(
                dest.join("src/lib.rs"),
                "// Error behavior is provided by the Tare anchor-lang runtime.\n",
            )?,
            "anchor-spl" => {
                let source = if version.starts_with("1.") {
                    "crates/tare-spl/versions/v1"
                } else {
                    "crates/tare-spl/src"
                };
                copy_tree(&toolchain.join(source), &dest.join("src"))?;
            }
            _ => write_changed(
                dest.join("src/lib.rs"),
                format!("compile_error!(\"Tare does not implement {name}\");\n"),
            )?,
        }
        let path = dest.to_string_lossy().replace('\\', "/");
        let registry = match package["source"].as_str() {
            None => file_url(original_dir),
            Some(source) if source.starts_with("registry+") => "crates-io".to_owned(),
            Some(source) if source.starts_with("git+") => {
                source[4..].split(['?', '#']).next().unwrap().to_owned()
            }
            Some(source) => bail!("unsupported Cargo source {source}"),
        };
        let replacement = toml::Value::Table(toml::map::Map::from_iter([
            ("package".into(), toml::Value::String(name.into())),
            ("path".into(), toml::Value::String(path)),
        ]));
        patches
            .entry(registry)
            .or_default()
            .insert(format!("{name}_{index}"), replacement);
    }
    if !patches.is_empty() {
        config.insert(
            "patch".into(),
            toml::Value::Table(
                patches
                    .into_iter()
                    .map(|(k, v)| (k, toml::Value::Table(v)))
                    .collect(),
            ),
        );
    }
    let file = config_out.join("overrides.toml");
    write_changed(&file, toml::to_string(&toml::Value::Table(config))?)?;
    Ok(Plan {
        root: out.to_owned(),
        config: file,
    })
}

fn resolved_graph_identity(metadata:&Value)->Result<String> {
    let packages=metadata["packages"].as_array().context("Cargo packages")?;
    let identities:BTreeMap<&str,String>=packages.iter().map(|package| {
        let source=package["source"].as_str().unwrap_or("path");
        (package["id"].as_str().unwrap(),format!("{}@{}:{source}",package["name"].as_str().unwrap(),package["version"].as_str().unwrap()))
    }).collect();
    let mut nodes=Vec::new();
    for node in metadata["resolve"]["nodes"].as_array().context("Cargo resolved nodes")? {
        let id=identities.get(node["id"].as_str().unwrap()).context("resolved package identity")?;
        let mut features=node["features"].as_array().unwrap().iter().map(|f|f.as_str().unwrap()).collect::<Vec<_>>();features.sort_unstable();
        let mut dependencies=node["deps"].as_array().unwrap().iter().map(|dep| {
            let package=identities.get(dep["pkg"].as_str().unwrap()).unwrap();
            format!("{}={package}:{}",dep["name"].as_str().unwrap(),dep["dep_kinds"])
        }).collect::<Vec<_>>();dependencies.sort();
        nodes.push(format!("{id}|{}|{}",features.join(","),dependencies.join(",")));
    }
    nodes.sort();
    Ok(crate::digest(nodes.join("\n").as_bytes()))
}

fn absolutize_dependencies(
    manifest: &mut toml::Value,
    dir: &Path,
    destinations: &BTreeMap<String, PathBuf>,
) -> Result<()> {
    fn dependencies(
        table: &mut toml::map::Map<String, toml::Value>,
        dir: &Path,
        destinations: &BTreeMap<String, PathBuf>,
    ) {
        for kind in ["dependencies", "build-dependencies", "dev-dependencies"] {
            if let Some(deps) = table.get_mut(kind).and_then(toml::Value::as_table_mut) {
                for (name, dependency) in deps.iter_mut() {
                    if let Some(dep) = dependency.as_table_mut() {
                        if let Some(path) = dep.get("path").and_then(toml::Value::as_str) {
                            let path = destinations
                                .get(name)
                                .cloned()
                                .unwrap_or_else(|| dir.join(path).canonicalize().unwrap());
                            let path = path
                                .to_string_lossy()
                                .replace(r"\\?\", "")
                                .replace('\\', "/");
                            dep.insert("path".into(), toml::Value::String(path));
                        }
                    }
                }
            }
        }
    }
    let table = manifest.as_table_mut().context("manifest table")?;
    dependencies(table, dir, destinations);
    if let Some(targets) = table.get_mut("target").and_then(toml::Value::as_table_mut) {
        for (_, target) in targets.iter_mut() {
            if let Some(target) = target.as_table_mut() {
                dependencies(target, dir, destinations);
            }
        }
    }
    Ok(())
}
fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for item in fs::read_dir(from)? {
        let item = item?;
        let path = item.path();
        let dest = to.join(item.file_name());
        if path.is_dir() {
            copy_tree(&path, &dest)?;
        } else {
            write_changed(&dest, fs::read(path)?)?;
        }
    }
    Ok(())
}
fn shared_macros(source: &str) -> String {
    let source = source
        .replace(
            "use proc_macro::TokenStream;",
            "use proc_macro2::TokenStream;",
        )
        .replace("parse_macro_input,", "")
        .replace("syn::parse::<", "syn::parse2::<")
        .replace(".parse(args)", ".parse2(args)")
        .replace(".parse(input)", ".parse2(input)");
    let mut result=String::from("macro_rules! parse_macro_input { ($input:ident as $kind:ty) => { match syn::parse2::<$kind>($input) {Ok(value)=>value,Err(error)=>return error.to_compile_error()} }; ($input:ident with $parser:path) => {match syn::parse::Parser::parse2($parser,$input) {Ok(value)=>value,Err(error)=>return error.to_compile_error()}}; }\n");
    let mut in_attribute = false;
    for line in source.lines() {
        if line.trim_start().starts_with("#[proc_macro") {
            in_attribute = true;
        }
        if in_attribute {
            if line.trim_end().ends_with(']') {
                in_attribute = false;
            }
            continue;
        }
        result.push_str(line);
        result.push('\n');
    }
    result
}
fn wrappers(name: &str, source: &str) -> String {
    let attrs: &[&str] = match name {
        "anchor-attribute-account" => &["account", "zero_copy"],
        "anchor-attribute-access-control" => &["access_control"],
        "anchor-attribute-constant" => &["constant"],
        "anchor-attribute-error" => &["error_code"],
        "anchor-attribute-event" => &["event", "event_cpi"],
        "anchor-attribute-program" => &[
            "program",
            "account",
            "error_code",
            "event",
            "event_cpi",
            "access_control",
            "zero_copy",
            "constant",
            "interface",
            "instruction",
        ],
        _ => &[],
    };
    let mut result = String::from("use proc_macro::TokenStream;\n");
    for attr in attrs {
        if source.contains(&format!("pub fn {attr}(")) {
            result+=&format!("#[proc_macro_attribute] pub fn {attr}(args:TokenStream,input:TokenStream)->TokenStream {{anchor_syn::tare::{attr}(args.into(),input.into()).into()}}\n");
        } else {
            result+=&format!("#[proc_macro_attribute] pub fn {attr}(_:TokenStream,_:TokenStream)->TokenStream {{\"compile_error!(\\\"Tare does not implement #[{attr}]\\\");\".parse().unwrap()}}\n");
        }
    }
    let functions: &[&str] = match name {
        "anchor-attribute-event" => &["emit_cpi"],
        "anchor-attribute-account" => &["declare_id", "pubkey"],
        "anchor-attribute-program" => &["emit_cpi", "declare_program", "declare_id", "pubkey"],
        _ => &[],
    };
    {
        for function in functions {
            if source.contains(&format!("pub fn {function}(")) {
                result+=&format!("#[proc_macro] pub fn {function}(input:TokenStream)->TokenStream {{anchor_syn::tare::{function}(input.into()).into()}}\n");
            }
        }
    }
    let derives: &[(&str, &str, &str)] = match name {
        "anchor-derive-accounts" => &[("Accounts", "accounts", "account,instruction")],
        "anchor-derive-space" => &[("InitSpace", "init_space", "max_len,borsh")],
        "anchor-derive-serde" => &[
            ("AnchorSerialize", "anchor_serialize", "borsh"),
            ("AnchorDeserialize", "anchor_deserialize", "borsh"),
        ],
        "anchor-attribute-program" => &[
            ("Accounts", "accounts", "account,instruction"),
            ("InitSpace", "init_space", "max_len,borsh"),
        ],
        _ => &[],
    };
    for (derive, function, attributes) in derives {
        if source.contains(&format!("pub fn {function}(")) {
            result+=&format!("#[proc_macro_derive({derive},attributes({attributes}))] pub fn {function}(input:TokenStream)->TokenStream {{anchor_syn::tare::{function}(input.into()).into()}}\n");
        } else {
            result+=&format!("#[proc_macro_derive({derive},attributes({attributes}))] pub fn {function}(_:TokenStream)->TokenStream {{\"compile_error!(\\\"Tare does not implement {derive}\\\");\".parse().unwrap()}}\n");
        }
    }
    result
}

pub fn verify(metadata: &Value, plan: &Plan) -> Result<()> {
    let root = plan.root.canonicalize()?;
    for package in metadata["packages"]
        .as_array()
        .context("patched packages")?
    {
        let name = package["name"].as_str().unwrap();
        if framework(name)
            && !Path::new(package["manifest_path"].as_str().unwrap())
                .canonicalize()?
                .starts_with(&root)
        {
            bail!("Cargo selected stock {name} {} at {}; all anchor-* dependencies must resolve to Tare",package["version"],package["manifest_path"]);
        }
    }
    Ok(())
}

fn file_url(path: &Path) -> String {
    let path = path
        .to_string_lossy()
        .replace('\\', "/")
        .replace(' ', "%20");
    if path.starts_with('/') {
        format!("file://{path}")
    } else {
        format!("file:///{path}")
    }
}

fn write_changed(path: impl AsRef<Path>, data: impl AsRef<[u8]>) -> std::io::Result<()> {
    let path = path.as_ref();
    let data = data.as_ref();
    if fs::read(path).ok().as_deref() != Some(data) {
        fs::write(path, data)?;
    }
    Ok(())
}

fn expand_workspace(manifest: &mut toml::Value, dir: &Path) -> Result<()> {
    let mut workspace = None;
    for ancestor in dir.ancestors() {
        let file = ancestor.join("Cargo.toml");
        if file.exists() {
            let doc: toml::Value = toml::from_str(&fs::read_to_string(file)?)?;
            if doc
                .get("workspace")
                .and_then(|w| w.get("dependencies"))
                .is_some()
            {
                workspace = Some((ancestor.to_owned(), doc));
                break;
            }
        }
    }
    let Some((root, doc)) = workspace else {
        return Ok(());
    };
    fn merge(
        table: &mut toml::map::Map<String, toml::Value>,
        root: &Path,
        workspace: &toml::Value,
    ) {
        for kind in ["dependencies", "build-dependencies", "dev-dependencies"] {
            if let Some(deps) = table.get_mut(kind).and_then(toml::Value::as_table_mut) {
                for (name, dep) in deps.iter_mut() {
                    if dep.get("workspace").and_then(toml::Value::as_bool) != Some(true) {
                        continue;
                    }
                    let Some(base) = workspace
                        .get("dependencies")
                        .and_then(|d| d.get(name))
                        .cloned()
                    else {
                        continue;
                    };
                    let mut combined = match base {
                        toml::Value::String(version) => toml::map::Map::from_iter([(
                            "version".into(),
                            toml::Value::String(version),
                        )]),
                        toml::Value::Table(t) => t,
                        _ => continue,
                    };
                    if let Some(path) = combined.get("path").and_then(toml::Value::as_str) {
                        combined.insert(
                            "path".into(),
                            toml::Value::String(
                                root.join(path).to_string_lossy().replace('\\', "/"),
                            ),
                        );
                    }
                    for (k, v) in dep.as_table().unwrap() {
                        if k == "workspace" {
                            continue;
                        }
                        if k == "features" {
                            let current = combined
                                .entry(k.clone())
                                .or_insert_with(|| toml::Value::Array(vec![]))
                                .as_array_mut()
                                .unwrap();
                            current.extend(v.as_array().unwrap().iter().cloned());
                        } else {
                            combined.insert(k.clone(), v.clone());
                        }
                    }
                    *dep = toml::Value::Table(combined);
                }
            }
        }
    }
    let table = manifest.as_table_mut().unwrap();
    if let Some(package) = table.get_mut("package").and_then(toml::Value::as_table_mut) {
        for (name, value) in package.iter_mut() {
            if value.get("workspace").and_then(toml::Value::as_bool) == Some(true) {
                if let Some(resolved) = doc["workspace"].get("package").and_then(|p| p.get(name)) {
                    *value = resolved.clone();
                }
            }
        }
    }
    merge(table, &root, &doc["workspace"]);
    if let Some(targets) = table.get_mut("target").and_then(toml::Value::as_table_mut) {
        for (_, target) in targets.iter_mut() {
            if let Some(t) = target.as_table_mut() {
                merge(t, &root, &doc["workspace"]);
            }
        }
    }
    Ok(())
}
fn version_dependencies(
    name: &str,
    version: &str,
    manifest: &mut toml::Value,
    metadata: &Value,
) -> Result<()> {
    let deps = manifest
        .as_table_mut()
        .unwrap()
        .entry("dependencies")
        .or_insert_with(|| toml::Value::Table(Default::default()))
        .as_table_mut()
        .unwrap();
    if name == "anchor-lang" && !version.starts_with("0.31") {
        let major = if version.starts_with("1.") {
            "3."
        } else {
            "2."
        };
        let stock_program = metadata["packages"]
            .as_array()
            .unwrap()
            .iter()
            .find_map(|package| {
                let selected = package["version"].as_str()?;
                (package["name"] == "solana-program" && selected.starts_with(major))
                    .then_some(selected)
            });
        // A transitive SPL dependency can lock an older minor of this same major.
        // Reuse that exact selection instead of forcing Cargo to upgrade the stock graph.
        let selected = stock_program.unwrap_or(if version.starts_with("1.") {
            "3.0.0"
        } else {
            "2.3.0"
        });
        deps.insert(
            "solana-program".into(),
            toml::Value::String(format!("={selected}")),
        );
        if version.starts_with("1.") {
            let borsh = deps.get_mut("borsh").unwrap();
            let version = borsh.as_str().unwrap_or("1.5.7").to_owned();
            *borsh = toml::Value::Table(toml::map::Map::from_iter([
                ("version".into(), toml::Value::String(version)),
                (
                    "features".into(),
                    toml::Value::Array(vec![toml::Value::String("derive".into())]),
                ),
            ]));
        }
    }
    if name == "anchor-syn" {
        deps.entry("serde_json")
            .or_insert_with(|| toml::Value::String("1".into()));
    }
    if name == "anchor-syn" && version.starts_with("1.") {
        deps.insert(
            "syn-legacy".into(),
            toml::Value::Table(toml::map::Map::from_iter([
                ("package".into(), toml::Value::String("syn".into())),
                ("version".into(), toml::Value::String("1".into())),
                (
                    "features".into(),
                    toml::Value::Array(
                        vec!["full", "extra-traits", "parsing"]
                            .into_iter()
                            .map(|x| toml::Value::String(x.into()))
                            .collect(),
                    ),
                ),
            ])),
        );
        deps.insert(
            "heck-legacy".into(),
            toml::Value::Table(toml::map::Map::from_iter([
                ("package".into(), toml::Value::String("heck".into())),
                ("version".into(), toml::Value::String("0.3".into())),
            ])),
        );
    }
    Ok(())
}
fn version_macros(source: &str, version: &str) -> String {
    let mut source = shared_macros(source);
    if version.starts_with("1.") {
        source = "use syn_legacy as syn; use heck_legacy as heck;\n".to_owned() + &source;
        source = source.replace(
            "const TARE_ANCHOR_MAJOR: u8 = 0;",
            "const TARE_ANCHOR_MAJOR: u8 = 1;",
        );
        source = source.replace(
            "CpiContext::new(__tare_cpi_program,",
            "CpiContext::new(*__tare_cpi_program.key,",
        );
        source = source.replace("pub use anchor_lang::cpi::Return;", cpi_return_type());
        source = source.replace(
            "anchor_lang::cpi::Return<#return_ty>",
            "crate::cpi::Return<#return_ty>",
        );
        source = source.replace(
            "anchor_lang::cpi::Return::new()",
            "crate::cpi::Return::new(ctx.program_id)",
        );
        source = source.replace(
            "CpiContext::new(system_program.to_account_info(),",
            "CpiContext::new(system_program.key(),",
        );
        // Match the stock 1.x handler boundary: narrow invariant AccountInfo
        // lifetimes to the call's local borrow, then release it before exit.
        let helper_marker = "let generated=quote! {";
        assert!(source.contains(helper_marker), "Tare Accounts output changed");
        source = source.replacen(helper_marker, r#"
    let shorten_impl = if input.generics.lifetimes().next().is_some() {
        let long_args = input.generics.params.iter().map(|param| match param {
            syn::GenericParam::Lifetime(_) => quote! { '__info },
            syn::GenericParam::Type(param) => {let ident=&param.ident;quote! {#ident}},
            syn::GenericParam::Const(param) => {let ident=&param.ident;quote! {#ident}},
        }).collect::<Vec<_>>();
        let short_args = input.generics.params.iter().map(|param| match param {
            syn::GenericParam::Lifetime(_) => quote! { '__a },
            syn::GenericParam::Type(param) => {let ident=&param.ident;quote! {#ident}},
            syn::GenericParam::Const(param) => {let ident=&param.ident;quote! {#ident}},
        }).collect::<Vec<_>>();
        quote! {
            impl #bumps_impl_g #name #ty_g #bumps_where_g {
                #[doc(hidden)] #[inline(always)]
                pub unsafe fn __shorten_invariant_lifetime<'__a,'__info:'__a>(value:&'__a mut #name<#(#long_args),*>)->&'__a mut #name<#(#short_args),*> {
                    // Every backing reference outlives this handler borrow.
                    // Context is consumed before the original value is used again.
                    unsafe {::core::mem::transmute(value)}
                }
            }
        }
    } else {
        quote! {
            impl #bumps_impl_g #name #ty_g #bumps_where_g {
                #[doc(hidden)] #[inline(always)]
                pub fn __shorten_invariant_lifetime(value:&mut Self)->&mut Self {value}
            }
        }
    };
    let generated=quote! {
        #shorten_impl
"#, 1);
        for accounts in ["&mut __accounts", "&mut *__accounts"] {
            let old = format!("anchor_lang::Context::new(__program_id, {accounts}, #remaining_slice, __bumps)");
            let new = format!(r#"anchor_lang::Context::new(__program_id,
                unsafe {{ <#accounts_ty>::__shorten_invariant_lifetime({accounts}) }},
                {{
                    #[inline(always)] unsafe fn __tare_shorten_remaining<'a,'info:'a>(value:&'a [anchor_lang::AccountInfo<'info>])->&'a [anchor_lang::AccountInfo<'a>] {{
                        unsafe {{::core::mem::transmute(value)}}
                    }}
                    unsafe {{ __tare_shorten_remaining(#remaining_slice) }}
                }}, __bumps)"#);
            source=source.replace(&old,&new);
        }
        // AccountInfo 3.x exposes resize and always zeroes a grown region.
        source = source.replace("__info.realloc(#space,#zero)?;", "__info.resize(#space)?;");
        // 1.x makes the old IDL account protocol opt-in at the program attribute.
        let program_start = source.find("pub fn program(").expect("Tare program macro");
        let tail = &source[program_start..];
        let old_args = r#"if !args.is_empty() {
        return fail(args.to_string(), "program attribute arguments").into();
    }"#;
        let args = r#"let legacy_idl = {
        let args = match syn::parse::Parser::parse2(
            syn::punctuated::Punctuated::<syn::Ident, syn::Token![,]>::parse_terminated,
            args,
        ) {
            Ok(args) => args,
            Err(error) => return error.to_compile_error(),
        };
        let mut legacy = false;
        for arg in args {
            if arg == "legacy_idl" {
                if legacy {
                    return syn::Error::new_spanned(arg, "Duplicate `legacy_idl` argument").to_compile_error();
                }
                legacy = true;
            } else {
                let message = format!("Invalid argument `{arg}`. Expected one of: `legacy_idl`");
                return syn::Error::new_spanned(arg, message).to_compile_error();
            }
        }
        legacy
    };"#;
        assert!(
            tail.contains(old_args),
            "Tare program argument parser changed"
        );
        source.replace_range(program_start.., &tail.replacen(old_args, args, 1));
        let marker = "if data.starts_with(&0x0a69e9a778bcf440u64.to_le_bytes())";
        assert!(source.contains(marker), "Tare IDL dispatch changed");
        source = source.replace(
            marker,
            "if #legacy_idl && data.starts_with(&0x0a69e9a778bcf440u64.to_le_bytes())",
        );
        source = source.replace(
            "anchor_lang::CpiInvoke::__invoke(&ctx.accounts,ctx.program,__data,ctx.signer_seeds,&ctx.remaining_accounts)?;",
            "anchor_lang::cpi::invoke_dynamic_with_id(&ctx.program_id, &anchor_lang::ToAccountInfos::to_account_infos(&ctx), &anchor_lang::ToAccountMetas::to_account_metas(&ctx,None), __data, ctx.signer_seeds)?;",
        );
    }
    source
}
fn borsh1_wrappers() -> &'static str {
    r#"
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Meta};
#[proc_macro_attribute]
pub fn __erase(_:TokenStream,_:TokenStream)->TokenStream { TokenStream::new() }
fn derive(input:TokenStream, serialize:bool)->TokenStream {
    let mut item = parse_macro_input!(input as DeriveInput);
    let mut options = Vec::<Meta>::new();
    let mut attrs = Vec::new();
    for attr in item.attrs {
        if attr.path().is_ident("borsh") {
            if let Meta::List(list) = attr.meta {
                let parsed = match syn::parse::Parser::parse2(
                    syn::punctuated::Punctuated::<Meta,syn::Token![,]>::parse_terminated,
                    list.tokens,
                ) {
                    Ok(parsed) => parsed,
                    Err(error) => return error.to_compile_error().into(),
                };
                options.extend(parsed);
            }
        } else {
            attrs.push(attr);
        }
    }
    item.attrs = attrs;
    let derive = if serialize {quote!(anchor_lang::borsh::BorshSerialize)}
                 else {quote!(anchor_lang::borsh::BorshDeserialize)};
    quote!(#[derive(#derive)] #[borsh(crate="anchor_lang::borsh", #(#options),*)]
           #[anchor_lang::__erase] #item).into()
}
#[proc_macro_derive(AnchorSerialize,attributes(borsh))]
pub fn anchor_serialize(input:TokenStream)->TokenStream { derive(input,true) }
#[proc_macro_derive(AnchorDeserialize,attributes(borsh))]
pub fn anchor_deserialize(input:TokenStream)->TokenStream { derive(input,false) }
"#
}

fn version_runtime(src: &Path, original: &Path) -> Result<()> {
    // Keep the selected release's mutually-exclusive Solana cohort guards and aliases.
    write_changed(
        src.join("compat.rs"),
        fs::read(original.join("src/compat.rs"))?,
    )?;
    let file = src.join("lib.rs");
    let code = fs::read_to_string(&file)?.replace("\r\n", "\n").replace(
        "pub use solana_program;",
        r#"
pub mod compat;
pub mod solana_program {
    pub use ::solana_program::*;
    pub use crate::compat::{solana_clock as clock, solana_pubkey as pubkey};
    pub use crate::compat::solana_pubkey::{declare_id, pubkey};
    pub use solana_sdk_ids::system_program;
    pub use solana_system_interface::instruction as system_instruction;
    pub mod sysvar {
        pub use crate::compat::solana_sysvar::*;
        pub use solana_sysvar_id::SysvarId;
        pub use crate::compat::solana_stake_interface::stake_history;
        pub use crate::compat::solana_instructions_sysvar as instructions;
    }
    pub mod rent { pub use crate::compat::solana_sysvar::rent::*; }
    pub mod bpf_loader_upgradeable {
        pub use crate::compat::solana_loader_v3_interface::{get_program_data_address,state::UpgradeableLoaderState};
        pub use crate::compat::solana_loader_v3_interface::instruction::*;
        pub use solana_sdk_ids::bpf_loader_upgradeable::{ID,id,check_id};
    }
}
"#,
    );
    assert!(code.contains("const TARE_ANCHOR_MAJOR: u8 = 0;"), "Tare runtime version marker changed");
    // 1.x ties all Context borrows to the account lifetime. Keep the public
    // signature and Debug layout from the selected stock source; no lifetime
    // extension is introduced at the handler boundary.
    let stock_context = fs::read_to_string(original.join("src/context.rs"))?.replace("\r\n", "\n");
    let context_start = stock_context.find("pub struct Context<").context("stock Context definition")?;
    let context_end = stock_context[context_start..].find("/// Context specifying").context("stock CpiContext boundary")? + context_start;
    let start = code.find("pub struct Context<").context("Tare Context definition")?;
    let end = code[start..].find("#[derive(Debug, Clone, Copy)]\npub struct System;").context("Tare Context boundary")? + start;
    let mut code = code;
    code.replace_range(start..end, &stock_context[context_start..context_end].replace("fmt::", "std::fmt::").replace("Accounts<'info, T::Bumps>", "Accounts<'info>"));
    let code=code.replace("const TARE_ANCHOR_MAJOR: u8 = 0;", "const TARE_ANCHOR_MAJOR: u8 = 1;")
        .replace("pub struct Program<'info, T>","pub struct Program<'info, T = ()>")
        .replace("pub use borsh;","pub use borsh;\npub use const_crypto::ed25519::derive_program_address;")
        .replace("type Bumps: Default;", "type Bumps: Default + std::fmt::Debug;")
        .replace("info.realloc(0, false).map_err", "info.resize(0).map_err");
    write_changed(
        &file,
        code.replace("use solana_program::", "use crate::solana_program::"),
    )?;
    fn rust_sources(directory:&Path,paths:&mut Vec<PathBuf>)->Result<()> {
        for entry in fs::read_dir(directory)? {
            let path=entry?.path();
            if path.is_dir() {rust_sources(&path,paths)?;}
            else if path.extension().and_then(|s|s.to_str())==Some("rs") {paths.push(path);}
        }
        Ok(())
    }
    let mut paths=Vec::new();rust_sources(src,&mut paths)?;
    for path in paths {
        if path.extension().and_then(|s| s.to_str()) != Some("rs")
            || (path == src.join("lib.rs") || path == src.join("compat.rs"))
        {
            continue;
        }
        let code = fs::read_to_string(&path)?
            .replace(
                "ProgramError::BorshIoError(value.to_string())",
                "ProgramError::from(value)",
            )
            .replace(".field(\"rent_epoch\",&u64::MAX)", "")
            // AccountInfo 3.x retains the ABI word as `_unused`. Initialize it
            // when constructing the public SDK struct; omitting the write
            // would leave an initialized AccountInfo with an invalid field.
            .replace("addr_of_mut!((*info).rent_epoch)", "addr_of_mut!((*info)._unused)")
            .replace("rent_epoch:info.rent_epoch", "rent_epoch:info._unused")
            .replace("cache.info.realloc(new_len,zero)", "cache.info.resize(new_len)");
        let code = if path.file_name().and_then(|n| n.to_str()) == Some("error.rs") {
            code.replace(
                "pub enum ErrorCode {",
                r#"pub enum ErrorCode {
    #[msg("A duplicate mutable account constraint was violated")]
    ConstraintDuplicateMutableAccount = 2040,
    #[msg("Account is already migrated")]
    AccountAlreadyMigrated = 2041,
    #[msg("Account must be migrated before exiting")]
    AccountNotMigrated = 2042,
    #[msg("A pausable extension constraint was violated")]
    ConstraintMintPausableExtension = 2043,
    #[msg("A pausable extension authority constraint was violated")]
    ConstraintMintPausableAuthority = 2044,
"#,
            )
        } else {
            code
        };
        if code.contains("solana_program::") {
            write_changed(&path, format!("{code}\nuse crate::solana_program;\n"))?;
        }
    }
    let file = src.join("lib.rs");
    let code = fs::read_to_string(&file)?.replace(
        "pub mod __private {",
        &format!("pub mod __private {{\n{}", cpi_return_data()),
    );
    write_changed(&file, code)?;
    let file = src.join("cpi.rs");
    let mut code = fs::read_to_string(&file)?;
    let start = code
        .find("pub struct Return<T>")
        .context("Tare CPI Return definition")?;
    let end = start
        + code[start..]
            .find("pub fn invoke_self")
            .context("Tare self invocation")?;
    code.replace_range(start..end, cpi_return_type());
    write_changed(&file, code)?;
    let context = src.join("context.rs");
    let code = fs::read_to_string(&context)?
        .replace("program: AccountInfo<'info>", "program_id: crate::Pubkey")
        .replace("            program,", "            program_id,")
        .replace("infos.push(self.program);", "")
        .replace("infos.push(self.program.clone());", "");
    write_changed(&context, code)?;
    let file = src.join("sysvar.rs");
    let code = fs::read_to_string(&file)?.replace(
        "solana_program::sysvar::Sysvar",
        "solana_program::sysvar::Sysvar + DecodeSysvar",
    );
    let bounds = r#"
#[cfg(feature="solana-v3")]
pub trait DecodeSysvar: solana_sysvar_v3::SysvarSerialize {}
#[cfg(feature="solana-v3")]
impl<T: solana_sysvar_v3::SysvarSerialize> DecodeSysvar for T {}
#[cfg(feature="solana-v4")]
pub trait DecodeSysvar: solana_sysvar_id::SysvarId + serde::de::DeserializeOwned {}
#[cfg(feature="solana-v4")]
impl<T: solana_sysvar_id::SysvarId + serde::de::DeserializeOwned> DecodeSysvar for T {}
"#;
    write_changed(&file, format!("{bounds}\n{code}"))?;
    Ok(())
}

fn cpi_return_type() -> &'static str {
    r#"
#[derive(Debug,Clone)]
pub struct Return<T> {
    phantom: ::std::marker::PhantomData<T>,
    program_id: anchor_lang::Pubkey,
    return_data: Option<anchor_lang::__private::CpiReturnData>,
}
impl<T> Return<T> {
    pub fn new(program_id:anchor_lang::Pubkey)->Self {
        Self {phantom: ::std::marker::PhantomData,program_id,return_data:anchor_lang::__private::CpiReturnData::snapshot()}
    }
}
impl<T:anchor_lang::AnchorDeserialize> Return<T> {
    pub fn get(&self)->T {self.return_data.as_ref().unwrap().get(self.program_id)}
    pub fn get_unchecked(&self)->T {
        let (_,data)=anchor_lang::solana_program::program::get_return_data().unwrap();
        T::try_from_slice(&data).unwrap()
    }
}
"#
}
fn cpi_return_data() -> &'static str {
    r#"
#[derive(Debug,Clone)]
pub struct CpiReturnData {program_id:crate::Pubkey,data:Vec<u8>}
impl CpiReturnData {
    pub fn new(program_id:crate::Pubkey,data:Vec<u8>)->Self {Self{program_id,data}}
    pub fn snapshot()->Option<Self> {
        let (program_id,data)=crate::solana_program::program::get_return_data()?;
        Some(Self::new(program_id,data))
    }
    pub fn get<T:crate::AnchorDeserialize>(&self,expected_program_id:crate::Pubkey)->T {
        if self.program_id!=expected_program_id {
            crate::solana_program::log::sol_log("CPI return data program_id mismatch");
            panic!();
        }
        T::try_from_slice(&self.data).unwrap()
    }
    pub fn return_data(&self)->(crate::Pubkey,&[u8]) {(self.program_id,&self.data)}
}
"#
}
