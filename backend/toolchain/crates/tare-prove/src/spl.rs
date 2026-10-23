//! SPL account roles and deterministic client-created bootstrap state.
use crate::*;
use solana_program_pack::Pack;
use spl_token_interface::state::{Account as TokenAccount, AccountState as TokenState, Mint};
const TOKEN: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const TOKEN_2022: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
const ASSOCIATED: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
impl Generator<'_> {
    pub(super) fn resolve_spl_programs(&mut self, ix: &Value) -> Result<()> {
        let instruction = required_str(ix, "name")?;
        if self.hints.spl_operations.iter().any(|(name, operation)| {
            normalize(name) == normalize(instruction) && operation == "set_authority"
        }) {
            let authority = self
                .hints
                .instructions
                .values()
                .flat_map(|fields| fields.values())
                .find(|hint| hint.init && hint.spl.as_ref().is_some_and(|spl| spl.kind == "mint"))
                .and_then(|hint| hint.spl.as_ref().and_then(|spl| spl.authority.clone()))
                .unwrap_or_else(|| "authority".into());
            let authority = self.identity(&authority);
            let mint = self.identity("mint");
            for account in flat_accounts(ix)? {
                let name = required_str(&account, "name")?;
                match normalize(name).as_str() {
                    "currentauthority" => {
                        self.identities.insert(normalize(name), authority);
                    }
                    "accountormint" => {
                        self.identities.insert(normalize(name), mint);
                    }
                    _ => {}
                }
            }
        }
        // Explicit token::token_program and associated_token::token_program
        // constraints may name an unchecked program account rather than a
        // Program<Token> wrapper. Resolve that supplied client account too.
        let declared=flat_accounts(ix)?;
        for account in &declared {
            let hint=self.hint(instruction,required_str(account,"name")?);
            if let Some(program_name)=hint.spl.and_then(|spl|spl.token_program) {
                if let Some(program)=declared.iter().find(|a|a["name"].as_str().is_some_and(|name|normalize(name)==normalize(&program_name))) {
                    let declared_name=required_str(program,"name")?;
                    if program.get("address").is_none() && self.hint(instruction,declared_name).spl.is_none(){self.bind_account(ix,declared_name,TOKEN.parse()?);}
                }
            }
        }
        for a in flat_accounts(ix)? {
            let name = required_str(&a, "name")?;
            if let Some(hint) = self.hint(instruction, name).spl {
                let id = match hint.kind.as_str() {
                    "program-classic" => Some(TOKEN),
                    "program-2022" => Some(TOKEN_2022),
                    "program-interface" => Some(if self.round % 2 == 0 {
                        TOKEN
                    } else {
                        TOKEN_2022
                    }),
                    "program-associated" => Some(ASSOCIATED),
                    _ => None,
                };
                if let Some(id) = id {
                    self.bind_account(ix,name,id.parse()?);
                }
            }
        }
        Ok(())
    }
    fn bound_identity(&mut self,instruction:&str,name:&str)->Address {
        self.account_bindings.get(&(normalize(instruction),normalize(name))).copied().unwrap_or_else(||self.identity(name))
    }
    fn fixture_token_program(&self,instruction:&str,spl:&SplHint)->Result<Address>{
        if let Some(name)=&spl.token_program {
            if let Some(address)=self.account_bindings.get(&(normalize(instruction),normalize(name))){return Ok(*address)}
        }
        let implicit=self.account_bindings.get(&(normalize(instruction),normalize("token_program"))).copied();
        Ok(implicit.filter(|address|address.to_string()==TOKEN||address.to_string()==TOKEN_2022).unwrap_or(TOKEN.parse()?))
    }
    pub(super) fn account_address(
        &mut self,
        ix: &str,
        name: &str,
        hint: &AccountHint,
    ) -> Result<Address> {
        if let Some(spl) = &hint.spl {
            if spl.associated {
                let authority = self.bound_identity(ix,spl.authority.as_deref().unwrap_or("authority"));
                let mint = self.bound_identity(ix,spl.mint.as_deref().unwrap_or("mint"));
                let token = self.fixture_token_program(ix,spl)?;
                return Ok(Address::find_program_address(
                    &[authority.as_ref(), token.as_ref(), mint.as_ref()],
                    &ASSOCIATED.parse()?,
                )
                .0);
            }
        }
        Ok(self.account_bindings.get(&(normalize(ix),normalize(name))).copied().unwrap_or_else(||self.identity(name)))
    }
    pub(super) fn account_fixture(&mut self, ty: &str, hint: &AccountHint, instruction:&str, account_name:&str) -> Result<Account> {
        if let Some(spl) = &hint.spl {
            if spl.kind == "mint" || spl.kind == "token" {
                let owner=self.fixture_token_program(instruction,spl)?;
                let authority = self.bound_identity(instruction,spl.authority.as_deref().unwrap_or("authority"));
                let data = if spl.kind == "mint" {
                    let mut mint = Mint::default();
                    mint.mint_authority = Some(authority.to_bytes().into()).into();
                    mint.supply = 10_000_000_000;
                    mint.decimals = spl.decimals.unwrap_or(9);
                    if let Some((name,scenario))=self.scenario.clone() {
                        if let Some(argument)=scenario.mint_decimals.get(account_name) {
                            let ix=self.idl["instructions"].as_array().unwrap().iter().find(|ix|ix["name"].as_str()==Some(name.as_str())).cloned();
                            if let Some(ix)=ix {let args=array(&ix,"args")?;let index=self.hints.instruction_args.get(&name).and_then(|aliases|aliases.get(argument)).copied();if let Some(arg)=args.iter().find(|arg|arg["name"].as_str()==Some(argument)).or_else(||index.and_then(|i|args.get(i))){mint.decimals=self.encode(&arg["type"],required_str(arg,"name")?)?[0];}}
                        }
                        if let Some(authority_name)=scenario.mint_freeze_authorities.get(account_name).cloned(){mint.freeze_authority=Some(self.bound_identity(instruction,&authority_name).to_bytes().into()).into();}
                    }
                    mint.is_initialized = true;
                    let mut data = vec![0; Mint::LEN];
                    Mint::pack(mint, &mut data)?;
                    data
                } else {
                    let mut account = TokenAccount::default();
                    account.mint = self
                        .bound_identity(instruction,spl.mint.as_deref().unwrap_or("mint"))
                        .to_bytes()
                        .into();
                    account.owner = authority.to_bytes().into();
                    account.amount = 1_000_000_000;
                    account.state = TokenState::Initialized;
                    let mut data = vec![0; TokenAccount::LEN];
                    TokenAccount::pack(account, &mut data)?;
                    data
                };
                return Ok(Account {
                    lamports: program_rent(data.len()),
                    data,
                    owner,
                    executable: false,
                    rent_epoch: 0,
                });
            }
        }
        let discriminator=account_discriminator(self.idl,ty)?;
        let fields=self.encode_defined(ty)?;
        let mut data=if hint.zero {vec![0;discriminator.len()+fields.len()]}else{discriminator};
        if !hint.zero {data.extend(fields);}
        let owner=match(&hint.owner_program,&hint.owner_crate){(Some(owner),_)=>owner.parse()?,(None,Some(name))=>bail!("generated corpus needs the owner program address for imported account {ty} from {name}"),_=>self.program};
        Ok(Account {
            lamports: program_rent(data.len()),
            data,
            owner,
            executable: false,
            rent_epoch: 0,
        })
    }
}


impl Generator<'_> {
    /// Replace client-creatable SPL snapshots with ordinary transactions. Program
    /// PDAs that need an unavailable program-owned mint authority remain explicit
    /// snapshots in starting_accounts; no setup failure is filtered from proof.
    pub(super) fn materialize_spl_fixtures(&mut self) -> Result<()> {
        use spl_token_interface::instruction::{self as token, AuthorityType};
        fn input(mut ix:solana_instruction::Instruction,name:&str,program:Address)->InputInstruction {
            ix.program_id=program.to_bytes().into();
            InputInstruction{program:ix.program_id.to_string(),name:name.into(),accounts:ix.accounts.into_iter().map(|a|InputAccount{address:a.pubkey.to_string(),signer:a.is_signer,writable:a.is_writable}).collect(),data_base64:BASE64.encode(ix.data)}
        }
        fn create(payer:Address,address:Address,owner:Address,len:usize)->InputInstruction {
            let mut data=0u32.to_le_bytes().to_vec();data.extend(program_rent(len).to_le_bytes());data.extend((len as u64).to_le_bytes());data.extend(owner.to_bytes());
            InputInstruction{program:Address::default().to_string(),name:"fixture_create_account".into(),accounts:vec![InputAccount{address:payer.to_string(),signer:true,writable:true},InputAccount{address:address.to_string(),signer:true,writable:true}],data_base64:BASE64.encode(data)}
        }
        let payer=self.transaction_payer;
        let signable:BTreeSet<Address>=self.seeds.values().map(|seed|Keypair::new_from_array(*seed).pubkey()).collect();
        let mut mints=BTreeMap::new();let mut tokens=BTreeMap::new();
        for (address,state) in &self.accounts {
            if state.owner!=TOKEN&&state.owner!=TOKEN_2022 {continue}
            let address:Address=address.parse()?;let data=BASE64.decode(&state.data_base64)?;
            if data.len()==Mint::LEN {if let Ok(mint)=Mint::unpack(&data){mints.insert(address,(state.owner.parse::<Address>()?,mint));}}
            if data.len()==TokenAccount::LEN {if let Ok(token)=TokenAccount::unpack(&data){tokens.insert(address,(state.owner.parse::<Address>()?,token));}}
        }
        let mut setup=Vec::new();let mut delayed:BTreeMap<usize,Vec<Case>>=BTreeMap::new();
        let mut initialized_mints=BTreeMap::new();
        for (index,case) in self.cases.iter().enumerate().filter(|(_,c)|c.commit){for ix in &case.instructions{
            if ix.program!=self.program.to_string(){continue}
            let Some(definition)=self.idl["instructions"].as_array().and_then(|xs|xs.iter().find(|d|d["name"].as_str()==Some(ix.name.as_str())))else{continue};
            let declared=flat_accounts(definition)?;
            for (field,account) in declared.iter().zip(&ix.accounts){
                let hint=self.hint(&ix.name,required_str(field,"name")?);
                if hint.init&&hint.spl.as_ref().is_some_and(|s|s.kind=="mint"){
                    let authority=hint.spl.and_then(|s|s.authority).and_then(|name|declared.iter().position(|a|a["name"].as_str()==Some(name.as_str()))).map(|i|ix.accounts[i].address.parse::<Address>()).transpose()?;
                    initialized_mints.entry(account.address.parse::<Address>()?).or_insert((index+1,authority));
                }
            }
        }}
        // An unchecked transfer need not pass its mint as an account. A
        // signable generated mint still gets a real SPL initialization.
        for (program,account) in tokens.values(){
            let mint=Address::from(account.mint.to_bytes());
            if !mints.contains_key(&mint)&&!initialized_mints.contains_key(&mint)&&signable.contains(&mint){
                let mut value=Mint::default();value.mint_authority=Some(payer.to_bytes().into()).into();value.decimals=9;value.is_initialized=true;
                mints.insert(mint,(*program,value));
            }
        }
        let mut reserve=BTreeMap::new();let mut removed=BTreeSet::new();
        for (address,(program,mint)) in &mints {
            if !signable.contains(address){continue}
            let pool=self.identity(&format!("__spl_reserve_{address}"));
            let desired=tokens.values().filter(|(_,t)|t.mint.to_bytes()==address.to_bytes()).map(|(_,t)|t.amount).sum::<u64>().max(mint.supply);
            let mut ixs=vec![create(payer,*address,*program,Mint::LEN),input(token::initialize_mint2(&TOKEN.parse()?,&address.to_bytes().into(),&payer.to_bytes().into(),Option::<_>::from(mint.freeze_authority).as_ref(),mint.decimals)?,"fixture_initialize_mint",*program),create(payer,pool,*program,TokenAccount::LEN),input(token::initialize_account3(&TOKEN.parse()?,&pool.to_bytes().into(),&address.to_bytes().into(),&payer.to_bytes().into())?,"fixture_initialize_reserve",*program),input(token::mint_to(&TOKEN.parse()?,&address.to_bytes().into(),&pool.to_bytes().into(),&payer.to_bytes().into(),&[],desired)?,"fixture_mint_reserve",*program)];
            let authority=Option::<_>::from(mint.mint_authority);
            ixs.push(input(token::set_authority(&TOKEN.parse()?,&address.to_bytes().into(),authority.as_ref(),AuthorityType::MintTokens,&payer.to_bytes().into(),&[])?,"fixture_set_mint_authority",*program));
            setup.push(Case{label:format!("fixture:mint:{address}"),kind:"generated".into(),instructions:ixs,commit:true,mutations:vec![]});
            reserve.insert(*address,pool);removed.insert(*address);
        }
        for (address,(program,account)) in &tokens {
            let mint=Address::from(account.mint.to_bytes());let owner=Address::from(account.owner.to_bytes());
            let associated=Address::find_program_address(&[owner.as_ref(),program.as_ref(),mint.as_ref()],&ASSOCIATED.parse()?).0==*address;
            if !signable.contains(address)&&!associated{continue}
            let authority=mints.get(&mint).and_then(|(_,m)|if m.mint_authority.is_some(){Some(Address::from(m.mint_authority.unwrap().to_bytes()))}else{None});
            let initialized=initialized_mints.get(&mint).copied();
            if !reserve.contains_key(&mint)&&!mints.contains_key(&mint)&&initialized.is_none(){continue}
            let authority=authority.or_else(||initialized.and_then(|(_,authority)|authority));
            let can_mint=authority.is_some_and(|a|signable.contains(&a));
            let mut ixs=if associated {
                vec![InputInstruction{program:ASSOCIATED.into(),name:"fixture_create_associated_token_account".into(),accounts:vec![InputAccount{address:payer.to_string(),signer:true,writable:true},InputAccount{address:address.to_string(),signer:false,writable:true},InputAccount{address:owner.to_string(),signer:false,writable:false},InputAccount{address:mint.to_string(),signer:false,writable:false},InputAccount{address:Address::default().to_string(),signer:false,writable:false},InputAccount{address:program.to_string(),signer:false,writable:false}],data_base64:BASE64.encode([1u8])}]
            }else{vec![create(payer,*address,*program,TokenAccount::LEN),input(token::initialize_account3(&TOKEN.parse()?,&address.to_bytes().into(),&mint.to_bytes().into(),&owner.to_bytes().into())?,"fixture_initialize_token_account",*program)]};
            if let Some(pool)=reserve.get(&mint){ixs.push(input(token::transfer(&TOKEN.parse()?,&pool.to_bytes().into(),&address.to_bytes().into(),&payer.to_bytes().into(),&[],account.amount)?,"fixture_fund_token_account",*program));}
            else if can_mint {ixs.push(input(token::mint_to(&TOKEN.parse()?,&mint.to_bytes().into(),&address.to_bytes().into(),&authority.unwrap().to_bytes().into(),&[],account.amount)?,"fixture_mint_token_account",*program));}
            let case=Case{label:format!("fixture:token:{address}"),kind:"generated".into(),instructions:ixs,commit:true,mutations:vec![]};
            if let Some((index,_))=initialized{delayed.entry(index).or_default().push(case);}else{setup.push(case);}
            removed.insert(*address);
        }
        for (address,_) in &mints {
            self.fixture_provenance.push(FixtureProvenance{address:address.to_string(),account_type:"spl_mint".into(),origin:if removed.contains(address){"spl_transactions"}else{"typed_snapshot"}.into(),reason:if removed.contains(address){"System create, SPL initialize/mint, and mint-authority handoff are committed setup transactions."}else{"The mint address has no generated transaction keypair; this independent state requires a program-created mint snapshot."}.into()});
        }
        for (address,(program,account)) in &tokens {
            let mint=Address::from(account.mint.to_bytes());let owner=Address::from(account.owner.to_bytes());
            let associated=Address::find_program_address(&[owner.as_ref(),program.as_ref(),mint.as_ref()],&ASSOCIATED.parse()?).0==*address;
            let reason=if removed.contains(address){if associated{"Associated Token Program creation is a committed setup transaction."}else{"System create and SPL account initialization are committed setup transactions."}}else if !signable.contains(address)&&!associated{"This is a program-derived token vault, so a client cannot create it directly through System/SPL. The independent state uses a snapshot; original lifecycle sequences retain the target program's initializer."}else{"No mint starting state or earlier target initializer resolves this token account's mint."};
            self.fixture_provenance.push(FixtureProvenance{address:address.to_string(),account_type:"spl_token_account".into(),origin:if removed.contains(address){"spl_transactions"}else{"typed_snapshot"}.into(),reason:reason.into()});
        }
        for address in removed {self.accounts.remove(&address.to_string());}
        for (index,case) in std::mem::take(&mut self.cases).into_iter().enumerate(){
            if let Some(mut cases)=delayed.remove(&index){setup.append(&mut cases);}setup.push(case);
        }
        for (_,mut cases) in delayed {setup.append(&mut cases);}
        self.cases=setup;
        Ok(())
    }
}
