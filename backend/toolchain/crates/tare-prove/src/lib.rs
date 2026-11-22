//! Differential execution of complete transactions against two SBF binaries.
//! The corpus is data: generated inputs use exactly the same runner as imported inputs.

use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use litesvm::{types::TransactionMetadata, LiteSVM};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use solana_account::Account;
use solana_address::Address;
use solana_instruction::{account_meta::AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SourceHints {
    #[serde(default)]
    pub scenarios: BTreeMap<String, ScenarioHint>,
    #[serde(default)]
    pub legacy_idl: bool,
    pub instructions: BTreeMap<String, BTreeMap<String, AccountHint>>,
    #[serde(default)]
    pub instruction_args: BTreeMap<String, BTreeMap<String, usize>>,
    #[serde(default)]
    pub spl_operations: BTreeMap<String, String>,
    #[serde(default)]
    pub remaining_accounts: BTreeMap<String, Vec<RemainingHint>>,
    #[serde(default)]
    pub event_cpi: BTreeSet<String>,
    #[serde(default)]
    pub validation_only: BTreeSet<String>,
    #[serde(default)]
    pub pubkey_arrays: BTreeSet<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ScenarioHint {
    pub arguments: BTreeMap<String, u64>,
    pub payers: BTreeSet<String>,
    pub owners: BTreeMap<String, String>,
    pub writable: BTreeSet<String>,
    pub initial_lamport_debits: BTreeMap<String, u64>,
    pub spaces: BTreeMap<String, usize>,
    pub mint_decimals: BTreeMap<String, String>,
    pub mint_freeze_authorities: BTreeMap<String, String>,
    pub bumps: BTreeMap<String, String>,
    pub seed_overrides: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AccountHint {
    pub init: bool,
    pub account_type: Option<String>,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub zero: bool,
    #[serde(default)]
    pub owner_crate: Option<String>,
    #[serde(default)]
    pub owner_program: Option<String>,
    #[serde(default)]
    pub spl: Option<SplHint>,
    #[serde(default)]
    pub executable: bool,
    #[serde(default)]
    pub program_type: Option<String>,
    #[serde(default)]
    pub program_address: Option<String>,
    /// A handler passes this account as a transaction-signed CPI authority.
    #[serde(default)]
    pub cpi_signer: bool,
    #[serde(default)]
    pub source_account: Option<String>,
    #[serde(default)]
    pub pda: Option<Value>,
    #[serde(default)]
    pub writable: bool,
    #[serde(default)]
    pub raw_data_len: Option<usize>,
    #[serde(default)]
    pub self_address: bool,
    #[serde(default)]
    pub fixed_address: Option<String>,
    #[serde(default)]
    pub cpi_name: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RemainingHint {
    pub index:usize,
    pub name:String,
    pub writable:bool,
    pub account:AccountHint,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SplHint {
    pub kind:String,
    pub mint:Option<String>,
    pub authority:Option<String>,
    pub token_program:Option<String>,
    pub decimals:Option<u8>,
    pub associated:bool,
}

pub mod json_u64;
mod source;
mod spl;
pub use source::{source_hints, source_hints_from_path};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountState {
    pub address: String,
    pub owner: String,
    #[serde(with = "crate::json_u64")]
    pub lamports: u64,
    pub executable: bool,
    pub data_base64: String,
    #[serde(default)]
    #[serde(with = "crate::json_u64")]
    pub rent_epoch: u64,
}
impl AccountState {
    fn from_account(address: Address, account: &Account) -> Self {
        Self {
            address: address.to_string(),
            owner: account.owner.to_string(),
            lamports: account.lamports,
            executable: account.executable,
            data_base64: BASE64.encode(&account.data),
            rent_epoch: account.rent_epoch,
        }
    }
    fn account(&self) -> Result<Account> {
        Ok(Account {
            lamports: self.lamports,
            data: BASE64.decode(&self.data_base64)?,
            owner: self.owner.parse()?,
            executable: self.executable,
            rent_epoch: self.rent_epoch,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputAccount {
    pub address: String,
    pub signer: bool,
    pub writable: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputInstruction {
    pub program: String,
    pub name: String,
    pub accounts: Vec<InputAccount>,
    pub data_base64: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AccountMutation {
    CopyAccount {
        from: String,
        address: String,
    },
    Owner {
        address: String,
        owner: String,
    },
    Data {
        address: String,
        data_base64: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Case {
    pub label: String,
    pub kind: String,
    pub instructions: Vec<InputInstruction>,
    /// Speculative cases execute on a fork and leave the sequence state intact.
    pub commit: bool,
    pub mutations: Vec<AccountMutation>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sequence {
    pub label: String,
    pub payer: String,
    pub starting_accounts: Vec<AccountState>,
    /// Deterministic test entropy, never user key material.
    pub signer_seeds: Vec<[u8; 32]>,
    pub cases: Vec<Case>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fixture_provenance: Vec<FixtureProvenance>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixtureProvenance {
    pub address: String,
    pub account_type: String,
    pub origin: String,
    pub reason: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Corpus {
    #[serde(with = "crate::json_u64")]
    pub seed: u64,
    #[serde(default)]
    pub program_aliases: Vec<String>,
    #[serde(default)]
    pub executable_fixtures: Vec<String>,
    pub sequences: Vec<Sequence>,
    pub instructions: Vec<InstructionIdentity>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstructionIdentity {
    pub name: String,
    pub discriminator: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CorpusCounts {
    #[serde(with = "crate::json_u64")]
    pub tests: u64,
    #[serde(with = "crate::json_u64")]
    pub generated: u64,
    #[serde(with = "crate::json_u64")]
    pub mainnet: u64,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Comparisons {
    #[serde(with = "crate::json_u64")]
    pub inputs: u64,
    #[serde(with = "crate::json_u64")]
    pub account_bytes: u64,
    #[serde(with = "crate::json_u64")]
    pub lamports: u64,
    #[serde(with = "crate::json_u64")]
    pub logs: u64,
    #[serde(with = "crate::json_u64")]
    pub return_data: u64,
    #[serde(with = "crate::json_u64")]
    pub errors: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstructionMeasurement {
    pub name: String,
    pub discriminator: String,
    #[serde(with = "crate::json_u64")]
    pub inputs: u64,
    #[serde(with = "crate::json_u64::option")]
    pub gross: Option<u64>,
    #[serde(with = "crate::json_u64::option")]
    pub net: Option<u64>,
    #[serde(with = "crate::json_u64::signed_option")]
    pub tare: Option<i64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunReport {
    pub corpus_hash: String,
    pub counts: CorpusCounts,
    pub comparisons: Comparisons,
    pub instructions: Vec<InstructionMeasurement>,
    #[serde(serialize_with = "crate::json_u64::serialize_json", deserialize_with = "crate::json_u64::deserialize_optional_json")]
    pub first_divergence: Option<Value>,
    #[serde(serialize_with = "crate::json_u64::serialize_json")]
    pub outcomes: Vec<Value>,
}
impl RunReport {
    pub fn matched(&self) -> bool {
        self.first_divergence.is_none()
    }
    pub fn proof(&self) -> Value {
        json!({ "status": if self.matched() { "matched" } else { "diverged" }, "corpus": { "hash": self.corpus_hash, "counts": self.counts }, "comparisons": self.comparisons, "firstDivergence": self.first_divergence })
    }
}

pub fn program_rent(binary_size: usize) -> u64 {
    LiteSVM::new().minimum_balance_for_rent_exemption(binary_size + 45)
}

pub fn generate_corpus(idl: &Value, seed: u64) -> Result<Corpus> {
    generate_corpus_with_hints(idl, &SourceHints::default(), seed)
}

pub fn generate_corpus_with_hints(idl: &Value, hints: &SourceHints, seed: u64) -> Result<Corpus> {
    generate_corpus_for_version(idl, hints, seed, "0.31.1")
}

/// Built-in instruction coverage follows the selected stock Anchor version.
pub fn generate_corpus_for_version(idl: &Value, hints: &SourceHints, seed: u64, anchor_version: &str) -> Result<Corpus> {
    let legacy_idl = match anchor_version.split('.').next() {
        Some("0") => true,
        Some("1") => hints.legacy_idl,
        _ => bail!("unsupported Anchor version for corpus generation: {anchor_version}"),
    };
    let program: Address = idl["address"]
        .as_str()
        .or_else(|| idl["metadata"]["address"].as_str())
        .ok_or_else(|| anyhow!("stock IDL is missing its program address"))?
        .parse()?;
    let instructions = idl["instructions"]
        .as_array()
        .ok_or_else(|| anyhow!("stock IDL is missing instructions"))?;
    let identities = instructions
        .iter()
        .map(|ix| {
            Ok(InstructionIdentity {
                name: required_str(ix, "name")?.to_owned(),
                discriminator: hex::encode(discriminator(ix, "global")?),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let mut ordered: Vec<&Value> = instructions.iter().collect();
    ordered.sort_by_key(|ix| {
        let name = ix["name"].as_str().unwrap_or("");
        let fields=hints.instructions.iter().find(|(key,_)|normalize(key)==normalize(name)).map(|(_,fields)|fields);
        if fields.is_some_and(|fields|fields.values().any(|hint|hint.zero)){0}
        else if fields.is_some_and(|fields|fields.values().any(|hint|hint.init&&hint.spl.as_ref().is_some_and(|s|s.kind=="mint"))){1}
        else if fields.is_some_and(|fields|fields.values().any(|hint|hint.init)){2}else{3}
    });
    let is_initializer=|ix:&&Value|hints.instructions.iter().find(|(key,_)|normalize(key)==normalize(ix["name"].as_str().unwrap_or(""))).is_some_and(|(_,fields)|fields.values().any(|h|h.init||h.zero));
    let (initializers,handlers):(Vec<_>,Vec<_>)=ordered.iter().copied().partition(is_initializer);
    let mut branches=vec![ordered.clone()];
    if !initializers.is_empty() && handlers.len()>1 && handlers.len()<=4 {
        for first in 1..handlers.len(){let mut branch=initializers.clone();branch.push(handlers[first]);branch.extend(handlers.iter().enumerate().filter_map(|(index,ix)|(index!=first).then_some(*ix)));branches.push(branch);}
    }
    let mut sequences = Vec::new();
    for round in 0..4 {for (branch_index,branch) in branches.iter().enumerate(){
        let mut generator = Generator::new(idl, hints, program, seed, round);
        // Allocate all identities first: args may name signers used in a later instruction.
        for ix in instructions {
            for account in flat_accounts(ix)? {
                let name=required_str(&account,"name")?;
                if boolean(&account,"signer","isSigner"){generator.signer_names.insert(normalize(name));}
                generator.identity(name);
            }
            for arg in array(ix, "args")? {
                if arg["type"] == "pubkey" || arg["type"] == "publicKey" {
                    generator.identity(required_str(arg, "name")?);
                }
            }
        }
        for _ in 0..2 {for ix in instructions {
            generator.resolve_accounts(ix)?;
        }}
        for ix in branch {
            generator.add_instruction(ix)?;
        }
        generator.add_builtin_cases(legacy_idl);
        if legacy_idl { generator.add_idl_lifecycle()?; }
        generator.materialize_spl_fixtures()?;
        let mut sequence=generator.finish();
        if branch_index>0{sequence.label=format!("{}-branch-{branch_index}",sequence.label);}
        sequences.push(sequence);
    }}
    // Independent typed-state scenarios avoid collisions between unrelated
    // handlers that reuse field names. They supplement the complete original
    // sequence; these accounts are seeded fixtures, not a claimed lifecycle.
    if !instructions.is_empty() {
        for ix in instructions {
            if flat_accounts(ix)?.is_empty(){continue}
            let name=required_str(ix,"name")?;
            let mut isolated_hints=hints.clone();
            isolated_hints.instructions.retain(|n,_|normalize(n)==normalize(name));
            for round in 0..4 {
                let mut generator=Generator::new(idl,&isolated_hints,program,seed,round);
                for account in flat_accounts(ix)? {
                    let account_name=required_str(&account,"name")?;
                    if boolean(&account,"signer","isSigner"){generator.signer_names.insert(normalize(account_name));}
                    generator.identity(account_name);
                }
                generator.resolve_accounts(ix)?;generator.resolve_accounts(ix)?;generator.add_instruction(ix)?;
                generator.materialize_spl_fixtures()?;
        let mut sequence=generator.finish();sequence.label=format!("{}-isolated-{name}",sequence.label);sequences.push(sequence);
            }
        }
    }
    for ix in instructions {
        let name=required_str(ix,"name")?;
        let Some(scenario)=hints.scenarios.get(name).filter(|s|!s.arguments.is_empty()||!s.payers.is_empty()||!s.owners.is_empty()||!s.writable.is_empty()||!s.mint_decimals.is_empty()||!s.mint_freeze_authorities.is_empty()||!s.bumps.is_empty()||!s.initial_lamport_debits.is_empty()) else {continue};
        let mut isolated=hints.clone();isolated.instructions.retain(|key,_|normalize(key)==normalize(name));
        for round in 0..4 {
            let mut generator=Generator::new(idl,&isolated,program,seed,round);
            generator.scenario=Some((name.into(),scenario.clone()));
            for a in flat_accounts(ix)? {let account_name=required_str(&a,"name")?;generator.identity(account_name);if boolean(&a,"signer","isSigner")||scenario.payers.contains(account_name){generator.signer_names.insert(normalize(account_name));}}
            for _ in 0..3 {generator.resolve_accounts(ix)?;}
            generator.add_instruction(ix)?;
            generator.materialize_spl_fixtures()?;
            let mut sequence=generator.finish();sequence.label=format!("{}-resolved-{name}",sequence.label);sequences.push(sequence);
        }
    }
    Ok(Corpus {
        seed,
        program_aliases: vec![wrong_program_address(program).to_string()],
        executable_fixtures: hints.instructions.values().flat_map(|fields|fields.values()).filter(|hint|hint.executable&&hint.program_type.is_none()).filter_map(|hint|hint.program_address.clone()).filter(|address|address!=&program.to_string()&&address!="11111111111111111111111111111111").collect::<BTreeSet<_>>().into_iter().collect(),
        sequences,
        instructions: identities,
    })
}

struct Generator<'a> {
    idl: &'a Value,
    hints: &'a SourceHints,
    program: Address,
    transaction_payer: Address,
    seed: u64,
    round: u64,
    identities: BTreeMap<String, Address>,
    account_bindings: BTreeMap<(String,String),Address>,
    signer_names: BTreeSet<String>,
    seeds: BTreeMap<String, [u8; 32]>,
    accounts: BTreeMap<String, AccountState>,
    cases: Vec<Case>,
    init_types: BTreeSet<(String,String)>,
    multiple_types: BTreeSet<String>,
    init_addresses: BTreeSet<Address>,
    derived_bumps: BTreeMap<String,u8>,
    fixture_provenance: Vec<FixtureProvenance>,
    scenario: Option<(String, ScenarioHint)>,
}
impl<'a> Generator<'a> {
    fn new(
        idl: &'a Value,
        hints: &'a SourceHints,
        program: Address,
        seed: u64,
        round: u64,
    ) -> Self {
        let init_types=hints.instructions.values().flat_map(|m|m.iter().filter(|(_,h)|h.init).filter_map(|(name,h)|h.account_type.as_ref().map(|ty|(normalize(name),ty.clone())))).collect();
        let mut types:BTreeMap<String,BTreeSet<String>>=BTreeMap::new();
        for fields in hints.instructions.values(){for (name,hint) in fields{if let Some(ty)=&hint.account_type{types.entry(normalize(name)).or_default().insert(ty.clone());}}}
        let multiple_types=types.into_iter().filter_map(|(name,types)|(types.len()>1).then_some(name)).collect();
        let mut this = Self {
            idl,
            hints,
            program,
            transaction_payer: Address::default(),
            seed,
            round,
            identities: BTreeMap::new(),
            account_bindings: BTreeMap::new(),
            signer_names: BTreeSet::new(),
            seeds: BTreeMap::new(),
            accounts: BTreeMap::new(),
            cases: Vec::new(),
            init_types,
            multiple_types,
            init_addresses:BTreeSet::new(),
            derived_bumps:BTreeMap::new(),
            fixture_provenance: Vec::new(),
            scenario: None,
        };
        let payer_seed: [u8; 32] = Sha256::digest(format!("tare:{seed}:{round}:transaction-fee-payer")).into();
        let payer = Keypair::new_from_array(payer_seed).pubkey();
        this.transaction_payer = payer;
        this.seeds.insert("#transaction-fee-payer".into(), payer_seed);
        this.accounts.insert(payer.to_string(), AccountState::from_account(payer, &Account {lamports:10_000_000_000,data:vec![],owner:Address::default(),executable:false,rent_epoch:0}));
        this
    }
    fn identity(&mut self, name: &str) -> Address {
        let name = normalize(name);
        if let Some(address) = self.identities.get(&name) {
            return *address;
        }
        let seed: [u8; 32] =
            Sha256::digest(format!("tare:{}:{}:{}", self.seed, self.round, name)).into();
        let address = Keypair::new_from_array(seed).pubkey();
        self.identities.insert(name.clone(), address);
        self.seeds.insert(name, seed);
        address
    }
    fn fund(&mut self, name: &str) {
        let address = self.identity(name);
        self.accounts.entry(address.to_string()).or_insert_with(|| {
            AccountState::from_account(
                address,
                &Account {
                    lamports: 10_000_000_000,
                    data: vec![],
                    owner: Address::default(),
                    executable: false,
                    rent_epoch: 0,
                },
            )
        });
    }
    fn hint(&self, ix: &str, name: &str) -> AccountHint {
        self.hints
            .instructions
            .iter()
            .find(|(n, _)| normalize(n) == normalize(ix))
            .and_then(|(_, fields)| fields.iter().find(|(n, _)| normalize(n) == normalize(name)))
            .map(|(_, hint)| hint.clone())
            .unwrap_or_default()
    }
    fn bind_account(&mut self,ix:&Value,name:&str,address:Address){
        let key=normalize(name);
        self.account_bindings.insert((normalize(ix["name"].as_str().unwrap_or("")),key.clone()),address);
        if !self.signer_names.contains(&key) {self.identities.insert(key,address);}
    }
    fn resolve_accounts(&mut self, ix: &Value) -> Result<()> {
        self.resolve_spl_programs(ix)?;
        let instruction=required_str(ix,"name")?;
        if self.hints.event_cpi.iter().any(|name|normalize(name)==normalize(instruction)){
            let (authority,_)=Address::find_program_address(&[b"__event_authority"],&self.program);
            self.identities.insert(normalize("event_authority"),authority);
            self.identities.insert(normalize("program"),self.program);
        }
        for account in flat_accounts(ix)? {
            let name = required_str(&account, "name")?;
            let key = normalize(name);
            if let Some(address) = account["address"].as_str() {
                self.bind_account(ix,name,address.parse()?);
                continue;
            }
            let hint=self.hint(instruction,name);
            if self.multiple_types.contains(&normalize(name)) {if let Some(ty)=&hint.account_type {
                let address=self.identity(&format!("__typed_{name}_{ty}"));self.bind_account(ix,name,address);
            }}
            if hint.self_address {self.account_bindings.insert((normalize(instruction),key.clone()),self.program);continue;}
            if let Some(address)=&hint.fixed_address {self.account_bindings.insert((normalize(instruction),key.clone()),address.parse()?);continue;}
            if let Some(source)=&hint.source_account {
                let source=self.identity(source);self.bind_account(ix,name,source);
            }
            if hint.executable {
                if let Some(address)=hint.program_address {self.bind_account(ix,name,address.parse()?);continue}
                if hint.program_type.is_none(){self.bind_account(ix,name,Address::default());continue}
            }
            if key == "systemprogram" {
                self.bind_account(ix,name,Address::default());
                continue;
            }
            if self.hint(required_str(ix,"name")?,name).spl.as_ref().is_some_and(|hint|hint.associated){continue}
            let pda=self.shared_pda(ix,&account)?;
            if let Some(pda)=pda {
                let seeds = pda["seeds"]
                    .as_array()
                    .ok_or_else(|| anyhow!("IDL PDA {} has no seeds", name))?
                    .iter()
                    .map(|s| self.seed_bytes(s, ix))
                    .collect::<Result<Vec<_>>>()?;
                let slices: Vec<&[u8]> = seeds.iter().map(Vec::as_slice).collect();
                let pda_program = if let Some(seed) = pda.get("program") {
                    let bytes: [u8; 32] = self
                        .seed_bytes(seed, ix)?
                        .try_into()
                        .map_err(|_| anyhow!("IDL PDA program is not 32 bytes"))?;
                    Address::from(bytes)
                } else {
                    self.program
                };
                let (mut address, mut bump) = Address::find_program_address(&slices, &pda_program);
                if let Some((_,scenario))=&self.scenario {if let Some(argument)=scenario.bumps.get(name) {if let Some(value)=scenario.arguments.get(argument) {bump=*value as u8;let mut fixed=slices.clone();let b=[bump];fixed.push(&b);if let Ok(derived)=Address::create_program_address(&fixed,&pda_program){address=derived;}}}}
                self.derived_bumps.insert(normalize(name),bump);
                self.bind_account(ix,name,address);
            } else if name.contains('.') && account["relations"].is_array() {
                let related=self.identity(name.rsplit('.').next().unwrap());
                self.bind_account(ix,name,related);
            }
        }
        Ok(())
    }
    fn shared_pda(&self,ix:&Value,account:&Value)->Result<Option<Value>> {
        if let Some((_,scenario))=&self.scenario {if let Some(pda)=scenario.seed_overrides.get(required_str(account,"name")?){return Ok(Some(pda.clone()))}}
        if account["pda"].is_object(){return Ok(Some(account["pda"].clone()))}
        let hint=self.hint(required_str(ix,"name")?,required_str(account,"name")?);
        if let Some(pda)=hint.pda {return Ok(Some(pda))}
        // A new independent SPL scenario uses a client-created fixture unless
        // this instruction itself requires a PDA. An unrelated initializer's
        // same-named mint must not force an unsignable snapshot here.
        if self.scenario.is_some() && !hint.init && hint.spl.is_some() {return Ok(None)}
        if boolean(account,"signer","isSigner") || account["relations"].is_array(){return Ok(None)}
        let name=required_str(account,"name")?;
        let names:BTreeSet<_>=flat_accounts(ix)?.iter().filter_map(|a|a["name"].as_str().map(str::to_owned)).collect();
        let mut candidates=Vec::new();
        for other in array(self.idl,"instructions")? {
            for a in flat_accounts(other)? {
                if a["name"].as_str()!=Some(name)||!a["pda"].is_object(){continue}
                let valid=a["pda"]["seeds"].as_array().is_some_and(|seeds|seeds.iter().all(|seed|match seed["kind"].as_str(){Some("const")=>true,Some("account")=>seed["path"].as_str().is_some_and(|path|!path.contains('.')&&names.contains(path)),_=>false}));
                if valid&&!candidates.contains(&a["pda"]){candidates.push(a["pda"].clone());}
            }
        }
        Ok(if candidates.len()==1{candidates.pop()}else{None})
    }
    fn seed_bytes(&mut self, seed: &Value, ix: &Value) -> Result<Vec<u8>> {
        match required_str(seed, "kind")? {
            "const" => bytes(&seed["value"]),
            "account" => {
                let path = required_str(seed, "path")?;
                if path.contains('.') && !self.identities.contains_key(&normalize(path)) {
                    bail!("generated corpus does not yet resolve account-data PDA seed {path}")
                }
                let bound=self.account_bindings.get(&(normalize(ix["name"].as_str().unwrap_or("")),normalize(path))).copied();
                Ok(bound.unwrap_or_else(||self.identity(path)).to_bytes().to_vec())
            }
            "arg" => {
                let path = required_str(seed, "path")?;
                let args=array(ix,"args")?;
                let index=self.hints.instruction_args.iter().find(|(name,_)|normalize(name)==normalize(ix["name"].as_str().unwrap_or(""))).and_then(|(_,aliases)|aliases.get(path));
                let mut arg=args.iter().find(|a|a["name"].as_str()==Some(path)).or_else(||index.and_then(|i|args.get(*i))).cloned();
                if arg.is_none() {
                    // A field of a struct argument, such as `args.program`: the same field the instruction data encodes.
                    let mut parts=path.split('.');
                    arg=args.iter().find(|a|a["name"].as_str()==parts.next()).cloned();
                    for field in parts {
                        let defined=arg.as_ref().and_then(|a|a["type"]["defined"].as_str().or_else(||a["type"]["defined"]["name"].as_str())).map(str::to_owned);
                        arg=defined.and_then(|name|self.idl["types"].as_array().and_then(|ts|ts.iter().find(|t|t["name"].as_str()==Some(&name))).and_then(|t|t["type"]["fields"].as_array()).and_then(|fs|fs.iter().find(|f|f["name"].as_str()==Some(field))).cloned());
                    }
                }
                let arg=arg.ok_or_else(||anyhow!("PDA seed argument {path} is missing"))?;
                let bytes=self.encode(&arg["type"],required_str(&arg,"name")?)?;
                if arg["type"]=="string"||arg["type"].get("vec").is_some()||(self.scenario.is_some()&&arg["type"]=="bytes"){Ok(bytes[4..].to_vec())}else{Ok(bytes)}
            }
            other => bail!("generated corpus does not implement PDA seed kind {other}"),
        }
    }
    fn encode(&mut self, ty: &Value, name: &str) -> Result<Vec<u8>> {
        if let Some(ty) = ty.as_str() {
            let name_key=normalize(name);
            let fixed=self.scenario.as_ref().and_then(|(_,s)|s.arguments.iter().find(|(key,_)|normalize(key)==name_key).map(|(_,n)|*n));
            let inferred_bump=if name_key=="nonce"||name_key=="bump" {
                if self.derived_bumps.len()==1{self.derived_bumps.values().next().copied()}else{None}
            }else{self.derived_bumps.get(name_key.trim_end_matches("bump")).copied()};
            let n = fixed.or_else(||inferred_bump.filter(|_|ty=="u8").map(u64::from)).unwrap_or([0u64, 1, 42, 255][self.round as usize]);
            return Ok(match ty {
                "bool" => vec![(self.round % 2) as u8],
                "u8" | "i8" => vec![n as u8],
                "u16" | "i16" => (n as u16).to_le_bytes().to_vec(),
                "u32" | "i32" | "f32" => (n as u32).to_le_bytes().to_vec(),
                "u64" | "i64" | "f64" => n.to_le_bytes().to_vec(),
                "u128" | "i128" => (n as u128).to_le_bytes().to_vec(),
                "u256" | "i256" => {
                    let mut b = vec![0; 32];
                    b[..8].copy_from_slice(&n.to_le_bytes());
                    b
                }
                "pubkey" | "publicKey" => self.identity(name).to_bytes().to_vec(),
                "string" => {
                    let text = format!("tare-{}", self.round);
                    let mut b = (text.len() as u32).to_le_bytes().to_vec();
                    b.extend(text.as_bytes());
                    b
                }
                "bytes" => vec![0, 0, 0, 0],
                other => bail!("generated corpus does not implement IDL argument type {other}"),
            });
        }
        if let Some(inner) = ty.get("option") {
            let _ = inner;
            return Ok(vec![0]);
        }
        if ty.get("vec").is_some() {
            return Ok(vec![0, 0, 0, 0]);
        }
        if let Some(array) = ty.get("array").and_then(Value::as_array) {
            let len = array[1]
                .as_u64()
                .ok_or_else(|| anyhow!("IDL array size is not an integer"))?;
            let mut out = Vec::new();
            for _ in 0..len {
                out.extend(self.encode(&array[0], name)?);
            }
            return Ok(out);
        }
        if let Some(defined) = ty.get("defined") {
            let name = defined
                .as_str()
                .or_else(|| defined["name"].as_str())
                .ok_or_else(|| anyhow!("IDL defined type has no name"))?;
            return self.encode_defined_with_generics(name,defined.get("generics").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]));
        }
        bail!("generated corpus does not implement IDL argument type {ty}")
    }
    fn encode_defined(&mut self, name: &str) -> Result<Vec<u8>> {
        self.encode_defined_with_generics(name,&[])
    }
    fn encode_defined_with_generics(&mut self,name:&str,arguments:&[Value])->Result<Vec<u8>> {
        let mut def = self.idl["types"]
            .as_array()
            .and_then(|ts| ts.iter().find(|t| t["name"].as_str() == Some(name)))
            .cloned()
            .ok_or_else(|| anyhow!("IDL type {name} has no definition"))?;
        let parameters=def.get("generics").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]);
        if parameters.len()!=arguments.len(){bail!("IDL type {name} needs {} generic arguments, found {}",parameters.len(),arguments.len())}
        let mut bindings=BTreeMap::new();
        for (parameter,argument) in parameters.iter().zip(arguments){
            let value=match required_str(parameter,"kind")?{
                "type"=>argument["type"].clone(),
                "const"=>{let value=argument.get("value").or_else(||argument.get("type")).context("IDL const generic value")?;let number=if let Some(number)=value.as_u64(){number}else{value.as_str().context("IDL const generic value")?.parse::<u64>()?};json!(number)},
                kind=>bail!("unsupported IDL generic kind {kind}"),
            };bindings.insert(required_str(parameter,"name")?.to_owned(),value);
        }
        fn substitute(value:&mut Value,bindings:&BTreeMap<String,Value>)->Result<()>{
            if let Some(name)=value.get("generic").and_then(Value::as_str){*value=bindings.get(name).with_context(||format!("unbound IDL generic {name}"))?.clone();return Ok(())}
            if value.get("kind").and_then(Value::as_str)==Some("const"){if let Some(name)=value.get("value").and_then(Value::as_str){if let Some(bound)=bindings.get(name){value["value"]=bound.clone();}}}
            match value{Value::Array(values)=>for value in values{substitute(value,bindings)?},Value::Object(values)=>for value in values.values_mut(){substitute(value,bindings)?},_=>{}}Ok(())
        }
        substitute(&mut def["type"],&bindings)?;
        let mut out = Vec::new();
        match required_str(&def["type"], "kind")? {
            "struct" => {
                let fields=match def["type"].get("fields"){None=>&[][..],Some(fields)=>fields.as_array().context("IDL struct fields must be an array")?.as_slice()};
                for f in fields {
                    let field=f["name"].as_str().unwrap_or("value");
                    if self.hints.pubkey_arrays.contains(&format!("{name}.{field}")){out.extend(self.identity(field).to_bytes());}
                    else {out.extend(self.encode(f.get("type").unwrap_or(f),field)?);}
                }
            }
            "enum" => {
                out.push(0);
                let variant = &array(&def["type"], "variants")?[0];
                if let Some(fields) = variant["fields"].as_array() {
                    for f in fields {
                        out.extend(self.encode(
                            f.get("type").unwrap_or(f),
                            f["name"].as_str().unwrap_or("value"),
                        )?);
                    }
                }
            }
            other => bail!("generated corpus does not implement IDL definition {other}"),
        }
        Ok(out)
    }
    fn add_instruction(&mut self, ix: &Value) -> Result<()> {
        let name = required_str(ix, "name")?;
        let mut data = discriminator(ix, "global")?;
        let mut alternate_arguments=Vec::new();
        for arg in array(ix, "args")? {
            let arg_name=required_str(arg,"name")?;
            let encoded=self.encode(&arg["type"],arg_name)?;
            if arg["type"]=="u8"&&(normalize(arg_name).contains("bump")||normalize(arg_name).contains("nonce")) {
                let original=[0u8,1,42,255][self.round as usize];
                if encoded!=[original]{alternate_arguments.push((arg_name.to_owned(),data.len(),original));}
            }
            data.extend(encoded);
        }
        let mut metas = Vec::new();
        let mut typed = Vec::new();
        let mut init = Vec::new();
        for a in flat_accounts(ix)? {
            let account_name = required_str(&a, "name")?;
            let hint = self.hint(name, account_name);
            let address = self.account_address(name,account_name,&hint)?;
            let signer = boolean(&a, "signer", "isSigner") || hint.cpi_signer || self.scenario.as_ref().is_some_and(|(_,s)|s.payers.contains(account_name));
            if signer && !hint.init && hint.account_type.is_none() {
                self.fund(account_name)
            }
            if hint.init {
                init.push(address);
                self.init_addresses.insert(address);
            }
            if let Some(len)=hint.raw_data_len {
                self.accounts.entry(address.to_string()).or_insert_with(||AccountState::from_account(address,&Account{lamports:program_rent(len),data:vec![0;len],owner:self.program,executable:false,rent_epoch:0}));
            }
            if let Some(ty) = hint.account_type.as_ref() {
                typed.push(address);
                if !self.init_types.contains(&(normalize(account_name),ty.clone()))
                    && !self.init_addresses.contains(&address)
                    && !self.accounts.contains_key(&address.to_string())
                {
                    let state=self.account_fixture(ty,&hint,name,account_name)?;
                    self.accounts.insert(
                        address.to_string(),
                        AccountState::from_account(address, &state),
                    );
                    if self.scenario.is_some() && hint.spl.is_none() {
                        self.fixture_provenance.push(FixtureProvenance {
                            address: address.to_string(), account_type: ty.clone(),
                            origin: "typed_snapshot".into(),
                            reason: "Independent source-resolved scenario starts from typed account state; this account is not claimed as created by an earlier instruction.".into(),
                        });
                    }
                }
            }
            if let Some((_,scenario))=&self.scenario {
                if let Some(debit)=scenario.initial_lamport_debits.get(account_name) {
                    let space=scenario.spaces.get(account_name).copied().unwrap_or(0);
                    self.accounts.insert(address.to_string(),AccountState::from_account(address,&Account{lamports:LiteSVM::new().minimum_balance_for_rent_exemption(space)+debit,data:vec![],owner:Address::default(),executable:false,rent_epoch:0}));
                }
                if let Some(owner)=scenario.owners.get(account_name).cloned() {
                    let owner=self.account_bindings.get(&(normalize(name),normalize(&owner))).copied().unwrap_or_else(||self.identity(&owner));
                    self.accounts.entry(address.to_string()).or_insert_with(||AccountState::from_account(address,&Account{lamports:10_000_000,data:vec![],owner,executable:false,rent_epoch:0})).owner=owner.to_string();
                }
            }
            metas.push(InputAccount {
                address: address.to_string(),
                signer,
                writable: boolean(&a, "writable", "isMut") || hint.writable || self.scenario.as_ref().is_some_and(|(_,s)|s.writable.contains(account_name)),
            });
        }
        let declared_count=metas.len();
        let remaining=self.hints.remaining_accounts.iter().find(|(instruction,_)|normalize(instruction)==normalize(name)).map(|(_,hints)|hints.clone()).unwrap_or_default();
        for remaining in remaining {
            let ty=remaining.account.account_type.as_ref().expect("source-derived remaining type");
            let prior=self.hints.instructions.iter().find_map(|(ix,fields)|fields.iter().find(|(_,hint)|(hint.init||hint.zero)&&hint.account_type.as_ref()==Some(ty)).map(|(field,hint)|(ix.clone(),field.clone(),hint.clone())));
            let address=if let Some((ix,field,hint))=prior {self.account_address(&ix,&field,&hint)?}else{
                let address=self.identity(&format!("__remaining_{name}_{}",remaining.index));
                let fixture=self.account_fixture(ty,&remaining.account,name,&remaining.name)?;
                self.accounts.insert(address.to_string(),AccountState::from_account(address,&fixture));address
            };
            typed.push(address);
            metas.push(InputAccount{address:address.to_string(),signer:false,writable:remaining.writable});
        }
        let instruction = InputInstruction {
            program: self.program.to_string(),
            name: name.to_owned(),
            accounts: metas,
            data_base64: BASE64.encode(&data),
        };
        let base = Case {
            label: format!("{name}:seed-{}", self.round),
            kind: "generated".into(),
            instructions: vec![instruction.clone()],
            commit: true,
            mutations: vec![],
        };
        // Keep the pre-resolution bump samples as additional inputs: supplying a
        // valid canonical bump must not erase the nonce/seed error paths.
        for (arg,offset,value) in alternate_arguments {
            let mut alternate=base.clone();alternate.commit=false;alternate.label=format!("{name}:alternate-{arg}");
            let mut bytes=data.clone();bytes[offset]=value;alternate.instructions[0].data_base64=BASE64.encode(bytes);self.cases.push(alternate);
        }
        for (index, account) in flat_accounts(ix)?.iter().enumerate() {
            if boolean(account,"optional","isOptional") || self.hint(name,required_str(account,"name")?).optional {
                let mut none=base.clone();none.commit=false;none.label=format!("{name}:optional-none-{index}");
                none.instructions[0].accounts[index]=InputAccount{address:self.program.to_string(),signer:false,writable:false};
                self.cases.push(none);
            }
        }
        for keep in declared_count..base.instructions[0].accounts.len(){
            let mut missing=base.clone();missing.commit=false;missing.label=format!("{name}:remaining-truncated-{}",keep-declared_count);missing.instructions[0].accounts.truncate(keep);self.cases.push(missing);
        }
        // Run malformed variants before the successful call, so init sees an uninitialized account.
        let mut short = base.clone();
        short.commit = false;
        short.label = format!("{name}:short-instruction");
        short.instructions[0].data_base64 = BASE64.encode(&data[..data.len().min(7)]);
        self.cases.push(short);
        let mut unknown = base.clone();
        unknown.commit = false;
        unknown.label = format!("{name}:unknown-discriminator");
        unknown.instructions[0].data_base64 = BASE64.encode([255; 8]);
        self.cases.push(unknown);
        if data.len() > 8 {
            let mut bad_args = base.clone();
            bad_args.commit = false;
            bad_args.label = format!("{name}:short-arguments");
            bad_args.instructions[0].data_base64 = BASE64.encode(&data[..8]);
            self.cases.push(bad_args);
        }
        for (i, meta) in instruction.accounts.iter().enumerate() {
            if meta.signer {
                let mut bad = base.clone();
                bad.commit = false;
                bad.label = format!("{name}:missing-signer-{i}");
                bad.instructions[0].accounts[i].signer = false;
                self.cases.push(bad.clone());
                if let Some(address) = typed.iter().find(|address| !init.contains(address)) {
                    bad.label = format!("{name}:multiple-faults-{i}");
                    bad.mutations.push(AccountMutation::Data {
                        address: address.to_string(),
                        data_base64: BASE64.encode([0; 3]),
                    });
                    self.cases.push(bad);
                }
            }
            if meta.writable {
                let mut bad = base.clone();
                bad.commit = false;
                bad.label = format!("{name}:readonly-{i}");
                bad.instructions[0].accounts[i].writable = false;
                self.cases.push(bad);
            }
        }
        if !instruction.accounts.is_empty() {
            let mut bad = base.clone();
            bad.commit = false;
            bad.label = format!("{name}:missing-accounts");
            bad.instructions[0].accounts.clear();
            self.cases.push(bad);
        }
        for address in typed.iter().filter(|a| !init.contains(a)) {
            for mutation in [
                AccountMutation::Owner {
                    address: address.to_string(),
                    owner: Address::default().to_string(),
                },
                AccountMutation::Data {
                    address: address.to_string(),
                    data_base64: BASE64.encode([255; 8]),
                },
                AccountMutation::Data {
                    address: address.to_string(),
                    data_base64: BASE64.encode([0; 3]),
                },
            ] {
                let mut bad = base.clone();
                bad.commit = false;
                bad.label = format!("{name}:invalid-account-{address}-{}", self.cases.len());
                bad.mutations.push(mutation);
                self.cases.push(bad);
            }
        }
        // Wrong seed uses a fresh account while preserving flags. Anchor must report the same first error.
        for (i, a) in flat_accounts(ix)?
            .iter()
            .enumerate()
            .filter(|(_, a)| a["pda"].is_object())
        {
            let _ = a;
            let mut bad = base.clone();
            bad.commit = false;
            bad.label = format!("{name}:wrong-seeds-{i}");
            let original = bad.instructions[0].accounts[i].address.clone();
            let wrong = self
                .identity(&format!("wrong-seeds-{name}-{i}"))
                .to_string();
            bad.instructions[0].accounts[i].address = wrong.clone();
            if !init.iter().any(|address| address.to_string() == original) {
                bad.mutations.push(AccountMutation::CopyAccount {
                    from: original,
                    address: wrong,
                });
            }
            self.cases.push(bad);
        }
        let mut wrong_program = base.clone();
        wrong_program.commit = false;
        wrong_program.label = format!("{name}:wrong-declared-program-id");
        wrong_program.instructions[0].program = wrong_program_address(self.program).to_string();
        self.cases.push(wrong_program);
        if let Some(account) = base.instructions[0].accounts.first() {
            let mut duplicate = base.clone();
            duplicate.commit = false;
            duplicate.label = format!("{name}:duplicate-account");
            duplicate.instructions[0].accounts.push(account.clone());
            self.cases.push(duplicate);
        }
        for (i, left) in instruction.accounts.iter().enumerate() {
            for (j, right) in instruction.accounts.iter().enumerate().skip(i + 1) {
                if left.writable && right.writable && (!right.signer || left.signer)
                    && left.address != right.address
                {
                    let mut duplicate = base.clone();
                    duplicate.commit = false;
                    duplicate.label = format!("{name}:duplicate-declared-{i}-{j}");
                    duplicate.instructions[0].accounts[j].address = left.address.clone();
                    self.cases.push(duplicate);
                }
            }
        }
        self.cases.push(base.clone());
        for repeat in 1..3 {
            let mut again = base.clone();
            again.label = format!("{name}:repeat-{repeat}");
            self.cases.push(again);
        }
        Ok(())
    }
    fn add_builtin_cases(&mut self, legacy_idl: bool) {
        let tag = 0x0a69e9a778bcf440u64.to_le_bytes();
        let variants: Vec<(&str, Vec<u8>)> = if legacy_idl { vec![
            ("tag-only", vec![]),
            ("unknown-variant", vec![255]),
            ("create-short", vec![0]),
            ("create-no-accounts", [vec![0], vec![0; 8]].concat()),
            ("create-buffer-no-accounts", vec![1]),
            ("write-short", vec![2]),
            ("write-no-accounts", vec![2, 0, 0, 0, 0]),
            ("write-short-payload", vec![2, 1, 0, 0, 0]),
            ("set-buffer-no-accounts", vec![3]),
            ("set-authority-short", vec![4]),
            ("set-authority-no-accounts", [vec![4], vec![0; 32]].concat()),
            ("close-no-accounts", vec![5]),
            ("resize-short", vec![6]),
            ("resize-no-accounts", [vec![6], vec![0; 8]].concat()),
        ] } else { vec![("unknown-tag", vec![])] };
        for (name, payload) in variants {
            self.cases.push(Case {
                label: format!("idl:{name}"),
                kind: "generated".into(),
                commit: false,
                instructions: vec![InputInstruction {
                    program: self.program.to_string(),
                    name: format!("__idl_{name}"),
                    accounts: vec![],
                    data_base64: BASE64.encode([tag.to_vec(), payload].concat()),
                }],
                mutations: vec![],
            });
        }
        self.cases.push(Case {
            label: "event-cpi:stub".into(),
            kind: "generated".into(),
            commit: false,
            instructions: vec![InputInstruction {
                program: self.program.to_string(),
                name: "__event_cpi".into(),
                accounts: vec![],
                data_base64: BASE64.encode(0x1d9acb512ea545e4u64.to_le_bytes()),
            }],
            mutations: vec![],
        });
    }
    fn idl_case(
        &mut self,
        name: &str,
        tag: u8,
        body: &[u8],
        accounts: Vec<(Address, bool, bool)>,
        commit: bool,
    ) {
        let mut data = 0x0a69e9a778bcf440u64.to_le_bytes().to_vec();
        data.push(tag);
        data.extend_from_slice(body);
        let case = Case {
            label: format!("idl:{name}"),
            kind: "generated".into(),
            commit,
            mutations: vec![],
            instructions: vec![InputInstruction {
                program: self.program.to_string(),
                name: format!("__idl_{name}"),
                data_base64: BASE64.encode(data),
                accounts: accounts
                    .into_iter()
                    .map(|(address, signer, writable)| InputAccount {
                        address: address.to_string(),
                        signer,
                        writable,
                    })
                    .collect(),
            }],
        };
        if commit {
            let mut missing = case.clone();
            missing.commit = false;
            missing.label.push_str(":missing-last-account");
            missing.instructions[0].accounts.pop();
            self.cases.push(missing);
            for (i, account) in case.instructions[0].accounts.iter().enumerate() {
                if account.signer {
                    let mut bad = case.clone();
                    bad.commit = false;
                    bad.label.push_str(&format!(":missing-signer-{i}"));
                    bad.instructions[0].accounts[i].signer = false;
                    self.cases.push(bad);
                }
                if account.writable {
                    let mut bad = case.clone();
                    bad.commit = false;
                    bad.label.push_str(&format!(":readonly-{i}"));
                    bad.instructions[0].accounts[i].writable = false;
                    self.cases.push(bad);
                }
            }
            if tag != 0 {
                let typed_count = if tag == 3 { 2 } else { 1 };
                for i in 0..typed_count {
                    let address = &case.instructions[0].accounts[i].address;
                    for (suffix, mutation) in [
                        (
                            "wrong-owner",
                            AccountMutation::Owner {
                                address: address.clone(),
                                owner: Address::default().to_string(),
                            },
                        ),
                        (
                            "short-account",
                            AccountMutation::Data {
                                address: address.clone(),
                                data_base64: BASE64.encode([0; 3]),
                            },
                        ),
                        (
                            "bad-discriminator",
                            AccountMutation::Data {
                                address: address.clone(),
                                data_base64: BASE64.encode([255; 8]),
                            },
                        ),
                    ] {
                        let mut bad = case.clone();
                        bad.commit = false;
                        bad.label.push_str(&format!(":{suffix}-{i}"));
                        bad.mutations.push(mutation);
                        self.cases.push(bad);
                    }
                }
            }
            if tag == 0 {
                let mut bad = case.clone();
                bad.commit = false;
                bad.label.push_str(":wrong-base");
                bad.instructions[0].accounts[2].address =
                    self.identity("__idl_wrong_base").to_string();
                self.cases.push(bad);
            }
            if tag == 3 {
                let mut bad = case.clone();
                bad.commit = false;
                bad.label.push_str(":aliased-buffer");
                bad.instructions[0].accounts[0].address =
                    bad.instructions[0].accounts[1].address.clone();
                self.cases.push(bad);
                let authority: Address = case.instructions[0].accounts[2]
                    .address
                    .parse()
                    .expect("generated authority");
                let header = |length: u32| {
                    let mut bytes = Sha256::digest(b"internal:IdlAccount")[..8].to_vec();
                    bytes.extend_from_slice(authority.as_ref());
                    bytes.extend_from_slice(&length.to_le_bytes());
                    bytes.push(0);
                    bytes
                };
                let mut short_target = case.clone();
                short_target.commit = false;
                short_target.label.push_str(":short-target");
                short_target.mutations.push(AccountMutation::Data {
                    address: case.instructions[0].accounts[1].address.clone(),
                    data_base64: BASE64.encode(header(0)),
                });
                self.cases.push(short_target);
                let mut long_buffer = case.clone();
                long_buffer.commit = false;
                long_buffer.label.push_str(":invalid-buffer-length");
                long_buffer.mutations.push(AccountMutation::Data {
                    address: case.instructions[0].accounts[0].address.clone(),
                    data_base64: BASE64.encode(header(1024)),
                });
                self.cases.push(long_buffer);
            }
            if tag == 5 {
                let mut bad = case.clone();
                bad.commit = false;
                bad.label.push_str(":self-close");
                bad.instructions[0].accounts[2].address =
                    bad.instructions[0].accounts[0].address.clone();
                self.cases.push(bad);
            }
        }
        self.cases.push(case);
    }
    fn add_idl_lifecycle(&mut self) -> Result<()> {
        let authority = self.identity("__idl_authority");
        let new_authority = self.identity("__idl_new_authority");
        self.fund("__idl_authority");
        self.fund("__idl_new_authority");
        let destination = self.transaction_payer;
        let buffer = self.identity("__idl_buffer");
        let (base, _) = Address::find_program_address(&[], &self.program);
        let canonical = Address::create_with_seed(&base, "anchor:idl", &self.program)?;
        let system = Address::default();
        let rent = LiteSVM::new().minimum_balance_for_rent_exemption(256);
        self.accounts.insert(
            buffer.to_string(),
            AccountState::from_account(
                buffer,
                &Account {
                    data: vec![0; 256],
                    lamports: rent,
                    owner: self.program,
                    executable: false,
                    rent_epoch: 0,
                },
            ),
        );
        self.idl_case(
            "create",
            0,
            &32u64.to_le_bytes(),
            vec![
                (authority, true, true),
                (canonical, false, true),
                (base, false, false),
                (system, false, false),
                (self.program, false, false),
            ],
            true,
        );
        self.idl_case(
            "resize",
            6,
            &256u64.to_le_bytes(),
            vec![
                (canonical, false, true),
                (authority, true, true),
                (system, false, false),
            ],
            true,
        );
        self.idl_case(
            "create-buffer",
            1,
            &[],
            vec![(buffer, false, true), (authority, true, false)],
            true,
        );
        let bytes = format!("tare-idl-seed-{}", self.round).into_bytes();
        let body = [(bytes.len() as u32).to_le_bytes().to_vec(), bytes].concat();
        self.idl_case(
            "write-buffer",
            2,
            &body,
            vec![(buffer, false, true), (authority, true, false)],
            true,
        );
        self.idl_case(
            "set-buffer-authority",
            4,
            new_authority.as_ref(),
            vec![(buffer, false, true), (authority, true, false)],
            true,
        );
        self.idl_case(
            "old-authority-write",
            2,
            &[0, 0, 0, 0],
            vec![(buffer, false, true), (authority, true, false)],
            false,
        );
        self.idl_case(
            "write-new-authority",
            2,
            &[1, 0, 0, 0, 42],
            vec![(buffer, false, true), (new_authority, true, false)],
            true,
        );
        self.idl_case(
            "set-canonical-authority",
            4,
            new_authority.as_ref(),
            vec![(canonical, false, true), (authority, true, false)],
            true,
        );
        self.idl_case(
            "set-buffer",
            3,
            &[],
            vec![
                (buffer, false, true),
                (canonical, false, true),
                (new_authority, true, false),
            ],
            true,
        );
        self.idl_case(
            "close-buffer",
            5,
            &[],
            vec![
                (buffer, false, true),
                (new_authority, true, false),
                (destination, false, true),
            ],
            true,
        );
        self.idl_case(
            "append-canonical",
            2,
            &[1, 0, 0, 0, 99],
            vec![(canonical, false, true), (new_authority, true, false)],
            true,
        );
        self.idl_case(
            "close-canonical",
            5,
            &[],
            vec![
                (canonical, false, true),
                (new_authority, true, false),
                (destination, false, true),
            ],
            true,
        );
        Ok(())
    }
    fn finish(self) -> Sequence {
        Sequence {
            label: format!("seed-{}-{}", self.seed, self.round),
            payer: self.transaction_payer.to_string(),
            starting_accounts: self.accounts.into_values().collect(),
            signer_seeds: self.seeds.into_values().collect(),
            cases: self.cases,
            fixture_provenance: self.fixture_provenance,
        }
    }
}

fn wrong_program_address(program: Address) -> Address {
    Address::from(<[u8; 32]>::from(Sha256::digest(format!(
        "tare:wrong-program:{program}"
    ))))
}

fn normalize(name: &str) -> String {
    name.chars()
        .filter(|c| *c != '_')
        .flat_map(char::to_lowercase)
        .collect()
}
fn snake(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push('_')
        }
        out.extend(c.to_lowercase());
    }
    out
}
fn required_str<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v[key]
        .as_str()
        .ok_or_else(|| anyhow!("IDL field {key} is missing or not a string"))
}
fn array<'a>(v: &'a Value, key: &str) -> Result<&'a Vec<Value>> {
    v[key]
        .as_array()
        .ok_or_else(|| anyhow!("IDL field {key} is missing or not an array"))
}
fn boolean(v: &Value, new: &str, old: &str) -> bool {
    v[new]
        .as_bool()
        .or_else(|| v[old].as_bool())
        .unwrap_or(false)
}
fn bytes(v: &Value) -> Result<Vec<u8>> {
    v.as_array()
        .ok_or_else(|| anyhow!("IDL bytes are not an array"))?
        .iter()
        .map(|b| {
            b.as_u64()
                .filter(|b| *b <= 255)
                .map(|b| b as u8)
                .ok_or_else(|| anyhow!("IDL byte is outside 0..255"))
        })
        .collect()
}
fn discriminator(ix: &Value, namespace: &str) -> Result<Vec<u8>> {
    if let Some(d) = ix.get("discriminator") {
        return bytes(d);
    }
    Ok(Sha256::digest(format!("{namespace}:{}", snake(required_str(ix, "name")?)))[..8].to_vec())
}
fn account_discriminator(idl: &Value, name: &str) -> Result<Vec<u8>> {
    if let Some(a) = idl["accounts"]
        .as_array()
        .and_then(|xs| xs.iter().find(|a| a["name"].as_str() == Some(name)))
    {
        if let Some(d) = a.get("discriminator") {
            return bytes(d);
        }
    }
    Ok(Sha256::digest(format!("account:{name}"))[..8].to_vec())
}
fn flat_accounts(ix: &Value) -> Result<Vec<Value>> {
    fn visit<'a>(xs:&'a [Value],prefix:&str,out:&mut Vec<(String,&'a Value)>){
        for account in xs {let name=account["name"].as_str().unwrap_or("");let path=if prefix.is_empty(){name.to_owned()}else{format!("{prefix}.{name}")};
            if let Some(inner)=account["accounts"].as_array(){visit(inner,&path,out)}else{out.push((path,account))}
        }
    }
    let mut leaves=Vec::new();visit(array(ix,"accounts")?,"",&mut leaves);
    let mut counts=BTreeMap::new();for (_,account) in &leaves {*counts.entry(required_str(account,"name")?).or_insert(0usize)+=1;}
    Ok(leaves.iter().map(|(path,account)|{let mut copy=(*account).clone();if counts[account["name"].as_str().unwrap()]>1 {copy["name"]=json!(path);}copy}).collect())
}

/// Executes every sequence independently, and compares after each submitted transaction.
pub fn run(program_id: &str, stock: &[u8], tare: &[u8], corpus: &Corpus) -> Result<RunReport> {
    run_inner(program_id, stock, tare, corpus, &[], #[cfg(feature="trace")] None)
}
fn run_inner(program_id: &str, stock: &[u8], tare: &[u8], corpus: &Corpus, external_programs: &[(String, Vec<u8>)], #[cfg(feature="trace")] recorder: Option<&trace::Recorder>) -> Result<RunReport> {
    let program: Address = program_id.parse()?;
    let mut report = RunReport {
        corpus_hash: hex::encode(Sha256::digest(serde_json::to_vec(corpus)?)),
        counts: CorpusCounts::default(),
        comparisons: Comparisons::default(),
        instructions: vec![],
        first_divergence: None,
        outcomes: vec![],
    };
    let mut measurements: BTreeMap<String, (Vec<u64>, Vec<u64>)> = BTreeMap::new();
    for sequence in &corpus.sequences {
        #[cfg(feature="trace")]
        let initial = if recorder.is_some() { LiteSVM::new_debuggable(true) } else { LiteSVM::new() };
        #[cfg(not(feature="trace"))]
        let initial = LiteSVM::new();
        let mut initial = initial.with_transaction_history(0)
            .with_log_bytes_limit(None);
        #[cfg(feature="trace")]
        if let Some(recorder)=recorder { initial.set_invocation_inspect_callback(recorder.clone()); }
        for account in &sequence.starting_accounts {
            initial.set_account(account.address.parse()?, account.account()?)?;
        }
        for (address, elf) in external_programs {
            let address: Address = address.parse()?;
            if address != program { initial.add_program(address, elf).context("load external stock ELF")?; }
        }
        let signers: Vec<Keypair> = sequence
            .signer_seeds
            .iter()
            .map(|seed| Keypair::new_from_array(*seed))
            .collect();
        let payer = signers
            .iter()
            .find(|key| key.pubkey().to_string() == sequence.payer)
            .ok_or_else(|| anyhow!("sequence {} has no fee payer keypair", sequence.label))?;
        let mut gross = initial.clone();
        gross
            .add_program(program, stock)
            .context("load stock ELF")?;
        let mut net = initial;
        net.add_program(program, tare).context("load Tare ELF")?;
        // add_program writes a fresh ProgramData; a starting ProgramData keeps its header (upgrade authority).
        let programdata=Address::find_program_address(&[program.as_ref()],&spl::LOADER_UPGRADEABLE.parse()?).0;
        if let Some(start)=sequence.starting_accounts.iter().find(|a|a.address==programdata.to_string()) {
            let start=start.account()?;
            for svm in [&mut gross,&mut net] {
                let mut loaded=svm.get_account(&programdata).context("loaded ProgramData")?;
                loaded.data[..start.data.len().min(45)].copy_from_slice(&start.data[..start.data.len().min(45)]);
                svm.set_account(programdata,loaded)?;
            }
        }
        for alias in &corpus.program_aliases {
            let address: Address = alias.parse()?;
            gross
                .add_program(address, stock)
                .context("load stock ELF under alternate program id")?;
            net.add_program(address, tare)
                .context("load Tare ELF under alternate program id")?;
        }
        for address in &corpus.executable_fixtures {
            if external_programs.iter().any(|(program,_)|program==address){continue;}
            let address:Address=address.parse()?;
            gross.add_program(address,stock).context("load shared executable validation fixture")?;
            net.add_program(address,stock).context("load shared executable validation fixture")?;
        }
        let mut tracked: BTreeSet<Address> = sequence
            .starting_accounts
            .iter()
            .map(|a| a.address.parse())
            .collect::<std::result::Result<_, _>>()?;
        // The program's ProgramData holds its ELF, which differs between the builds by design.
        tracked.remove(&programdata);
        for case in &sequence.cases {
            let mut gross_fork = gross.clone();
            let mut net_fork = net.clone();
            for mutation in &case.mutations {
                apply_mutation(&mut gross_fork, mutation)?;
                apply_mutation(&mut net_fork, mutation)?;
            }
            let instructions = case
                .instructions
                .iter()
                .map(|ix| input_instruction(ix))
                .collect::<Result<Vec<_>>>()?;
            for ix in &instructions {
                for account in &ix.accounts {
                    if account.pubkey != program && account.pubkey != programdata {
                        tracked.insert(account.pubkey);
                    }
                }
            }
            let message = Message::new(&instructions, Some(&payer.pubkey()));
            let required: Vec<&Keypair> = signers
                .iter()
                .filter(|key| {
                    message.account_keys[..message.header.num_required_signatures as usize]
                        .contains(&key.pubkey())
                })
                .collect();
            let mut transaction = Transaction::new_unsigned(message);
            transaction.try_sign(&required, gross_fork.latest_blockhash()).with_context(|| {
                let unresolved = transaction.message.account_keys[..transaction.message.header.num_required_signatures as usize]
                    .iter()
                    .filter(|address| !required.iter().any(|key| key.pubkey() == **address))
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();
                format!(
                    "generated account resolver cannot sign sequence '{}' input '{}' (instructions: {}; unresolved signer accounts: {})",
                    sequence.label,
                    case.label,
                    case.instructions.iter().map(|ix| ix.name.as_str()).collect::<Vec<_>>().join(", "),
                    unresolved.join(", ")
                )
            })?;
            let before: Vec<AccountState> = tracked
                .iter()
                .filter_map(|key| {
                    gross_fork
                        .get_account(key)
                        .map(|a| AccountState::from_account(*key, &a))
                })
                .collect();
            #[cfg(feature="trace")]
            if let Some(recorder)=recorder {recorder.context(&sequence.label,&case.label,"gross");}
            let gross_result = gross_fork.send_transaction(transaction.clone());
            #[cfg(feature="trace")]
            if let Some(recorder)=recorder {recorder.context(&sequence.label,&case.label,"net");}
            let net_result = net_fork.send_transaction(transaction.clone());
            let (gross_meta, gross_error) = outcome(gross_result);
            let (net_meta, net_error) = outcome(net_result);
            let mut differences = Vec::new();
            for key in &tracked {
                let a = gross_fork.get_account(key);
                let b = net_fork.get_account(key);
                report.comparisons.account_bytes += 1;
                report.comparisons.lamports += 1;
                if a.as_ref().map(|a| &a.data) != b.as_ref().map(|a| &a.data) {
                    differences.push(json!({"kind":"account_bytes","account":key.to_string(),"gross":a.as_ref().map(|a|BASE64.encode(&a.data)),"net":b.as_ref().map(|a|BASE64.encode(&a.data))}));
                }
                if a.as_ref().map(|a| a.lamports) != b.as_ref().map(|a| a.lamports) {
                    differences.push(json!({"kind":"lamports","account":key.to_string(),"gross":a.as_ref().map(|a|a.lamports),"net":b.as_ref().map(|a|a.lamports)}));
                }
                if a.as_ref().map(|a| (a.owner, a.executable, a.rent_epoch))
                    != b.as_ref().map(|a| (a.owner, a.executable, a.rent_epoch))
                {
                    differences.push(json!({"kind":"account_metadata","account":key.to_string(),"gross":a.as_ref().map(|a|json!({"owner":a.owner.to_string(),"executable":a.executable,"rentEpoch":a.rent_epoch})),"net":b.as_ref().map(|a|json!({"owner":a.owner.to_string(),"executable":a.executable,"rentEpoch":a.rent_epoch}))}));
                }
            }
            let gross_logs = observable_logs(&gross_meta.logs);
            let net_logs = observable_logs(&net_meta.logs);
            if gross_logs != net_logs {
                differences.push(json!({"kind":"logs","gross":gross_logs,"net":net_logs}));
            }
            if gross_meta.return_data != net_meta.return_data {
                differences.push(json!({"kind":"return_data","gross":return_data(&gross_meta),"net":return_data(&net_meta)}));
            }
            if gross_error != net_error {
                differences.push(json!({"kind":"error","gross":gross_error,"net":net_error}));
            }
            report.comparisons.inputs += 1;
            report.comparisons.logs += 1;
            report.comparisons.return_data += 1;
            report.comparisons.errors += 1;
            match case.kind.as_str() {
                "tests" => report.counts.tests += 1,
                "mainnet" => report.counts.mainnet += 1,
                _ => report.counts.generated += 1,
            }
            let success = gross_error.is_none() && net_error.is_none();
            let single_instruction = (case.instructions.len() == 1).then(|| &case.instructions[0]);
            let instruction_data = single_instruction.map(|ix| BASE64.decode(&ix.data_base64)).transpose()?;
            let discriminator = instruction_data.as_ref().map(|data| hex::encode(&data[..data.len().min(8)]));
            report.outcomes.push(json!({"label":case.label,"sequence":sequence.label,"kind":case.kind,"success":success,"gross":gross_meta.compute_units_consumed,"net":net_meta.compute_units_consumed,"grossError":gross_error,"netError":net_error,"targetProgram":program_id,"instructionName":single_instruction.map(|ix| &ix.name),"discriminator":discriminator,"instructionDataBase64":single_instruction.map(|ix| &ix.data_base64),"instructionGross":single_instruction.map(|_| gross_meta.compute_units_consumed),"instructionNet":single_instruction.map(|_| net_meta.compute_units_consumed)}));
            if success && case.instructions.len() == 1 {
                let samples = measurements
                    .entry(case.instructions[0].name.clone())
                    .or_default();
                samples.0.push(gross_meta.compute_units_consumed);
                samples.1.push(net_meta.compute_units_consumed);
            }
            if !differences.is_empty() && report.first_divergence.is_none() {
                // The pre-state and signed transaction are sufficient to replay this precise failing input.
                let accounts:Vec<Value>=before.iter().map(|a|json!({"address":a.address,"owner":a.owner,"lamports":a.lamports,"executable":a.executable,"dataBase64":a.data_base64,"rentEpoch":a.rent_epoch})).collect();
                report.first_divergence = Some(
                    json!({"input":{"kind":case.kind,"label":case.label,"transactionBase64":BASE64.encode(bincode::serialize(&transaction)?),"accounts":accounts,"slot":null},"instructionIndex":0,"differences":differences}),
                );
            }
            if case.commit {
                gross = gross_fork;
                net = net_fork;
            }
        }
    }
    for identity in &corpus.instructions {
        let samples = measurements.remove(&identity.name).unwrap_or_default();
        let gross = median(samples.0);
        let net = median(samples.1.clone());
        report.instructions.push(InstructionMeasurement {
            name: identity.name.clone(),
            discriminator: identity.discriminator.clone(),
            inputs: samples.1.len() as u64,
            gross,
            net,
            tare: gross.zip(net).map(|(g, n)| g as i64 - n as i64),
        });
    }
    Ok(report)
}
fn apply_mutation(svm: &mut LiteSVM, mutation: &AccountMutation) -> Result<()> {
    let address = match mutation {
        AccountMutation::Owner { address, .. }
        | AccountMutation::Data { address, .. }
        | AccountMutation::CopyAccount { address, .. } => address,
    }
    .parse()?;
    let mut account = svm.get_account(&address).unwrap_or(Account {
        lamports: 1_000_000_000,
        data: vec![],
        owner: Address::default(),
        executable: false,
        rent_epoch: 0,
    });
    match mutation {
        AccountMutation::CopyAccount { from, .. } => {
            account = svm.get_account(&from.parse()?).unwrap_or(account)
        }
        AccountMutation::Owner { owner, .. } => account.owner = owner.parse()?,
        AccountMutation::Data { data_base64, .. } => account.data = BASE64.decode(data_base64)?,
    }
    svm.set_account(address, account)?;
    Ok(())
}
fn input_instruction(ix: &InputInstruction) -> Result<Instruction> {
    Ok(Instruction {
        program_id: ix.program.parse()?,
        accounts: ix
            .accounts
            .iter()
            .map(|a| {
                Ok(AccountMeta {
                    pubkey: a.address.parse()?,
                    is_signer: a.signer,
                    is_writable: a.writable,
                })
            })
            .collect::<Result<_>>()?,
        data: BASE64.decode(&ix.data_base64)?,
    })
}
fn outcome(result: litesvm::types::TransactionResult) -> (TransactionMetadata, Option<String>) {
    match result {
        Ok(meta) => (meta, None),
        Err(failed) => (failed.meta, Some(format!("{:?}", failed.err))),
    }
}
fn return_data(meta: &TransactionMetadata) -> Value {
    json!({"program":meta.return_data.program_id.to_string(),"dataBase64":BASE64.encode(&meta.return_data.data)})
}
fn median(mut values: Vec<u64>) -> Option<u64> {
    if values.is_empty() {
        None
    } else {
        values.sort_unstable();
        Some(values[values.len() / 2])
    }
}

/// Only the runtime's exact compute-consumption grammar is ignored. User logs are retained.
pub fn observable_logs(logs: &[String]) -> Vec<String> {
    logs.iter()
        .filter(|line| !runtime_compute_line(line))
        .cloned()
        .collect()
}
fn runtime_compute_line(line: &str) -> bool {
    let p: Vec<&str> = line.split(' ').collect();
    p.len() == 8
        && p[0] == "Program"
        && p[1].parse::<Address>().is_ok()
        && p[2] == "consumed"
        && p[3].parse::<u64>().is_ok()
        && p[4] == "of"
        && p[5].parse::<u64>().is_ok()
        && p[6] == "compute"
        && p[7] == "units"
}

#[cfg(feature = "trace")]
pub mod trace;
#[cfg(feature = "trace")]
pub fn run_traced(program_id: &str, stock: &[u8], tare: &[u8], corpus: &Corpus) -> Result<(RunReport, Vec<trace::InvocationTrace>)> {
    let recorder = trace::Recorder::default();
    let report = run_inner(program_id, stock, tare, corpus, &[], Some(&recorder))?;
    Ok((report, recorder.records()))
}

/// Both executions receive identical external stock programs for CPI.
pub fn run_with_programs(program_id: &str, stock: &[u8], tare: &[u8], corpus: &Corpus, external_programs: &[(String, Vec<u8>)]) -> Result<RunReport> {
    run_inner(program_id, stock, tare, corpus, external_programs, #[cfg(feature="trace")] None)
}

pub mod recorded;
pub use recorded::{RecordedCorpus, RecordedProgram, RecordedAccount, RecordedTransaction, RecordedSequence, ValidatorOutcome, RecordedRun, RecordedProvenance, run_recorded, merge_recorded};
