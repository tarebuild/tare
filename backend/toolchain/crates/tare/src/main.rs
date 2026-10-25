mod account_abi;
mod adapters;
mod chain;
mod container_paths;
mod repro;
mod wire;
mod stock_idl;
use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

static VERBOSE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
fn verbose() -> bool { VERBOSE.load(std::sync::atomic::Ordering::Relaxed) }

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(
    name = "tare",
    version,
    about = "Build and weigh unchanged Anchor programs"
)]
struct Cli {
    #[arg(long, global = true)]
    verbose: bool,
    #[command(subcommand)]
    command: Action,
}
#[derive(Subcommand)]
enum Action {
    Build(Workspace),
    Weigh(Workspace),
    Prove(ProofArgs),
    Attest(chain::AttestArgs),
    Check(chain::CheckArgs),
    Flag(chain::FlagArgs),
    #[command(name = "__idl", hide = true)]
    Idl(IdlArgs),
}
#[derive(Args)]
struct IdlArgs {
    #[arg(long)] manifest: PathBuf,
    #[arg(long)] library: String,
    #[arg(long)] version: String,
    #[arg(long)] output: PathBuf,
    #[arg(long, value_delimiter = ',')] features: Vec<String>,
}
#[derive(Args, Clone)]
struct Workspace {
    #[arg(long)]
    host: bool,
    #[arg(long, value_delimiter = ',')]
    features: Vec<String>,
    #[arg(long)]
    idl: Option<PathBuf>,
    #[arg(long, default_value = "Cargo.toml")]
    manifest_path: PathBuf,
    #[arg(long)]
    tests_corpus: Option<PathBuf>,
    #[arg(long)]
    replay: Option<PathBuf>,
    #[arg(long)]
    mainnet_corpus: Option<PathBuf>,
}
#[derive(Args)]
struct ProofArgs {
    #[command(flatten)]
    workspace: Workspace,
    #[arg(long)]
    program: Option<String>,
    #[arg(long, default_value_t=3, value_parser=clap::value_parser!(u32).range(1..=20))]
    samples: u32,
    #[arg(long, env="TARE_MAINNET_RPC_URL")]
    url: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildRecord {
    schema_version: String,
    #[serde(default)]
    toolchain_version: String,
    name: String,
    anchor_version: String,
    manifest_path: PathBuf,
    workspace_root: PathBuf,
    source_path: PathBuf,
    stock: PathBuf,
    tare: PathBuf,
    idl: PathBuf,
    stock_hash: String,
    tare_hash: String,
    source: Value,
    architecture: String,
    platform_tools: String,
    #[serde(default)]
    optimization: Value,
    #[serde(default)]
    programs: Vec<ExternalProgram>,
    #[serde(default)]
    features: Vec<String>,
    #[serde(default)]
    rehearsal_defect: Option<String>,
    #[serde(default)]
    reproducibility: String,
    #[serde(default)]
    build_environment: Value,
    #[serde(default)]
    account_abi: Value,
}
#[derive(Serialize, Deserialize)]
struct ExternalProgram {
    address: String,
    path: PathBuf,
    #[serde(default)]
    idl: Option<PathBuf>,
}
struct Package {
    name: String,
    lib_name: String,
    manifest: PathBuf,
    root: PathBuf,
    source: PathBuf,
    anchor_version: String,
}

fn main() {
    if let Some(code) = repro::metadata_proxy() { std::process::exit(code); }
    if let Some(code) = repro::compiler_wrapper() { std::process::exit(code); }
    if let Err(error) = execute(Cli::parse()) {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}
fn execute(cli: Cli) -> Result<()> {
    VERBOSE.store(cli.verbose, std::sync::atomic::Ordering::Relaxed);
    match cli.command {
        Action::Build(workspace) => {
            if container(&workspace, "build")? { return Ok(()) }
            build(&workspace)
        }
        Action::Weigh(workspace) => measure(&workspace, false),
        Action::Prove(args) => {
            if let Some(program) = args.program {
                if container_request(&args.workspace,"prove",rehearsal_defect()?,Some((&program,args.samples,args.url.as_deref())))? { return Ok(()) }
                chain::prove_mainnet(&args.workspace, &program, args.samples, args.url.as_deref())
            } else { measure(&args.workspace, true) }
        }
        Action::Attest(args) => chain::attest(args),
        Action::Check(args) => chain::check(args),
        Action::Flag(args) => chain::flag(args),
        Action::Idl(args) => stock_idl_inline(&args).map(|_| ()),
    }
}
fn container(workspace: &Workspace, verb: &str) -> Result<bool> {
    container_with_defect(workspace,verb,rehearsal_defect()?)
}
fn container_with_defect(workspace: &Workspace, verb: &str, defect:Option<String>) -> Result<bool> {
    container_request(workspace,verb,defect,None)
}
fn container_request(workspace: &Workspace, verb: &str, defect:Option<String>, mainnet:Option<(&str,u32,Option<&str>)>) -> Result<bool> {
    if workspace.host || env::var_os("TARE_IN_CONTAINER").is_some() {
        return Ok(false);
    }
    let image = env::var("TARE_IMAGE").unwrap_or_else(|_| format!("tare-toolchain:{VERSION}"));
    let manifest = canonical(&workspace.manifest_path)?;
    let package = discover(&manifest)?;
    let source_root = git(&package.root, &["rev-parse", "--show-toplevel"])
        .map(PathBuf::from)
        .unwrap_or_else(|| package.root.clone());
    let paths = container_paths::map(&source_root.to_string_lossy(), &package.root.to_string_lossy(), &manifest.to_string_lossy())?;
    let mount = format!("{}:/workspace", source_root.display());
    let mut command = Command::new("docker");
    command
        .args([
            "run",
            "--rm",
            "-e",
            "TARE_IN_CONTAINER=1",
            "-v",
            &mount,
            "-w",
        ])
        .arg(&paths.workdir);
    let mut forwarded=Vec::new();
    for (name,path) in [("tests-corpus",&workspace.tests_corpus),("replay",&workspace.replay),("mainnet-corpus",&workspace.mainnet_corpus),("idl",&workspace.idl)] {
        if let Some(path)=path {
            let path=canonical(path)?;let destination=format!("/tare-input/{name}.json");
            command.args(["-v",&format!("{}:{destination}:ro",path.display())]);forwarded.push((name,destination));
        }
    }
    if env::var_os("TARE_CHECK_SUMMARY").is_some() { command.args(["-e","TARE_CHECK_SUMMARY=1"]); }
    if let Some(defect) = defect { command.args(["-e", &format!("TARE_REHEARSAL_DEFECT={defect}")]); }
    if let Some((_,_,url))=mainnet {
        if let Some(url)=url.map(str::to_owned).or_else(||env::var("TARE_MAINNET_RPC_URL").ok()) {
            // Docker copies this variable from its environment; the credential is not an argument.
            command.env("TARE_MAINNET_RPC_URL",url).args(["-e","TARE_MAINNET_RPC_URL"]);
        }
    }
    command.args(["-e",&format!("TARE_BUILD_IMAGE={image}")]);
    command.args([&image,verb,"--host","--manifest-path"]).arg(&paths.manifest);
    if let Some((program,samples,_))=mainnet { command.arg("--program").arg(program).arg("--samples").arg(samples.to_string()); }
    if verbose() { command.arg("--verbose"); }
    if !workspace.features.is_empty() { command.arg("--features").arg(workspace.features.join(",")); }
    for (name,path) in forwarded {command.arg(format!("--{name}")).arg(path);}
    if verbose() { print_command(&command); }
    let status = command.status().context("start the pinned Tare image")?;
    if !status.success() {
        bail!("container {verb} exited with {status}");
    }
    Ok(true)
}
fn print_command(command: &Command) {
    let quote = |value: &std::ffi::OsStr| { let s=value.to_string_lossy(); if s.contains([' ', '"', '\\']) { format!("\"{}\"", s.replace('"', "\\\"")) } else { s.into_owned() } };
    eprintln!("$ {} {}", quote(command.get_program()), command.get_args().map(quote).collect::<Vec<_>>().join(" "));
}
fn output(command: &mut Command) -> Result<String> {
    if verbose() { print_command(command); }
    let result=command.stdin(Stdio::null()).output().with_context(|| format!("run {}",command.get_program().to_string_lossy()))?;
    let stdout=String::from_utf8(result.stdout)?;
    let stderr=String::from_utf8_lossy(&result.stderr);
    if verbose() || !result.status.success() { eprint!("{stderr}"); }
    if !result.status.success() { eprint!("{stdout}"); bail!("{} exited with {}",command.get_program().to_string_lossy(),result.status); }
    if stderr.contains("has altered the original list of") { bail!("Tare adapter changed the dependency graph: {stderr}"); }
    Ok(stdout)
}
fn run(command: &mut Command) -> Result<()> {
    if verbose() { print_command(command); }
    if command.get_program() == "cargo-build-sbf" || command.get_args().any(|arg| arg == "build-sbf") {
        if let Some(root)=env::var_os("TARE_BUILD_SOURCE_ROOT") { repro::configure(command, Path::new(&root), env::var_os("TARE_BUILD_REHEARSAL").is_some())?; }
    }
    let result=command.stdin(Stdio::null()).output()?;
    let stdout=String::from_utf8_lossy(&result.stdout);
    let stderr=String::from_utf8_lossy(&result.stderr);
    let diagnostics=format!("{stdout} {stderr}").split_whitespace().collect::<Vec<_>>().join(" ");
    let bad_stack=(diagnostics.contains("Stack offset") && diagnostics.contains("exceeded")) || diagnostics.contains("overflows the maximum allowed frame space");
    if verbose() || !result.status.success() || bad_stack { print!("{stdout}"); eprint!("{stderr}"); }
    if !result.status.success() { bail!("{} exited with {}",command.get_program().to_string_lossy(),result.status); }
    if bad_stack { bail!("SBF stack frame exceeds the runtime limit; the ELF is not admitted"); }
    Ok(())
}
fn rehearsal_defect() -> Result<Option<String>> {
    match env::var("TARE_REHEARSAL_DEFECT") {
        Err(env::VarError::NotPresent) => Ok(None),
        Ok(value) if value == "dispatch-log" => Ok(Some(value)),
        _ => bail!("TARE_REHEARSAL_DEFECT only accepts dispatch-log; unset it for ordinary builds"),
    }
}
fn discover(manifest: &Path) -> Result<Package> {
    let manifest = canonical(manifest).with_context(|| format!("read {}", manifest.display()))?;
    let metadata: Value = serde_json::from_str(&output(
        Command::new("cargo")
            .args([
                "metadata",
                "--format-version",
                "1",
                "--no-deps",
                "--manifest-path",
            ])
            .arg(&manifest),
    )?)?;
    let root = PathBuf::from(
        metadata["workspace_root"]
            .as_str()
            .context("Cargo workspace root")?,
    );
    let packages = metadata["packages"].as_array().context("Cargo packages")?;
    let selected = packages
        .iter()
        .filter(|p| {
            p["targets"].as_array().is_some_and(|ts| {
                ts.iter().any(|t| {
                    t["kind"]
                        .as_array()
                        .is_some_and(|ks| ks.iter().any(|k| k == "cdylib"))
                })
            })
        })
        .collect::<Vec<_>>();
    let package = selected
        .iter()
        .copied()
        .find(|p| Path::new(p["manifest_path"].as_str().unwrap_or("")) == manifest)
        .or_else(|| {
            if selected.len() == 1 {
                selected.first().copied()
            } else {
                None
            }
        })
        .context("select one program with --manifest-path path/to/program/Cargo.toml")?;
    let target = package["targets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| {
            t["kind"]
                .as_array()
                .is_some_and(|ks| ks.iter().any(|k| k == "cdylib"))
        })
        .unwrap();
    let dependency = package["dependencies"]
        .as_array()
        .context("Cargo dependencies")?
        .iter()
        .find(|d| d["name"] == "anchor-lang")
        .context("the selected program does not depend on anchor-lang")?;
    let anchor_path = dependency["path"].as_str().map(PathBuf::from);
    let anchor_version = if let Some(path) = &anchor_path {
        let doc: toml::Value = toml::from_str(&fs::read_to_string(path.join("Cargo.toml"))?)?;
        if let Some(version) = doc["package"]["version"].as_str() {
            version.to_owned()
        } else {
            let mut found = None;
            for parent in path.ancestors() {
                let file = parent.join("Cargo.toml");
                if file.exists() {
                    let doc: toml::Value = toml::from_str(&fs::read_to_string(file)?)?;
                    if let Some(version) = doc
                        .get("workspace")
                        .and_then(|w| w.get("package"))
                        .and_then(|p| p.get("version"))
                        .and_then(toml::Value::as_str)
                    {
                        found = Some(version.to_owned());
                        break;
                    }
                }
            }
            found.context("Anchor workspace version")?
        }
    } else {
        let lock = fs::read_to_string(root.join("Cargo.lock")).context(
            "a stock Cargo.lock is required for registry dependencies; run cargo fetch first",
        )?;
        let doc: toml::Value = toml::from_str(&lock)?;
        let versions = doc["package"]
            .as_array()
            .context("Cargo.lock packages")?
            .iter()
            .filter(|p| p["name"].as_str() == Some("anchor-lang"))
            .collect::<Vec<_>>();
        if versions.len() != 1 {
            bail!("expected one anchor-lang version in Cargo.lock");
        }
        versions[0]["version"]
            .as_str()
            .context("Anchor lockfile version")?
            .to_owned()
    };
    if !["0.31.1", "0.32.1", "0.32.3", "1.2.2"].contains(&anchor_version.as_str()) {
        let text=fs::read_to_string(&manifest)?;
        let line=text.lines().position(|line|line.trim_start().starts_with("anchor-lang")).map(|line|line+1).unwrap_or(1);
        let relative=manifest.strip_prefix(&root).unwrap_or(&manifest);
        bail!("unsupported Anchor version {anchor_version} at {}:{line}; supported: 0.31.1, 0.32.1, 0.32.3, 1.2.2", relative.display());
    }
    Ok(Package {
        name: package["name"].as_str().unwrap().into(),
        lib_name: target["name"].as_str().unwrap().into(),
        manifest: PathBuf::from(package["manifest_path"].as_str().unwrap()),
        root,
        source: PathBuf::from(target["src_path"].as_str().unwrap()),
        anchor_version,
    })
}
struct LockSnapshot {
    path: PathBuf,
    original: Option<Vec<u8>>,
}
impl LockSnapshot {
    fn new(root: &Path) -> Result<Self> {
        let path = root.join("Cargo.lock");
        let original = if path.exists() {
            Some(fs::read(&path)?)
        } else {
            None
        };
        Ok(Self { path, original })
    }
}
impl Drop for LockSnapshot {
    fn drop(&mut self) {
        match &self.original {
            Some(bytes) => {
                let _ = fs::write(&self.path, bytes);
            }
            None => {
                if self.path.exists() {
                    let _ = fs::remove_file(&self.path);
                }
            }
        }
    }
}
fn canonical(path: &Path) -> Result<PathBuf> {
    let path = path.canonicalize()?;
    Ok(PathBuf::from(
        path.to_string_lossy()
            .strip_prefix(r"\\?\")
            .unwrap_or(&path.to_string_lossy()),
    ))
}
fn toolchain_root() -> PathBuf {
    canonical(
        &env::var_os("TARE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")),
    )
    .expect("Tare source directory; set TARE_HOME")
}

fn git_command(root:&Path)->Command {
    let trusted=root.ancestors().find(|path|path.join(".git").exists()).unwrap_or(root);
    let mut command=Command::new("git");command.arg("-c").arg(format!("safe.directory={}",trusted.display())).arg("-C").arg(root);command
}
fn git(root: &Path, args: &[&str]) -> Option<String> {
    let mut command=git_command(root);
    command.args(args);
    if verbose() { print_command(&command); }
    let result=command.stdin(Stdio::null()).output().ok()?;
    if verbose() { eprint!("{}",String::from_utf8_lossy(&result.stderr)); }
    if !result.status.success() { return None; }
    String::from_utf8(result.stdout).ok().map(|text|text.trim().to_owned()).filter(|text|!text.is_empty())
}

fn verify_agave_artifacts(paths: &[&Path]) -> Result<()> {
    let root = toolchain_root();
    let name = if cfg!(windows) { "tare-loader-check.exe" } else { "tare-loader-check" };
    let installed = root.join("target/native-versions/release").join(name);
    let cached = installed.clone();
    let checker = if installed.exists() && !root.join("../bench/loader-check/Cargo.toml").exists() {
        installed
    } else {
        run(Command::new("cargo").args(["build", "--locked", "--release", "--manifest-path"])
            .arg(root.join("../bench/loader-check/Cargo.toml"))
            .arg("--target-dir").arg(root.join("target/native-versions")))?;
        cached
    };
    let output = Command::new(checker).args(paths).output().context("run Agave 4.4 ELF admission")?;
    if !output.status.success() {
        bail!("Agave 4.4 rejected build artifacts:\n{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}

fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    fs::write(path, serde_json::to_vec_pretty(value)?)
        .with_context(|| format!("write {}", path.display()))
}
fn cache_path(package: &Package, phase: &str) -> PathBuf {
    toolchain_root()
        .join("target/sbf-cache")
        .join(&package.anchor_version)
        .join("repro-v1")
        .join(phase)
}
fn clear_program_outputs(cache: &Path, library: &str) -> Result<()> {
    let plain = [
        format!("{library}.so"),
        format!("{library}.dll"),
        format!("{library}.d"),
        format!("lib{library}.rlib"),
        format!("lib{library}.d"),
    ];
    let program_prefix = format!("{library}-");
    let lib_prefix = format!("lib{library}-");
    for profile in [
        "debug",
        "release",
        "sbpf-solana-solana/debug",
        "sbpf-solana-solana/release",
    ] {
        for directory in [cache.join(profile), cache.join(profile).join("deps")] {
            if !directory.exists() {
                continue;
            }
            for entry in fs::read_dir(directory)? {
                let entry = entry?;
                if !entry.file_type()?.is_file() {
                    continue;
                }
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if plain.iter().any(|p| p == &name)
                    || name.starts_with(&program_prefix)
                    || name.starts_with(&lib_prefix)
                {
                    fs::remove_file(entry.path())?;
                }
            }
        }
    }
    Ok(())
}
fn build(workspace: &Workspace) -> Result<()> {
    build_with_defect(workspace, rehearsal_defect()?)
}
fn build_with_defect(workspace: &Workspace, defect: Option<String>) -> Result<()> {
    let package = discover(&workspace.manifest_path)?;
    println!("building {} (Anchor {})", package.name, package.anchor_version);
    if let Some(defect)=&defect { eprintln!("REHEARSAL DEFECT ENABLED: {defect}; this artifact must not be attested"); }
    let source_root=git(&package.root,&["rev-parse","--show-toplevel"]).map(PathBuf::from).unwrap_or_else(||package.root.clone());
    let cache_root = toolchain_root()
        .join("target/sbf-cache")
        .join(&package.anchor_version);
    fs::create_dir_all(&cache_root)?;
    let _build_guard = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(cache_root.join(".tare-build.lock"))?;
    _build_guard.lock()?;
    let target = package.root.join("target");
    let deploy = target.join("deploy");
    let tare = target.join("tare");
    fs::create_dir_all(&deploy)?;
    fs::create_dir_all(&tare)?;
    for previous_record in [
        tare.join("build.json"),
        tare.join(format!("{}.build.json", package.lib_name)),
    ] {
        if previous_record.exists() {
            fs::remove_file(previous_record)?;
        }
    }
    let _compile_settings = CompileSettings::new(&source_root, defect.is_some())?;
    let original_source = fs::read(&package.source)?;
    let original_manifest = fs::read(&package.manifest)?;
    let _lock = LockSnapshot::new(&package.root)?;
    let tools = output(Command::new("cargo-build-sbf").arg("--version"))?;
    let stock_metadata: Value = serde_json::from_str(&output(
        Command::new("cargo")
            .args(["metadata", "--format-version", "1", "--manifest-path"])
            .arg(&package.manifest),
    )?)?;
    let resolved_stock_lock = fs::read(package.root.join("Cargo.lock"))
        .context("read Cargo's resolved stock dependency lock")?;
    write_json(&tare.join("stock-dependency-graph.json"), &stock_metadata)?;
    // Resolve layout observations while Cargo.lock still describes the stock
    // graph; patch resolution below temporarily replaces that lock.
    let observable=account_layout_observation(&package.source)?;
    fs::write(tare.join(format!("{}.stock.Cargo.lock", package.lib_name)), &resolved_stock_lock)?;
    // True layout observations keep the descriptor-compatible representation.
    // A typed SDK boundary without layout observations uses compact SDK views.
    let external_entry = observable.is_none();
    let adapters = adapters::prepare(&stock_metadata, &toolchain_root(), &tare.join("adapters"))?;
    let mut patch = vec![
        "--config".to_owned(),
        adapters.config.to_string_lossy().into_owned(),
    ];
    if !workspace.features.is_empty() { patch.extend(["--features".into(), workspace.features.join(",")]); }
    let meta: Value = serde_json::from_str(&output(
        Command::new("cargo")
            .args(["metadata", "--format-version", "1", "--manifest-path"])
            .arg(&package.manifest)
            .args(&patch),
    )?)?;
    adapters::verify(&meta, &adapters)?;
    let metadata_config=adapters.config.clone();
    let resolved_tare_lock = fs::read(package.root.join("Cargo.lock"))?;
    fs::write(tare.join(format!("{}.tare.Cargo.lock", package.lib_name)), &resolved_tare_lock)?;
    write_json(&tare.join(format!("{}.dependency-locks.json", package.lib_name)), &json!({
        "stock": {"hash": digest(&resolved_stock_lock), "path": format!("{}.stock.Cargo.lock", package.lib_name)},
        "tare": {"hash": digest(&resolved_tare_lock), "path": format!("{}.tare.Cargo.lock", package.lib_name)}
    }))?;
    write_json(&tare.join("dependency-graph.json"), &meta)?;
    clear_program_outputs(&cache_path(&package, "net"), &package.lib_name)?;
    let cargo_proxy=env::current_exe()?;
    let compile = |sdk:bool| {
      let mut command=Command::new("cargo-build-sbf");
      command
        .args(["--arch", "v0", "--manifest-path"])
        .arg(&package.manifest)
        .arg("--sbf-out-dir")
        .arg(&tare)
        .arg("--")
        .args(&patch)
        .env("CARGO",&cargo_proxy)
        .env("TARE_METADATA_CONFIG",&metadata_config)
        .env("TARE_REQUIRE_ADAPTER_ROOT",&adapters.root)
        .env(
            "CARGO_TARGET_DIR",
            toolchain_root()
                .join("target/sbf-cache")
                .join(&package.anchor_version)
                .join("repro-v1")
                .join("net"),
        );
      if sdk {
          command.env("TARE_ACCOUNT_ABI","sdk");
          if external_entry { command.env("TARE_EXTERNAL_ENTRY","1"); }
          else { command.env_remove("TARE_EXTERNAL_ENTRY"); }
      } else {
          command.env_remove("TARE_ACCOUNT_ABI").env_remove("TARE_EXTERNAL_ENTRY");
      }
      command
    };
    let mut account_abi=compile_account_abi(&compile,&tare,observable)?;
    if external_entry && account_abi["mode"] == "sdk" {
        account_abi["selection"] = json!(format!("{}; compact SDK account views",account_abi["selection"].as_str().unwrap()));
        println!("account input: compact SDK account views");
    }
    // Recheck the graph left by the actual SBF invocation, not only the plan.
    let compiled_graph:Value=serde_json::from_str(&output(Command::new("cargo").args(["metadata","--format-version","1","--locked","--manifest-path"]).arg(&package.manifest).args(&patch))?)?;
    adapters::verify(&compiled_graph,&adapters)?;
    if fs::read(package.root.join("Cargo.lock"))? != resolved_tare_lock { bail!("SBF build changed the verified Tare dependency lock"); }
    let net = tare.join(format!("{}.so", package.lib_name));
    let symbols=cache_path(&package,"net").join("sbpf-solana-solana/release").join(format!("{}.so",package.lib_name));
    fs::copy(symbols,tare.join(format!("{}.symbols.so",package.lib_name)))?;
    let linked_bytes = fs::read(&net)?;
    fs::write(
        tare.join(format!("{}.linked.so", package.lib_name)),
        &linked_bytes,
    )?;
    let optimized = tare_opt::optimize(&linked_bytes)?;
    let net_bytes = optimized.bytes;
    fs::write(&net, &net_bytes)?;
    let mut optimization = serde_json::to_value(&optimized.stats)?;
    optimization["inputHash"] = json!(digest(&linked_bytes));
    optimization["outputHash"] = json!(digest(&net_bytes));
    write_json(&tare.join("optimization.json"), &optimization)?;
    println!(
        "bytecode: {} instructions rewritten; {} -> {} bytes",
        optimized.stats.modified_pcs.len(),
        linked_bytes.len(),
        net_bytes.len()
    );
    env::remove_var("TARE_BUILD_REHEARSAL");
    fs::write(package.root.join("Cargo.lock"), &resolved_stock_lock)?;
    clear_program_outputs(&cache_path(&package, "stock"), &package.lib_name)?;
    run(Command::new("cargo-build-sbf")
        .args(["--arch", "v0", "--manifest-path"])
        .arg(&package.manifest)
        .arg("--sbf-out-dir")
        .arg(&deploy)
        .arg("--")
        .args(feature_args(&workspace.features))
        .env(
            "CARGO_TARGET_DIR",
            toolchain_root()
                .join("target/sbf-cache")
                .join(&package.anchor_version)
                .join("repro-v1")
                .join("stock"),
        ))?;
    let stock = deploy.join(format!("{}.so", package.lib_name));
    let stock_symbols=cache_path(&package,"stock").join("sbpf-solana-solana/release").join(format!("{}.so",package.lib_name));
    fs::copy(stock_symbols,tare.join(format!("{}.stock-symbols.so",package.lib_name)))?;
    tare_opt::validate(&fs::read(&stock)?)?;
    if let Err(error) = verify_agave_artifacts(&[&stock, &net]) {
        fs::remove_file(&net)?;
        return Err(error);
    }
    let idl_path = target
        .join("idl")
        .join(format!("{}.json", package.lib_name));
    fs::create_dir_all(idl_path.parent().unwrap())?;
    let idl=if let Some(path)=&workspace.idl {
        println!("IDL: explicit supplied input {}",path.file_name().unwrap_or_default().to_string_lossy());
        let value:Value=serde_json::from_slice(&fs::read(path)?)?;
        let _:anchor_lang_idl::types::Idl=serde_json::from_value(value.clone()).context("supplied IDL schema")?;
        write_json(&idl_path,&value)?;value
    } else { stock_idl(&package.manifest,&package.lib_name,&package.anchor_version,&idl_path,&workspace.features)? };
    let stock_bytes = fs::read(&stock)?;
    let programs = prepare_external_programs(
        &package,
        &stock_metadata,
        &deploy,
        &serde_json::to_value(&idl)?,
    )?;
    if original_source != fs::read(&package.source)?
        || original_manifest != fs::read(&package.manifest)?
    {
        bail!("workspace source or manifest changed during build");
    }
    let record = BuildRecord {
        schema_version: "1".into(),
        toolchain_version: VERSION.into(),
        name: package.lib_name.clone(),
        anchor_version: package.anchor_version,
        manifest_path: package.manifest,
        workspace_root: package.root.clone(),
        source_path: package.source,
        stock,
        tare: net,
        idl: idl_path,
        stock_hash: digest(&stock_bytes),
        tare_hash: digest(&net_bytes),
        source: json!({"repository":git(&package.root,&["remote","get-url","origin"]).map(|repository|wire::canonical_repository(&repository).to_owned()),"commit":git(&package.root,&["rev-parse","HEAD"])}),
        architecture: "v0".into(),
        platform_tools: tools.trim().into(),
        optimization,
        programs,
        features: workspace.features.clone(),
        rehearsal_defect: defect,
        reproducibility: "remap-metadata-v1".into(),
        build_environment: build_environment(workspace.host)?,
        account_abi,
    };
    write_json(&tare.join("build.json"), &record)?;
    write_json(
        &tare.join(format!("{}.build.json", package.lib_name)),
        &record,
    )?;
    println!(
        "{}: stock {} bytes -> tare {} bytes",
        package.name,
        stock_bytes.len(),
        net_bytes.len()
    );
    Ok(())
}
fn measure(workspace: &Workspace, prove: bool) -> Result<()> {
    if container(workspace, if prove { "prove" } else { "weigh" })? {
        return Ok(());
    }
    let package = discover(&workspace.manifest_path)?;
    let dir = package.root.join("target/tare");
    let program_record = dir.join(format!("{}.build.json", package.lib_name));
    let record_path = if program_record.exists() {
        program_record
    } else {
        dir.join("build.json")
    };
    let mut record: BuildRecord =
        serde_json::from_slice(&fs::read(record_path).context("run tare build --host first")?)?;
    wire::canonicalize_source(&mut record.source);
    if record.manifest_path != package.manifest {
        bail!("build metadata belongs to another program or host/container path; build and prove in the same mode (--host or image)");
    }
    if record.toolchain_version != VERSION { bail!("build was produced by Tare {}; run tare build with {} before weighing",record.toolchain_version,VERSION); }
    let stock = fs::read(&record.stock)?;
    let net = fs::read(&record.tare)?;
    if digest(&stock) != record.stock_hash || digest(&net) != record.tare_hash {
        bail!("build artifacts changed since tare build");
    }
    if record.features != workspace.features { bail!("build features differ; use --features {}", record.features.join(",")); }
    verify_agave_artifacts(&[&record.stock, &record.tare])?;
    let idl: Value = serde_json::from_slice(&fs::read(&record.idl)?)?;
    let mut hints = tare_prove::source_hints_from_path(&record.source_path)?;
    let owners = record
        .programs
        .iter()
        .map(|program| {
            (
                program
                    .path
                    .file_stem()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                program.address.clone(),
            )
        })
        .collect();
    hints.resolve_account_owners(&owners);
    let external_idls=record.programs.iter().filter_map(|p|p.idl.as_ref()).map(|p|->Result<Value>{Ok(serde_json::from_slice(&fs::read(p)?)?)}).collect::<Result<Vec<_>>>()?;
    hints.resolve_external_account_pdas(&external_idls)?;
    let supplied:Option<chain::ReplayBundle>=workspace.replay.as_ref().map(|path|->Result<_>{Ok(serde_json::from_slice(&fs::read(path)?)?)}).transpose()?;
    let corpus = if let Some(bundle)=&supplied {bundle.generated.clone()} else {tare_prove::generate_corpus_for_version(&idl,&hints,0x54415245,&record.anchor_version)?};
    write_json(&dir.join("corpus.json"),&corpus)?;
    write_json(&dir.join(format!("{}.corpus.json",package.lib_name)),&corpus)?;
    let program=idl["address"].as_str().context("stock IDL program address")?;
    let programs:Vec<(String,Vec<u8>)> = if let Some(bundle)=&supplied {
        use base64::Engine;
        bundle.programs.iter().map(|p|->Result<_>{let bytes=base64::engine::general_purpose::STANDARD.decode(&p.elf_base64)?;if digest(&bytes)!=p.sha256{bail!("replay external ELF hash mismatch")};Ok((p.address.clone(),bytes))}).collect::<Result<_>>()?
    } else {record.programs.iter().map(|p|Ok((p.address.clone(),fs::read(&p.path)?))).collect::<Result<_>>()?};
    let mut report=tare_prove::run_with_programs(program,&stock,&net,&corpus,&programs)?;
    let tests = if let Some(path)=&workspace.tests_corpus {
        Some(serde_json::from_slice::<tare_prove::RecordedCorpus>(&fs::read(path)?)?)
    } else if let Some(bundle)=&supplied { bundle.tests.clone() } else {
        recorded_path(&package).map(|path|->Result<tare_prove::RecordedCorpus>{Ok(serde_json::from_slice(&fs::read(path)?)?)}).transpose()?
    };
    let mainnet = if let Some(path)=&workspace.mainnet_corpus {
        Some(serde_json::from_slice::<tare_prove::RecordedCorpus>(&fs::read(path)?)?)
    } else { supplied.as_ref().and_then(|bundle|bundle.mainnet.clone()) };
    for (expected_kind,recorded) in [("tests",tests.as_ref()),("mainnet",mainnet.as_ref())] {
        if let Some(recorded)=recorded {
            if recorded.kind!=expected_kind { bail!("{expected_kind} corpus has kind {}",recorded.kind); }
            if recorded.target_program!=program { bail!("{expected_kind} corpus target does not match the built program"); }
        }
    }
    let mut recorded_provenance=Vec::new();
    for recorded in tests.iter().chain(mainnet.iter()) {
        if recorded.kind=="tests" {if let Some(commit)=record.source["commit"].as_str(){if commit!=recorded.source_commit {bail!("recorded corpus source commit does not match this build")}}}
        let replay=tare_prove::run_recorded(&stock,&net,recorded)?;
        let provenance=tare_prove::merge_recorded(&mut report,replay)?;
        if recorded.kind=="tests" {println!("recorded outcome reproduction: {} / {} inputs matched",provenance.validator_reproduction.matched,provenance.validator_reproduction.inputs);}
        recorded_provenance.push(provenance);
    }
    use base64::Engine;
    let source_root=git(&package.root,&["rev-parse","--show-toplevel"]).map(PathBuf::from).unwrap_or_else(||package.root.clone());
    let bundle=chain::ReplayBundle{schema_version:"1".into(),manifest_path:package.manifest.strip_prefix(&source_root)?.to_string_lossy().replace('\\',"/"),generated:corpus,features:record.features.clone(),supplied_idl:workspace.idl.as_ref().map(|_|idl.clone()),programs:programs.iter().map(|(address,bytes)|tare_prove::RecordedProgram{address:address.clone(),elf_base64:base64::engine::general_purpose::STANDARD.encode(bytes),sha256:digest(bytes)}).collect(),tests,mainnet};
    let replay=chain::pack_replay(&bundle)?;
    report.corpus_hash=replay["sha256"].as_str().unwrap().to_owned();
    write_json(&dir.join("replay.json"),&bundle)?;
    write_json(&dir.join("execution.json"), &report)?;
    write_json(
        &dir.join(format!("{}.execution.json", package.lib_name)),
        &report,
    )?;
    let gross: u64 = report.instructions.iter().filter_map(|i| i.gross).sum();
    let net_units: u64 = report.instructions.iter().filter_map(|i| i.net).sum();
    let inputs: u64 = report.instructions.iter().map(|i| i.inputs).sum();
    let weighing = wire::document(&json!({"schemaVersion":"1","program":{"name":record.name,"address":program},"toolchainVersion":VERSION,"anchorVersion":record.anchor_version,"buildEnvironment":record.build_environment,"accountAbi":record.account_abi,"source":record.source,"builds":{"stock":{"hash":record.stock_hash,"sizeBytes":stock.len(),"rentLamports":tare_prove::program_rent(stock.len())},"tare":{"hash":record.tare_hash,"sizeBytes":net.len(),"rentLamports":tare_prove::program_rent(net.len())}},"instructions":report.instructions,"totals":{"gross":gross,"net":net_units,"tare":gross as i64-net_units as i64,"inputs":inputs},"proof":report.proof(),"optimization":record.optimization,"recordedCorpus":recorded_provenance,"replay":replay}));
    write_json(
        &dir.join(if prove { "proof.json" } else { "weigh.json" }),
        &weighing,
    )?;
    write_json(
        &dir.join(format!(
            "{}.{}",
            package.lib_name,
            if prove { "proof.json" } else { "weigh.json" }
        )),
        &weighing,
    )?;
    if env::var_os("TARE_CHECK_SUMMARY").is_none() {
    let width = report.instructions.iter().map(|item| item.name.chars().count())
        .chain([record.name.chars().count(), 5]).max().unwrap();
    println!("{:<width$} {:>10} {:>10} {:>10} {:>10}", record.name,"gross","tare","net","inputs");
    for item in &report.instructions {
        match (item.gross, item.tare, item.net) {
            (Some(g), Some(t), Some(n)) => println!(
                "{:<width$} {:>10} {:>10} {:>10} {:>10}", item.name, g, t, n, item.inputs
            ),
            _ => println!("{:<width$} not measured (0 successful inputs)", item.name),
        }
    }
    println!("{:<width$} {:>10} {:>10} {:>10} {:>10}", "total",gross,gross as i64-net_units as i64,net_units,inputs);
    println!(
        "proof: {} ({} transactions)",
        if report.matched() {
            "matched"
        } else {
            "diverged"
        },
        report.comparisons.inputs
    );
    }
    if let Some(divergence) = report.first_divergence {
        eprintln!("{}", serde_json::to_string_pretty(&divergence)?);
        bail!(
            "observable divergence; see {}",
            Path::new("target/tare").join(if prove { "proof.json" } else { "weigh.json" }).display()
        );
    }
    Ok(())
}

fn feature_args(features: &[String]) -> Vec<String> {
    if features.is_empty() { Vec::new() } else { vec!["--features".into(), features.join(",")] }
}
struct CompileSettings(Vec<(&'static str, Option<std::ffi::OsString>)>);
impl CompileSettings {
    fn new(root: &Path, rehearsal: bool) -> Result<Self> {
        let saved=Self(["TARE_BUILD_SOURCE_ROOT","TARE_BUILD_REHEARSAL"].into_iter().map(|key|(key,env::var_os(key))).collect());
        env::set_var("TARE_BUILD_SOURCE_ROOT",root);
        if rehearsal { env::set_var("TARE_BUILD_REHEARSAL","1"); } else { env::remove_var("TARE_BUILD_REHEARSAL"); }
        Ok(saved)
    }
}
impl Drop for CompileSettings { fn drop(&mut self) { for (key,value) in &self.0 { match value { Some(value)=>env::set_var(key,value),None=>env::remove_var(key) } } } }
fn stock_idl(manifest:&Path,library:&str,version:&str,path:&Path,features:&[String])->Result<Value>{
    let mut command=Command::new(env::current_exe()?);
    command.args(["__idl","--manifest"]).arg(manifest).args(["--library",library,"--version",version,"--output"]).arg(path).args(feature_args(features));
    run(&mut command).context("build stock IDL")?;
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}
fn stock_idl_inline(args:&IdlArgs)->Result<()> {
    // Native dependency build scripts (notably OpenSSL's Perl Configure) still
    // use MAX_PATH-limited filesystem calls on Windows. Keep this shared IDL
    // cache short; stock and Tare SBF artifacts retain their versioned caches.
    let cache=toolchain_root().join("target/i").join(&args.version);
    clear_program_outputs(&cache,&args.library)?;
    env::set_var("CARGO_TARGET_DIR",cache);
    write_json(&args.output,&stock_idl::build(&args.manifest,&args.features)?)
}

fn prepare_external_programs(
    package: &Package,
    metadata: &Value,
    deploy: &Path,
    idl: &Value,
) -> Result<Vec<ExternalProgram>> {
    let members = metadata["workspace_members"]
        .as_array()
        .context("Cargo members")?;
    let packages = metadata["packages"].as_array().unwrap();
    let selected = packages
        .iter()
        .find(|p| p["manifest_path"].as_str() == package.manifest.to_str())
        .unwrap();
    let nodes = metadata["resolve"]["nodes"].as_array().unwrap();
    let mut dependencies = std::collections::BTreeSet::new();
    let mut pending = vec![selected["id"].as_str().unwrap().to_owned()];
    while let Some(id) = pending.pop() {
        if !dependencies.insert(id.clone()) {
            continue;
        }
        if let Some(node) = nodes.iter().find(|node| node["id"] == id) {
            pending.extend(
                node["deps"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|dep| dep["pkg"].as_str().unwrap().to_owned()),
            );
        }
    }
    fn references(value: &Value, address: &str) -> bool {
        match value {
            Value::String(s) => s == address,
            Value::Array(values) => values.iter().any(|v| references(v, address)),
            Value::Object(values) => values.values().any(|v| references(v, address)),
            _ => false,
        }
    }
    let mut programs = vec![];
    for other in metadata["packages"].as_array().unwrap() {
        if !members.contains(&other["id"])
            || other["manifest_path"].as_str() == package.manifest.to_str()
        {
            continue;
        }
        let Some(target) = other["targets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["kind"].as_array().unwrap().iter().any(|v| v == "cdylib"))
        else {
            continue;
        };
        let source = fs::read_to_string(target["src_path"].as_str().unwrap())?;
        let Some(start) = source.find("declare_id!(") else {
            continue;
        };
        let Some(address) = source[start..].split('"').nth(1) else {
            continue;
        };
        if address.len() < 32 || address.len() > 44 {
            continue;
        }
        if !dependencies.contains(other["id"].as_str().unwrap()) && !references(idl, address) {
            continue;
        }
        clear_program_outputs(
            &cache_path(package, "stock"),
            target["name"].as_str().unwrap(),
        )?;
        run(Command::new("cargo-build-sbf")
            .args(["--arch", "v0", "--manifest-path"])
            .arg(other["manifest_path"].as_str().unwrap())
            .arg("--sbf-out-dir")
            .arg(deploy)
            .env(
                "CARGO_TARGET_DIR",
                toolchain_root()
                    .join("target/sbf-cache")
                    .join(&package.anchor_version)
                    .join("repro-v1")
                    .join("stock"),
            ))?;
        let library=target["name"].as_str().unwrap();
        let idl_path=deploy.parent().unwrap().join("idl").join(format!("{library}.json"));
        stock_idl(Path::new(other["manifest_path"].as_str().unwrap()),library,&package.anchor_version,&idl_path,&[])?;
        programs.push(ExternalProgram {
            address: address.into(),
            path: deploy.join(format!("{library}.so")),
            idl:Some(idl_path),
        });
    }
    Ok(programs)
}

fn recorded_path(package: &Package) -> Option<PathBuf> {
    let local = package.root.join("target/tare/tests.json");
    if local.exists() {
        return Some(local);
    }
    let bench = toolchain_root().parent()?.join("bench");
    let manifest: Value =
        serde_json::from_slice(&fs::read(bench.join("manifest.json")).ok()?).ok()?;
    for entry in manifest["programs"].as_array()? {
        let checkout = entry["checkout"]
            .as_str()
            .or_else(|| manifest["checkout"].as_str())?;
        let path = bench.join(checkout).join(entry["manifest"].as_str()?);
        if canonical(&path).ok().as_ref() == Some(&package.manifest) {
            let recorded = bench
                .join("corpus-data")
                .join(entry["id"].as_str()?)
                .join("tests.json");
            if recorded.exists() {
                return Some(recorded);
            }
        }
    }
    None
}

fn build_environment(host: bool) -> Result<Value> {
    if !host || env::var_os("TARE_IN_CONTAINER").is_some() {
        let image=env::var("TARE_BUILD_IMAGE").or_else(|_|env::var("TARE_IMAGE"))
            .unwrap_or_else(|_|format!("tare-toolchain:{VERSION}"));
        Ok(json!({"kind":"image","image":image}))
    } else {
        let info=output(Command::new("rustc").arg("-vV"))?;
        let target=info.lines().find_map(|line|line.strip_prefix("host: "))
            .context("rustc host target triple")?;
        Ok(json!({"kind":"host","targetTriple":target}))
    }
}
fn environment_label(value:&Value) -> String {
    match value["kind"].as_str() {
        Some("image")=>format!("image {}",value["image"].as_str().unwrap_or("unknown")),
        Some("host")=>format!("host {}",value["targetTriple"].as_str().unwrap_or("unknown")),
        _=>"unrecorded".into(),
    }
}

fn compile_account_abi(compile:&impl Fn(bool)->Command, directory:&Path, observation:Option<String>) -> Result<Value> {
    let attempt=|sdk|->Result<std::process::Output> {
        let mut command=compile(sdk);
        if verbose(){print_command(&command);}
        if let Some(root)=env::var_os("TARE_BUILD_SOURCE_ROOT") {
            repro::configure(&mut command,Path::new(&root),env::var_os("TARE_BUILD_REHEARSAL").is_some())?;
        }
        Ok(command.stdin(Stdio::null()).output()?)
    };
    if let Some(reason)=observation {
        let output=attempt(true)?;
        let log=format!("{}{}",String::from_utf8_lossy(&output.stdout),String::from_utf8_lossy(&output.stderr));
        if verbose()||!output.status.success(){print!("{log}");}
        if !output.status.success(){bail!("cargo-build-sbf exited with {}",output.status);}
        if (log.contains("Stack offset")&&log.contains("exceeded"))||log.contains("overflows the maximum allowed frame space"){bail!("SBF stack frame exceeds the runtime limit; the ELF is not admitted");}
        println!("account ABI: SDK ({reason})");
        return Ok(json!({"mode":"sdk","selection":reason}));
    }
    let raw=attempt(false)?;
    let raw_log=format!("{}{}",String::from_utf8_lossy(&raw.stdout),String::from_utf8_lossy(&raw.stderr));
    let requires_sdk=!raw.status.success() && sdk_type_diagnostic(&raw_log);
    let (output,mode)=if requires_sdk {
        fs::write(directory.join("account-abi-diagnostic.txt"),&raw_log)?;
        println!("account ABI: SDK (typed AccountInfo boundary)");
        (attempt(true)?,"sdk")
    }else{(raw,"raw")};
    let log=format!("{}{}",String::from_utf8_lossy(&output.stdout),String::from_utf8_lossy(&output.stderr));
    let stack=(log.contains("Stack offset")&&log.contains("exceeded"))||log.contains("overflows the maximum allowed frame space");
    if verbose()||!output.status.success()||stack {print!("{log}");}
    if !output.status.success(){bail!("cargo-build-sbf exited with {}",output.status);}
    if stack{bail!("SBF stack frame exceeds the runtime limit; the ELF is not admitted");}
    Ok(json!({"mode":mode,"selection":if requires_sdk{"Rust type checking requires SDK AccountInfo"}else{"raw representation type checked; source has no detected layout observations"}}))
}

fn account_layout_observation(source:&Path)->Result<Option<String>> {
    account_abi::account_layout_observation(source)
}

fn sdk_type_diagnostic(log:&str)->bool {
    log.split("error[").skip(1).any(|error| {
        (error.starts_with("E0308]") || error.starts_with("E0277]") || error.starts_with("E0599]"))
            && ["AccountInfo","AccountData","AccountLamports","DataRef","LamportsRef"].iter().any(|name|error.contains(name))
    })
}
