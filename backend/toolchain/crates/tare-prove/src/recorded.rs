//! Replay stock-suite recordings, keeping validator reproduction separate from equivalence.
use crate::*;
use solana_transaction::versioned::VersionedTransaction;
use solana_message::{SanitizedMessage, SanitizedVersionedMessage};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordedCorpus {
    pub schema_version: String,
    pub kind: String,
    pub target_program: String,
    pub stock_build_hash: String,
    #[serde(default)]
    pub capture_errors: Vec<String>,
    pub source_commit: String,
    pub programs: Vec<RecordedProgram>,
    pub sequences: Vec<RecordedSequence>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordedProgram {
    pub address: String,
    pub elf_base64: String,
    pub sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordedSequence {
    pub label: String,
    pub genesis_hash: String,
    pub transactions: Vec<RecordedTransaction>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordedAccount {
    pub address: String,
    #[serde(serialize_with = "crate::json_u64::serialize_json", deserialize_with = "crate::json_u64::deserialize_optional_json")]
    pub account: Option<Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordedTransaction {
    pub label: String,
    #[serde(default)]
    #[serde(serialize_with = "crate::json_u64::serialize_json", deserialize_with = "crate::json_u64::deserialize_optional_json")]
    pub input: Option<Value>,
    pub stage: String,
    pub transaction_base64: String,
    pub signature: Option<String>,
    #[serde(with = "crate::json_u64")]
    pub pre_slot: u64,
    #[serde(with = "crate::json_u64")]
    pub slot: u64,
    pub pre_accounts: Vec<RecordedAccount>,
    pub post_accounts: Vec<RecordedAccount>,
    #[serde(default)]
    pub clock_exact: bool,
    pub validator: ValidatorOutcome,
}
impl RecordedTransaction {
    pub fn replay_input(&self) -> Value { self.replay_input_for("tests") }
    pub fn replay_input_for(&self, kind: &str) -> Value {
        let accounts:Vec<Value>=self.pre_accounts.iter().filter_map(|entry|entry.account.as_ref().map(|a|json!({"address":entry.address,"owner":a["owner"],"lamports":a["lamports"],"executable":a["executable"],"dataBase64":a["dataBase64"],"rentEpoch":a["rentEpoch"]}))).collect();
        crate::json_u64::canonicalize(&json!({"kind":kind,"label":self.label,"transactionBase64":self.transaction_base64,"accounts":accounts,"slot":crate::json_u64::value(self.pre_slot)}))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidatorOutcome {
    #[serde(serialize_with = "crate::json_u64::serialize_json", deserialize_with = "crate::json_u64::deserialize_json")]
    pub error: Value,
    pub logs: Vec<String>,
    #[serde(serialize_with = "crate::json_u64::serialize_json", deserialize_with = "crate::json_u64::deserialize_optional_json")]
    pub return_data: Option<Value>,
    #[serde(with = "crate::json_u64::option")]
    pub compute_units: Option<u64>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reproduction {
    #[serde(with = "crate::json_u64")]
    pub inputs: u64,
    #[serde(with = "crate::json_u64")]
    pub matched: u64,
    #[serde(with = "crate::json_u64")]
    pub failed: u64,
    #[serde(with = "crate::json_u64")]
    pub clock_exact_inputs: u64,
    #[serde(serialize_with = "crate::json_u64::serialize_json")]
    pub first_difference: Option<Value>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordedRun {
    pub differential: RunReport,
    pub validator_reproduction: Reproduction,
    pub captured_stock_hash: String,
    pub replay_stock_hash: String,
    pub same_stock_binary: bool,
}

// JSON transports may use decimal strings even for small typed integers.
fn normalize_replay_integers(input: &Value) -> Value {
    let mut input = crate::json_u64::canonicalize(input);
    if let Some(slot) = crate::json_u64::integer(&input["slot"]) {
        input["slot"] = crate::json_u64::value(slot);
    }
    if let Some(accounts) = input["accounts"].as_array_mut() {
        for account in accounts {
            for key in ["lamports", "rentEpoch"] {
                if let Some(number) = crate::json_u64::integer(&account[key]) {
                    account[key] = crate::json_u64::value(number);
                }
            }
        }
    }
    input
}

fn integer(value: &Value, key: &str) -> Result<u64> {
    crate::json_u64::integer(&value[key]).with_context(||format!("recorded account {key} is not an unsigned integer"))
}

fn decode_account(value: &Value) -> Result<Account> {
    Ok(Account {
        owner: value["owner"].as_str().context("recorded owner")?.parse()?,
        lamports: integer(value, "lamports")?,
        data: BASE64.decode(
            value["dataBase64"]
                .as_str()
                .context("recorded account data")?,
        )?,
        executable: value["executable"]
            .as_bool()
            .context("recorded executable")?,
        rent_epoch: integer(value, "rentEpoch")?,
    })
}
fn snapshot_map(accounts: &[RecordedAccount]) -> Result<BTreeMap<Address, Option<Account>>> {
    accounts
        .iter()
        .map(|a| {
            Ok((
                a.address.parse()?,
                a.account.as_ref().map(decode_account).transpose()?,
            ))
        })
        .collect()
}
fn account_equal(a: &Option<Account>, b: &Option<Account>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}
fn fresh(
    accounts: &BTreeMap<Address, Account>,
    program: Address,
    elf: &[u8],
    externals: &[(Address, Vec<u8>)],
) -> Result<LiteSVM> {
    let mut svm = LiteSVM::new()
        .with_blockhash_check(false)
        .with_transaction_history(0)
        .with_log_bytes_limit(None);
    for (address, account) in accounts {
        if !account.executable {
            svm.set_account(*address, account.clone())?;
        }
    }
    for (address, bytes) in externals {
        if *address != program {
            svm.add_program(*address, bytes)?;
        }
    }
    svm.add_program(program, elf)?;
    let loader:Address="BPFLoaderUpgradeab1e11111111111111111111111".parse()?;
    let programdata=Address::find_program_address(&[program.as_ref()],&loader).0;
    if let Some(captured)=accounts.get(&programdata) {
        if captured.data.len()<45 || captured.data[..4]!=[3,0,0,0] {bail!("invalid captured ProgramData header")}
        let mut loaded=svm.get_account(&programdata).context("loaded ProgramData")?;
        loaded.data[..45].copy_from_slice(&captured.data[..45]);
        loaded.lamports=captured.lamports;loaded.rent_epoch=captured.rent_epoch;
        svm.set_account(programdata,loaded)?;
    }
    Ok(svm)
}
fn execute(
    svm: &mut LiteSVM,
    tx: &VersionedTransaction,
    simulation: bool,
) -> Result<(TransactionMetadata, Value)> {
    if simulation {
        match svm.simulate_transaction(tx.clone()) {
            Ok(r) => Ok((r.meta, Value::Null)),
            Err(e) => Ok((e.meta, serde_json::to_value(e.err)?)),
        }
    } else {
        match svm.send_transaction(tx.clone()) {
            Ok(meta) => Ok((meta, Value::Null)),
            Err(e) => Ok((e.meta, serde_json::to_value(e.err)?)),
        }
    }
}
fn compare_accounts(
    a: &Option<Account>,
    b: &Option<Account>,
    address: Address,
    out: &mut Vec<Value>,
) {
    if a.as_ref().map(|v| &v.data) != b.as_ref().map(|v| &v.data) {
        out.push(json!({"kind":"account_bytes","account":address.to_string(),"gross":a.as_ref().map(|v|BASE64.encode(&v.data)),"net":b.as_ref().map(|v|BASE64.encode(&v.data))}));
    }
    if a.as_ref().map(|v| v.lamports) != b.as_ref().map(|v| v.lamports) {
        out.push(json!({"kind":"lamports","account":address.to_string(),"gross":a.as_ref().map(|v|v.lamports),"net":b.as_ref().map(|v|v.lamports)}));
    }
    if a.as_ref().map(|v| (v.owner, v.executable, v.rent_epoch))
        != b.as_ref().map(|v| (v.owner, v.executable, v.rent_epoch))
    {
        out.push(json!({"kind":"account_metadata","account":address.to_string(),"gross":a.as_ref().map(|v|json!({"owner":v.owner.to_string(),"executable":v.executable,"rentEpoch":v.rent_epoch})),"net":b.as_ref().map(|v|json!({"owner":v.owner.to_string(),"executable":v.executable,"rentEpoch":v.rent_epoch}))}));
    }
}
/// Replay frames in order. Recorded external changes update both state maps;
/// program-produced differences survive into the next frame and are never filtered.
pub fn run_recorded(stock: &[u8], tare: &[u8], corpus: &RecordedCorpus) -> Result<RecordedRun> {
    if corpus.schema_version != "1" || !matches!(corpus.kind.as_str(),"tests"|"mainnet") {
        bail!("unsupported recorded corpus schema or kind")
    }
    let replay_stock_hash = hex::encode(Sha256::digest(stock));
    if !corpus.capture_errors.is_empty() {
        bail!(
            "recording is incomplete: {}",
            corpus.capture_errors.join("; ")
        )
    }
    if corpus
        .sequences
        .iter()
        .all(|sequence| sequence.transactions.is_empty())
    {
        bail!("recording contains no transactions")
    }
    let program: Address = corpus.target_program.parse()?;
    let externals = corpus
        .programs
        .iter()
        .map(|p| {
            let bytes = BASE64.decode(&p.elf_base64)?;
            if hex::encode(Sha256::digest(&bytes)) != p.sha256 {
                bail!("recorded external ELF hash mismatch: {}", p.address)
            }
            Ok((p.address.parse()?, bytes))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut differential = RunReport {
        corpus_hash: hex::encode(Sha256::digest(serde_json::to_vec(corpus)?)),
        counts: CorpusCounts::default(),
        comparisons: Comparisons::default(),
        instructions: vec![],
        first_divergence: None,
        outcomes: vec![],
    };
    let mut reproduction = Reproduction {
        inputs: 0,
        matched: 0,
        failed: 0,
        clock_exact_inputs: 0,
        first_difference: None,
    };
    for sequence in &corpus.sequences {
        let mut stock_state = BTreeMap::new();
        let mut tare_state = BTreeMap::new();
        let mut previous = BTreeMap::new();
        for frame in &sequence.transactions {
            let input = frame.replay_input_for(&corpus.kind);
            if frame.input.as_ref().is_some_and(|projection| {
                // Recordings predating rentEpoch in ReplayInput still carry it in
                // rich preAccounts. Fill only that omitted optional field; an
                // explicitly supplied value must agree with the snapshot.
                let mut projection = projection.clone();
                if let (Some(actual), Some(expected)) = (projection["accounts"].as_array_mut(), input["accounts"].as_array()) {
                    for (actual, expected) in actual.iter_mut().zip(expected) {
                        if let Some(account) = actual.as_object_mut() {
                            account.entry("rentEpoch").or_insert_with(|| expected["rentEpoch"].clone());
                        }
                    }
                }
                normalize_replay_integers(&projection) != normalize_replay_integers(&input)
            }) {
                bail!(
                    "recorded ReplayInput differs from rich pre-state: {}",
                    frame.label
                )
            }
            let before = snapshot_map(&frame.pre_accounts)?;
            let after = snapshot_map(&frame.post_accounts)?;
            // Differences between observed frames are environmental inputs (funding,
            // sysvar progression, or test setup); apply each identically to both sides.
            for (address, account) in &before {
                if !previous.contains_key(address)
                    || !account_equal(previous.get(address).unwrap(), account)
                {
                    match account {
                        Some(a) => {
                            stock_state.insert(*address, a.clone());
                            tare_state.insert(*address, a.clone());
                        }
                        None => {
                            stock_state.remove(address);
                            tare_state.remove(address);
                        }
                    }
                }
            }
            let mut stock_vm = fresh(&stock_state, program, stock, &externals)?;
            let mut tare_vm = fresh(&tare_state, program, tare, &externals)?;
            let raw = BASE64.decode(&frame.transaction_base64)?;
            let transaction: VersionedTransaction =
                bincode::deserialize(&raw).context("decode recorded signed transaction")?;
            // Use the same AccountsDb address loader as LiteSVM, including its
            // Clock/SlotHashes lookup-table activation rules. Invalid messages
            // remain executable proof inputs; they simply have no CU attribution.
            let resolved = SanitizedVersionedMessage::try_new(transaction.message.clone())
                .ok()
                .and_then(|message| SanitizedMessage::try_new(
                    message, stock_vm.accounts_db(), &Default::default(),
                ).ok());
            let single_target = resolved.as_ref().and_then(|message| {
                let keys=message.account_keys();
                let instructions=message.instructions();
                (instructions.len()==1)
                    .then(|| &instructions[0])
                    .filter(|ix|keys.get(ix.program_id_index as usize)==Some(&program))
            });
            let discriminator =
                single_target.map(|ix| hex::encode(&ix.data[..ix.data.len().min(8)]));
            let simulation = match frame.stage.as_str() {
                "preflight_rejected" => true,
                "landed" => false,
                other => bail!("unsupported recorded transaction stage: {other}"),
            };
            let (a, ae) = execute(&mut stock_vm, &transaction, simulation)?;
            let (b, be) = execute(&mut tare_vm, &transaction, simulation)?;
            let mut difference = Vec::new();
            let mut reproduced = Vec::new();
            if !frame.clock_exact {
                reproduced.push(json!({"kind":"clock_capture","expected":"exact execution-bank Clock","actual":"snapshot Clock only"}));
            }
            for address in before
                .keys()
                .chain(after.keys())
                .copied()
                .collect::<BTreeSet<_>>()
            {
                // ELF bytes differ intentionally and are identified by their hashes.
                if externals.iter().any(|(p, _)| *p == address) || address == program {
                    continue;
                }
                let mut sa = stock_vm.get_account(&address);
                let mut sb = tare_vm.get_account(&address);
                // Header-only ProgramData snapshots explicitly exclude the selected ELF.
                let header_only=before.get(&address).and_then(Option::as_ref).is_some_and(|a|a.owner.to_string()=="BPFLoaderUpgradeab1e11111111111111111111111" && a.data.len()==45 && a.data[..4]==[3,0,0,0]);
                if header_only {for account in [&mut sa,&mut sb] {if let Some(account)=account {account.data.truncate(45);}}}
                differential.comparisons.account_bytes += 1;
                differential.comparisons.lamports += 1;
                compare_accounts(&sa, &sb, address, &mut difference);
                if let Some(expected) = after.get(&address) {
                    compare_accounts(expected, &sa, address, &mut reproduced);
                }
                match sa {
                    Some(v) => {
                        stock_state.insert(address, v);
                    }
                    None => {
                        stock_state.remove(&address);
                    }
                }
                match sb {
                    Some(v) => {
                        tare_state.insert(address, v);
                    }
                    None => {
                        tare_state.remove(&address);
                    }
                }
            }
            let al = observable_logs(&a.logs);
            let bl = observable_logs(&b.logs);
            let expected_logs = observable_logs(&frame.validator.logs);
            if al != bl {
                difference.push(json!({"kind":"logs","gross":al,"net":bl}));
            }
            if a.return_data != b.return_data {
                difference.push(
                    json!({"kind":"return_data","gross":return_data(&a),"net":return_data(&b)}),
                );
            }
            if ae != be {
                difference.push(json!({"kind":"error","gross":ae,"net":be}));
            }
            if al != expected_logs {
                reproduced.push(json!({"kind":"logs","expected":expected_logs,"actual":al}));
            }
            if ae != frame.validator.error {
                reproduced
                    .push(json!({"kind":"error","expected":frame.validator.error,"actual":ae}));
            }
            if let Some(units) = frame.validator.compute_units {
                if units != a.compute_units_consumed {
                    reproduced.push(json!({"kind":"compute_units","expected":units,"actual":a.compute_units_consumed}));
                }
            }
            let expected_return = frame
                .validator
                .return_data
                .as_ref()
                .map(|r| json!({"program":r["programId"],"dataBase64":r["data"][0]}));
            let actual_return = if a.return_data.data.is_empty() {
                None
            } else {
                Some(return_data(&a))
            };
            if expected_return != actual_return {
                reproduced.push(
                    json!({"kind":"return_data","expected":expected_return,"actual":actual_return}),
                );
            }
            reproduction.inputs += 1;
            reproduction.clock_exact_inputs += u64::from(frame.clock_exact);
            if reproduced.is_empty() {
                reproduction.matched += 1
            } else {
                reproduction.failed += 1;
                if reproduction.first_difference.is_none() {
                    reproduction.first_difference = Some(
                        json!({"sequence":sequence.label,"label":frame.label,"differences":reproduced}),
                    );
                }
            }
            differential.comparisons.inputs += 1;
            differential.comparisons.logs += 1;
            differential.comparisons.return_data += 1;
            differential.comparisons.errors += 1;
            if corpus.kind=="mainnet" { differential.counts.mainnet+=1; } else { differential.counts.tests+=1; }
            let instruction_gross =
                single_target.and_then(|_| target_units(&a.logs, &corpus.target_program));
            let instruction_net =
                single_target.and_then(|_| target_units(&b.logs, &corpus.target_program));
            differential.outcomes.push(json!({"label":frame.label,"sequence":sequence.label,"kind":corpus.kind,"success":ae.is_null()&&be.is_null(),"gross":a.compute_units_consumed,"net":b.compute_units_consumed,"grossError":ae,"netError":be,"stage":frame.stage,"targetProgram":corpus.target_program,"discriminator":discriminator,"instructionDataBase64":single_target.map(|ix|BASE64.encode(&ix.data)),"instructionGross":instruction_gross,"instructionNet":instruction_net}));
            if !difference.is_empty() && differential.first_divergence.is_none() {
                differential.first_divergence =
                    Some(json!({"input":input,"instructionIndex":0,"differences":difference}));
            }
            previous = after;
        }
    }
    Ok(RecordedRun {
        differential,
        validator_reproduction: reproduction,
        captured_stock_hash: corpus.stock_build_hash.clone(),
        same_stock_binary: replay_stock_hash == corpus.stock_build_hash,
        replay_stock_hash,
    })
}

/// Meter the one top-level invocation from runtime lines, excluding other
/// instructions in its transaction while retaining all of its nested CPI cost.
fn target_units(logs: &[String], program: &str) -> Option<u64> {
    let mut stack = Vec::new();
    let mut samples = Vec::new();
    for line in logs {
        let Some(body) = line.strip_prefix("Program ") else { continue };
        let Some((address, message)) = body.split_once(' ') else { continue };
        if address.parse::<Address>().is_err() { continue }
        if let Some(depth) = message.strip_prefix("invoke [").and_then(|m|m.strip_suffix(']')).and_then(|n|n.parse::<usize>().ok()) {
            if depth != stack.len() + 1 { return None }
            stack.push(address);
        } else if runtime_compute_line(line) {
            if stack.len() == 1 && stack[0] == program && address == program {
                samples.push(message.split(' ').nth(1)?.parse().ok()?);
            }
        } else if message == "success" || message.starts_with("failed: ") {
            if stack.pop() != Some(address) { return None }
        }
    }
    (stack.is_empty() && samples.len() == 1).then(|| samples[0])
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordedProvenance {
    pub generated_corpus_hash: String,
    pub recorded_corpus_hash: String,
    pub captured_stock_hash: String,
    pub replay_stock_hash: String,
    pub same_stock_binary: bool,
    pub validator_reproduction: Reproduction,
}

/// Append every recorded comparison and rebuild instruction medians from the
/// successful raw samples. Transactions without unique target metering remain
/// proof inputs and never become estimated instruction measurements.
pub fn merge_recorded(report: &mut RunReport, recorded: RecordedRun) -> Result<RecordedProvenance> {
    let RecordedRun {
        differential,
        validator_reproduction,
        captured_stock_hash,
        replay_stock_hash,
        same_stock_binary,
    } = recorded;
    let provenance = RecordedProvenance {
        generated_corpus_hash: report.corpus_hash.clone(),
        recorded_corpus_hash: differential.corpus_hash.clone(),
        captured_stock_hash,
        replay_stock_hash,
        same_stock_binary,
        validator_reproduction,
    };
    report.corpus_hash = hex::encode(Sha256::digest(serde_json::to_vec(&[
        &report.corpus_hash,
        &differential.corpus_hash,
    ])?));
    report.counts.tests += differential.counts.tests;
    report.counts.generated += differential.counts.generated;
    report.counts.mainnet += differential.counts.mainnet;
    report.comparisons.inputs += differential.comparisons.inputs;
    report.comparisons.account_bytes += differential.comparisons.account_bytes;
    report.comparisons.lamports += differential.comparisons.lamports;
    report.comparisons.logs += differential.comparisons.logs;
    report.comparisons.return_data += differential.comparisons.return_data;
    report.comparisons.errors += differential.comparisons.errors;
    if report.first_divergence.is_none() {
        report.first_divergence = differential.first_divergence;
    }
    report.outcomes.extend(differential.outcomes);
    for instruction in &mut report.instructions {
        let discriminator = hex::decode(&instruction.discriminator)?;
        let mut gross = Vec::new();
        let mut net = Vec::new();
        for outcome in &report.outcomes {
            if outcome["success"] != true {
                continue;
            }
            let Some(data) = outcome["instructionDataBase64"].as_str() else {
                continue;
            };
            if !BASE64.decode(data)?.starts_with(&discriminator) {
                continue;
            }
            if let (Some(g), Some(n)) = (
                crate::json_u64::integer(&outcome["instructionGross"]),
                crate::json_u64::integer(&outcome["instructionNet"]),
            ) {
                gross.push(g);
                net.push(n);
            }
        }
        instruction.inputs = gross.len() as u64;
        instruction.gross = median(gross);
        instruction.net = median(net);
        instruction.tare = instruction
            .gross
            .zip(instruction.net)
            .map(|(g, n)| g as i64 - n as i64);
    }
    Ok(provenance)
}
