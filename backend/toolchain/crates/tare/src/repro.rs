use super::*;

// A compiler wrapper appends individual OsString arguments. Unlike RUSTFLAGS,
// it preserves paths containing spaces and keeps all Cargo/profile flags intact.
pub fn configure(command: &mut Command, source_root: &Path, rehearsal: bool) -> Result<()> {
    let cargo = env::var_os("CARGO_HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env::var_os("USERPROFILE").or_else(|| env::var_os("HOME")).unwrap()).join(".cargo"));
    let rustup = env::var_os("RUSTUP_HOME").map(PathBuf::from).unwrap_or_else(|| cargo.parent().unwrap().join(".rustup"));
    let mut roots = vec![(source_root.to_path_buf(), "/workspace"), (cargo, "/cargo"), (rustup, "/rustup"), (toolchain_root(), "/tare")];
    roots.sort_by_key(|(path, _)| path.as_os_str().len());
    let mut flags = Vec::new();
    for (path, replacement) in roots {
        let original = canonical(&path)?.to_string_lossy().into_owned();
        for from in [original.clone(), original.replace('\\', "/")] {
            flags.push(format!("--remap-path-prefix={from}={replacement}"));
        }
    }
    // This stable flag fingerprints the wrapper's code-generation policy in Cargo.
    let key="CARGO_TARGET_SBPF_SOLANA_SOLANA_RUSTFLAGS";
    let existing=env::var(key).unwrap_or_default();
    command.env(key,format!("{existing} --cfg tare_repro_v5"));
    if rehearsal { flags.extend(["--cfg".into(), "tare_rehearsal_defect".into()]); }
    if command.get_envs().any(|(k,v)| k=="TARE_ACCOUNT_ABI" && v.is_some_and(|v|v=="sdk")) {
        flags.extend(["--cfg".into(), "tare_sdk".into()]);
        command.env(key,format!("{existing} --cfg tare_repro_v5 --cfg tare_sdk"));
        if command.get_envs().any(|(k,v)|k=="TARE_EXTERNAL_ENTRY"&&v.is_some()) {
            flags.extend(["--cfg".into(),"tare_external_entry".into()]);
            command.env(key,format!("{existing} --cfg tare_repro_v5 --cfg tare_sdk --cfg tare_external_entry"));
        }
    }
    if let Some(wrapper) = env::var_os("RUSTC_WRAPPER") { command.env("TARE_UPSTREAM_RUSTC_WRAPPER", wrapper); }
    command.env("TARE_RUSTC_FLAGS", serde_json::to_string(&flags)?);
    command.env("RUSTC_WRAPPER", env::current_exe()?);
    Ok(())
}

pub fn compiler_wrapper() -> Option<i32> {
    let flags = env::var("TARE_RUSTC_FLAGS").ok()?;
    let mut remaps:Vec<String>=serde_json::from_str(&flags).expect("Tare compiler flags");
    if let Ok(dir)=env::var("CARGO_MANIFEST_DIR") {
        if dir.replace('\\',"/").contains("/target/adapters/") {
            let path=Path::new(&dir);let name=path.file_name().unwrap().to_string_lossy();
            let stable=name.rsplit_once('-').map(|(stem,_)|stem).unwrap_or(&name);
            for from in [dir.clone(),dir.replace('\\',"/")] {remaps.push(format!("--remap-path-prefix={from}=/tare/adapters/{stable}"));}
        }
    }
    let args: Vec<_> = env::args_os().skip(1).collect();
    let compiler = args.first()?;
    // Cargo invokes a wrapper with the rustc path as its first argument.
    if !Path::new(compiler).file_stem()?.to_string_lossy().starts_with("rustc") { return None }
    let mut command = if let Some(wrapper) = env::var_os("TARE_UPSTREAM_RUSTC_WRAPPER") {
        let mut command = Command::new(wrapper); command.arg(compiler); command
    } else { Command::new(compiler) };
    let mut index=1;
    while index<args.len() {
        let arg=args[index].to_string_lossy();
        if matches!(arg.as_ref(), "-C"|"--codegen") && args.get(index+1).is_some_and(|next|next.to_string_lossy().starts_with("metadata=")) {
            index+=2;
        } else if arg.starts_with("-Cmetadata=") || arg.starts_with("--codegen=metadata=") {
            index+=1;
        } else {
            command.arg(&args[index]);index+=1;
        }
    }
    if let Some(root)=env::var_os("TARE_REQUIRE_ADAPTER_ROOT") {
        let name=env::var("CARGO_PKG_NAME").unwrap_or_default();
        if adapters::framework(&name) {
            let manifest=env::var_os("CARGO_MANIFEST_DIR").map(PathBuf::from);
            let allowed=manifest.as_ref().and_then(|path|canonical(path).ok()).zip(canonical(Path::new(&root)).ok()).is_some_and(|(path,root)|path.starts_with(root));
            if !allowed {
                eprintln!("Tare refused unpatched Anchor crate {name} from {} (required adapter root {})",manifest.as_ref().map(|p|p.display().to_string()).unwrap_or_else(||"<missing CARGO_MANIFEST_DIR>".into()),Path::new(&root).display());
                return Some(1);
            }
        }
    }
    // rustc accumulates all -C metadata values, so the original Cargo value
    // must be removed, not merely followed by another value.
    // Cargo hashes absolute path dependency IDs into -C metadata. That changes
    // Rust symbol identities and LLVM function order even after filename remap.
    // Keep crate identities dependent on package/version/features/target instead.
    let mut source=env::var("CARGO_MANIFEST_DIR").unwrap_or_default().replace('\\',"/");

    for flag in remaps.iter().rev() {
        if let Some((from,to))=flag.strip_prefix("--remap-path-prefix=").and_then(|s|s.rsplit_once('=')) {
            let from=from.replace('\\',"/");
            if source.starts_with(&from) { source=format!("{to}{}",&source[from.len()..]); break; }
        }
    }
    let mut identity=vec![env::var("CARGO_PKG_NAME").unwrap_or_default(),env::var("CARGO_PKG_VERSION").unwrap_or_default(),source];
    for pair in args.windows(2) {
        if matches!(pair[0].to_str(),Some("--crate-name"|"--crate-type"|"--target"|"--cfg")) {
            identity.push(pair[0].to_string_lossy().into_owned());
            identity.push(pair[1].to_string_lossy().into_owned());
        }
    }
    let stable_metadata=format!("metadata=tare-{}",digest(identity.join("\n").as_bytes()));
    command.arg("-C").arg(&stable_metadata);
    // Host proc-macro crates participate in expansion identities as well.
    command.args(&remaps);
    if let Some(directory)=env::var_os("TARE_REPRO_AUDIT") {
        let directory=PathBuf::from(directory);
        fs::create_dir_all(&directory).expect("create compiler audit directory");
        let record=json!({"package":env::var("CARGO_PKG_NAME").ok(),"manifest":env::var("CARGO_MANIFEST_DIR").ok(),"identity":identity,"stableMetadata":stable_metadata,"compiler":compiler,"args":command.get_args().map(|v|v.to_string_lossy()).collect::<Vec<_>>(),"remaps":remaps});
        let file=directory.join(format!("{}-{}.json",env::var("CARGO_PKG_NAME").unwrap_or_else(|_|"probe".into()),std::process::id()));
        fs::write(file,serde_json::to_vec_pretty(&record).unwrap()).expect("write compiler audit");
    }
    Some(command.status().expect("run Rust compiler").code().unwrap_or(1))
}

// cargo-build-sbf's preliminary cargo_metadata call otherwise ignores the
// command-line patch passed to its final cargo build invocation.
pub fn metadata_proxy() -> Option<i32> {
    let config=env::var_os("TARE_METADATA_CONFIG")?;
    if env::args().nth(1).as_deref()!=Some("metadata") {return None}
    let mut command=Command::new("cargo");
    command.args(env::args_os().skip(1)).arg("--config").arg(config).arg("--locked");
    command.env_remove("TARE_METADATA_CONFIG").env_remove("CARGO");
    Some(command.status().expect("run patched Cargo metadata").code().unwrap_or(1))
}
