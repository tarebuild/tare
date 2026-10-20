//! Allocation-free C ABI invocation used by generated account initialization.
use crate::AccountInfo;
use crate::{AccountView, Pubkey, Result};
use solana_program::program_error::ProgramError;
#[repr(C)]
struct Meta {
    key: *const Pubkey,
    writable: bool,
    signer: bool,
}
#[repr(C)]
struct Instruction {
    program: *const Pubkey,
    accounts: *const Meta,
    accounts_len: u64,
    data: *const u8,
    data_len: u64,
}
#[repr(C)]
struct Info {
    key: *const Pubkey,
    lamports: *mut u64,
    data_len: u64,
    data: *mut u8,
    owner: *const Pubkey,
    rent_epoch: u64,
    flags: u64,
}
#[repr(C)]
struct Seed {
    address: *const u8,
    len: u64,
}
#[repr(C)]
struct SignerSeeds {
    address: *const Seed,
    len: u64,
}
#[cfg(target_os = "solana")]
extern "C" {
    fn sol_invoke_signed_c(
        instruction: *const u8,
        infos: *const u8,
        infos_len: u64,
        seeds: *const u8,
        seeds_len: u64,
    ) -> u64;
}
#[inline(always)]
pub(crate) fn invoke<const N: usize>(
    program: AccountView,
    accounts: [AccountView; N],
    signer: [bool; N],
    writable: [bool; N],
    data: &[u8],
    seeds: &[&[u8]],
) -> Result<()> {
    let metas = std::array::from_fn::<_, N, _>(|i| Meta {
        key: accounts[i].key_ref(),
        writable: writable[i],
        signer: signer[i],
    });
    invoke_inner(program.key_ref(), accounts, metas, data, seeds)
}
#[inline(always)]
pub(crate) fn invoke_with_keys<const N: usize>(
    program: AccountView,
    accounts: [AccountView; N],
    keys: [Pubkey; N],
    signer: [bool; N],
    writable: [bool; N],
    data: &[u8],
    seeds: &[&[u8]],
) -> Result<()> {
    let metas = std::array::from_fn::<_, N, _>(|i| Meta {
        key: &keys[i],
        writable: writable[i],
        signer: signer[i],
    });
    invoke_inner(program.key_ref(), accounts, metas, data, seeds)
}
// IDL management is off the ordinary instruction path. Share its marshalling
// across operations rather than duplicating it in every lifecycle handler.
#[inline(never)]
pub(crate) fn invoke_idl<const N: usize>(
    program: AccountView, accounts: [AccountView; N], signer: [bool; N],
    writable: [bool; N], data: &[u8], seeds: &[&[u8]],
) -> Result<()> {
    invoke(program, accounts, signer, writable, data, seeds)
}
#[inline(never)]
pub(crate) fn invoke_idl_with_keys<const N: usize>(
    program: AccountView, accounts: [AccountView; N], keys: [Pubkey; N],
    signer: [bool; N], writable: [bool; N], data: &[u8], seeds: &[&[u8]],
) -> Result<()> {
    invoke_with_keys(program, accounts, keys, signer, writable, data, seeds)
}
#[inline(always)]
fn invoke_inner<const N: usize>(
    program: &Pubkey,
    accounts: [AccountView; N],
    metas: [Meta; N],
    data: &[u8],
    seeds: &[&[u8]],
) -> Result<()> {
    #[cfg(tare_sdk)]
    if accounts.iter().any(|info|info.sdk_exposed()) {
        return invoke_sdk_fixed(program,accounts,metas,data,seeds);
    }
    #[cfg(not(tare_sdk))]
    if accounts.iter().any(|info|info.sdk_exposed()) {
        let infos=accounts.iter().map(|info|info.sdk().clone()).collect::<Vec<_>>();
        let metas=metas.iter().map(|meta|solana_program::instruction::AccountMeta{pubkey:unsafe{*meta.key},is_signer:meta.signer,is_writable:meta.writable}).collect::<Vec<_>>();
        let signer_groups=if seeds.is_empty(){Vec::new()}else{vec![seeds]};
        return invoke_dynamic_with_id(program,&infos,&metas,data,&signer_groups);
    }
    #[allow(unused_mut, unused_variables)]
    let mut infos: [std::mem::MaybeUninit<Info>;N]=[const {std::mem::MaybeUninit::uninit()};N];
    for (i,slot) in infos.iter_mut().enumerate() {
        let raw=accounts[i].raw();
        unsafe {slot.write(Info{key:&(*raw).key,lamports:&mut (*raw).lamports,data_len:(*raw).data_len,data:raw.add(1).cast(),owner:&(*raw).owner,rent_epoch:0,flags:(accounts[i].is_signer() as u64)|((accounts[i].is_writable() as u64)<<8)|((accounts[i].executable() as u64)<<16)});}
    }
    // Only the prefix described by len crosses the syscall boundary. Framework
    // seeds are compiler-bounded at 16, so unused descriptors need no stores.
    let mut c_seeds: [std::mem::MaybeUninit<Seed>; 16] =
        [const { std::mem::MaybeUninit::uninit() }; 16];
    for (dst, src) in c_seeds.iter_mut().zip(seeds) {
        dst.write(Seed {
            address: src.as_ptr(),
            len: src.len() as u64,
        });
    }
    let signer_seeds = SignerSeeds {
        address: c_seeds.as_ptr().cast(),
        len: seeds.len() as u64,
    };
    let instruction = Instruction {
        program,
        accounts: metas.as_ptr(),
        accounts_len: N as u64,
        data: data.as_ptr(),
        data_len: data.len() as u64,
    };
    for (index,meta) in metas.iter().enumerate() {
        let key=unsafe {&*meta.key};
        let info=if key==accounts[index].key_ref() {Some(&accounts[index])} else {accounts.iter().find(|info|info.key_ref()==key)};
        if let Some(info)=info {
            info.raw_borrow_check(meta.writable)?;
        }
    }
    #[cfg(target_os = "solana")]
    let status = unsafe {
        sol_invoke_signed_c(
            (&instruction as *const Instruction).cast(),
            infos.as_mut_ptr().cast(),
            N as u64,
            (&signer_seeds as *const SignerSeeds).cast(),
            if seeds.is_empty() { 0 } else { 1 },
        )
    };
    #[cfg(not(target_os = "solana"))]
    let status: u64 = {
        let _ = (&instruction, &signer_seeds);
        return Err(ProgramError::UnsupportedSysvar.into());
    };
    #[allow(unreachable_code)]
    if status != 0 {
        return Err(ProgramError::from(status).into());
    }
    for i in 0..N {
        unsafe {
            (*accounts[i].raw()).data_len = infos[i].assume_init_ref().data_len;
        }
    }
    Ok(())
}
#[cfg(tare_sdk)]
#[inline]
fn invoke_sdk_fixed<const N:usize>(program:&Pubkey,accounts:[AccountView;N],metas:[Meta;N],data:&[u8],seeds:&[&[u8]])->Result<()> {
    if seeds.len()>16 {
        let infos=accounts.iter().map(|info|info.sdk().clone()).collect::<Vec<_>>();
        let metas=metas.iter().map(|meta|solana_program::instruction::AccountMeta{pubkey:unsafe{*meta.key},is_signer:meta.signer,is_writable:meta.writable}).collect::<Vec<_>>();
        return invoke_dynamic_with_id(program,&infos,&metas,data,&[seeds]);
    }
    let sdk=accounts.each_ref().map(|account|account.sdk());
    // Match SDK invoke_signed: test each instruction meta, in order, against
    // the first supplied account with the same key. Extra infos are unchecked.
    for meta in &metas {
        let key=unsafe{&*meta.key};
        if let Some(info)=sdk.iter().find(|info|info.key==key){
            if meta.writable {drop(info.try_borrow_mut_lamports()?);drop(info.try_borrow_mut_data()?);}
            else {drop(info.try_borrow_lamports()?);drop(info.try_borrow_data()?);}
        }
    }
    let mut infos=sdk.map(|info|unsafe{
        let lamports=&*info.lamports.as_ptr();let bytes=&*info.data.as_ptr();
        Info{key:info.key,lamports:(&**lamports as *const u64).cast_mut(),data_len:bytes.len() as u64,data:bytes.as_ptr().cast_mut(),owner:info.owner,rent_epoch:info.rent_epoch,flags:(info.is_signer as u64)|((info.is_writable as u64)<<8)|((info.executable as u64)<<16)}
    });
    let mut c_seeds:[std::mem::MaybeUninit<Seed>;16]=[const{std::mem::MaybeUninit::uninit()};16];
    for(dst,src)in c_seeds.iter_mut().zip(seeds){dst.write(Seed{address:src.as_ptr(),len:src.len() as u64});}
    let signer=SignerSeeds{address:c_seeds.as_ptr().cast(),len:seeds.len() as u64};
    let instruction=Instruction{program,accounts:metas.as_ptr(),accounts_len:N as u64,data:data.as_ptr(),data_len:data.len() as u64};
    #[cfg(target_os="solana")]
    let status=unsafe{sol_invoke_signed_c((&instruction as *const Instruction).cast(),infos.as_mut_ptr().cast(),N as u64,(&signer as *const SignerSeeds).cast(),if seeds.is_empty(){0}else{1})};
    #[cfg(not(target_os="solana"))]
    let status:u64={let _=(&instruction,&signer,&mut infos);return Err(ProgramError::UnsupportedSysvar.into());};
    #[allow(unreachable_code)]
    if status!=0{return Err(ProgramError::from(status).into());}
    for(account,info)in sdk.into_iter().zip(infos){if account.data_len()!=info.data_len as usize{let mut data=account.data.borrow_mut();unsafe{*data=std::slice::from_raw_parts_mut(info.data,info.data_len as usize);}}}
    Ok(())
}
#[cfg_attr(not(tare_sdk), inline(always))]
#[cfg_attr(tare_sdk, inline(never))]
pub fn create_account(
    payer: AccountView,
    target: AccountView,
    system: AccountView,
    owner: &Pubkey,
    space: usize,
    rent: &solana_program::rent::Rent,
    seeds: &[&[u8]],
) -> Result<()> {
    let current = target.lamports();
    if current == 0 {
        #[repr(C, packed)]
        struct CreateAccountData { tag: u32, lamports: u64, space: u64, owner: Pubkey }
        let wire=CreateAccountData {tag:0,lamports:crate::rent::minimum_balance(rent,space),space:space as u64,owner:*owner};
        let data=unsafe {std::slice::from_raw_parts((&wire as *const CreateAccountData).cast::<u8>(),52)};
        invoke(
            system,
            [payer, target],
            [true, true],
            [true, true],
            data,
            seeds,
        )?;
    } else {
        let required = crate::rent::minimum_balance(rent, space)
            .max(1)
            .saturating_sub(current);
        if crate::TARE_ANCHOR_MAJOR >= 1 {
            // SystemInstruction::CreateAccountAllowPrefund is variant 13. Its
            // target comes first, and the payer is a meta only when funding.
            let mut data = [0u8; 52];
            data[..4].copy_from_slice(&13u32.to_le_bytes());
            data[4..12].copy_from_slice(&required.to_le_bytes());
            data[12..20].copy_from_slice(&(space as u64).to_le_bytes());
            data[20..].copy_from_slice(owner.as_ref());
            if required > 0 {
                invoke(system, [target, payer], [true, true], [true, true], &data, seeds)?;
            } else {
                invoke(system, [target], [true], [true], &data, seeds)?;
            }
            return Ok(());
        }
        if required > 0 {
            let mut data = [0u8; 12];
            data[..4].copy_from_slice(&2u32.to_le_bytes());
            data[4..].copy_from_slice(&required.to_le_bytes());
            invoke(
                system,
                [payer, target],
                [true, false],
                [true, true],
                &data,
                &[],
            )?;
        }
        let mut allocate = [0u8; 12];
        allocate[..4].copy_from_slice(&8u32.to_le_bytes());
        allocate[4..].copy_from_slice(&(space as u64).to_le_bytes());
        invoke(system, [target], [true], [true], &allocate, seeds)?;
        let mut assign = [0u8; 36];
        assign[..4].copy_from_slice(&1u32.to_le_bytes());
        assign[4..].copy_from_slice(owner.as_ref());
        invoke(system, [target], [true], [true], &assign, seeds)?;
    }
    Ok(())
}
/// Invoke an instruction emitted by an Accounts derive. The ordinary fixed
/// account case uses stack descriptors; explicit remaining accounts or several
/// PDA signers take the general C ABI path.
pub fn invoke_generated<'info,const N: usize>(
    program: AccountInfo<'info>,
    accounts: [AccountInfo<'info>; N],
    signer: [bool; N],
    writable: [bool; N],
    data: &[u8],
    signer_seeds: &[&[&[u8]]],
    remaining: &[AccountInfo<'info>],
) -> Result<()> {
    #[cfg(not(tare_sdk))]
    {
        // The fixed shape can use stack descriptors. Keep the general path for
        // aliases, remaining accounts, and seed groups that this ABI cannot
        // represent (notably one explicitly empty signer group).
        let fixed_seeds = signer_seeds.is_empty()
            || (signer_seeds.len() == 1 && !signer_seeds[0].is_empty()
                && signer_seeds[0].len() <= 16);
        let unique = accounts.iter().enumerate()
            .all(|(index, account)| accounts[..index].iter()
                .all(|prior| prior.raw() != account.raw()));
        if remaining.is_empty() && fixed_seeds && unique {
            return invoke(program, accounts, signer, writable, data,
                signer_seeds.first().copied().unwrap_or(&[]));
        }
    }
    let mut infos=accounts.to_vec();
    infos.extend_from_slice(remaining);
    let mut metas=accounts.iter().zip(signer).zip(writable).map(|((account,signer),writable)|solana_program::instruction::AccountMeta{pubkey:*account.key,is_signer:signer,is_writable:writable}).collect::<Vec<_>>();
    metas.extend(remaining.iter().map(|info|solana_program::instruction::AccountMeta{pubkey:*info.key,is_signer:info.is_signer,is_writable:info.is_writable}));
    invoke_dynamic_with_id(program.key,&infos,&metas,data,signer_seeds)
}

pub fn invoke_dynamic(
    program: AccountInfo,
    accounts: &[AccountInfo],
    metas: &[solana_program::instruction::AccountMeta],
    data: &[u8],
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    invoke_dynamic_with_id(program.key, accounts, metas, data, signer_seeds)
}

#[cfg(tare_sdk)]
pub fn invoke_dynamic_with_id(
    program: &Pubkey,
    accounts: &[AccountInfo],
    metas: &[solana_program::instruction::AccountMeta],
    data: &[u8],
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    for meta in metas {
        if let Some(info)=accounts.iter().find(|info|info.key==&meta.pubkey) {
            if meta.is_writable {drop(info.try_borrow_mut_lamports()?);drop(info.try_borrow_mut_data()?);}
            else {drop(info.try_borrow_lamports()?);drop(info.try_borrow_data()?);}
        }
    }
    let c_metas=metas.iter().map(|meta|Meta{key:&meta.pubkey,writable:meta.is_writable,signer:meta.is_signer}).collect::<Vec<_>>();
    let mut c_infos=accounts.iter().map(|info| unsafe {
        // invoke_signed checks only metas above. Unused extra account infos may
        // have active RefCell borrows, as in the SDK's unchecked ABI marshalling.
        let lamports=&*info.lamports.as_ptr();let data=&*info.data.as_ptr();
        Info{key:info.key,lamports:(&**lamports as *const u64).cast_mut(),data_len:data.len() as u64,data:data.as_ptr().cast_mut(),owner:info.owner,rent_epoch:info.rent_epoch,flags:(info.is_signer as u64)|((info.is_writable as u64)<<8)|((info.executable as u64)<<16)}
    }).collect::<Vec<_>>();
    let seed_lists=signer_seeds.iter().map(|list|list.iter().map(|seed|Seed{address:seed.as_ptr(),len:seed.len() as u64}).collect::<Vec<_>>()).collect::<Vec<_>>();
    let c_seeds=seed_lists.iter().map(|list|SignerSeeds{address:list.as_ptr(),len:list.len() as u64}).collect::<Vec<_>>();
    let instruction=Instruction{program,accounts:c_metas.as_ptr(),accounts_len:c_metas.len() as u64,data:data.as_ptr(),data_len:data.len() as u64};
    #[cfg(target_os="solana")]
    let status=unsafe {sol_invoke_signed_c((&instruction as *const Instruction).cast(),c_infos.as_mut_ptr().cast(),c_infos.len() as u64,c_seeds.as_ptr().cast(),c_seeds.len() as u64)};
    #[cfg(not(target_os="solana"))]
    let status:u64={let _=(&instruction,&c_seeds);return Err(ProgramError::UnsupportedSysvar.into());};
    #[allow(unreachable_code)]
    if status!=0 {return Err(ProgramError::from(status).into());}
    for (account,info) in accounts.iter().zip(&c_infos) {
        if account.data_len()!=info.data_len as usize {let mut data=account.data.borrow_mut();unsafe{*data=std::slice::from_raw_parts_mut(info.data,info.data_len as usize);}}
    }
    Ok(())
}

pub struct Return<T> {
    phantom: std::marker::PhantomData<T>,
}
impl<T> Return<T> {
    pub fn new() -> Self {
        Self {
            phantom: std::marker::PhantomData,
        }
    }
}
impl<T: crate::AnchorDeserialize> Return<T> {
    pub fn get(&self) -> T {
        let (_, data) = solana_program::program::get_return_data().unwrap();
        T::try_from_slice(&data).unwrap()
    }
}

#[cfg_attr(not(tare_sdk), inline(always))]
pub fn invoke_self(
    program: &Pubkey,
    authority: AccountView,
    data: &[u8],
    seeds: &[&[u8]],
) -> Result<()> {
    invoke_inner(
        program,
        [authority],
        [Meta {
            key: authority.key_ref(),
            writable: false,
            signer: true,
        }],
        data,
        seeds,
    )
}

#[cfg(not(tare_sdk))]
pub fn invoke_dynamic_with_id(
    program: &Pubkey,
    accounts: &[AccountInfo],
    metas: &[solana_program::instruction::AccountMeta],
    data: &[u8],
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    for meta in metas {
        if let Some(account) = accounts.iter().find(|a| *a.key == meta.pubkey) {
            if meta.is_writable {
                drop(account.try_borrow_mut_lamports()?);
                drop(account.try_borrow_mut_data()?);
            } else {
                drop(account.try_borrow_lamports()?);
                drop(account.try_borrow_data()?);
            }
        }
    }
    let c_metas: Vec<Meta> = metas
        .iter()
        .map(|m| Meta {
            key: &m.pubkey,
            writable: m.is_writable,
            signer: m.is_signer,
        })
        .collect();
    let mut infos: Vec<Info> = accounts
        .iter()
        .map(|a| unsafe {
            let raw = a.raw();
            Info {
                key: a.key,
                lamports: &mut (*raw).lamports,
                data_len: (*raw).data_len,
                data: raw.add(1).cast(),
                owner: a.owner,
                rent_epoch: 0,
                flags:(a.is_signer as u64)|((a.is_writable as u64)<<8)|((a.executable as u64)<<16),
            }
        })
        .collect();
    let seed_buffers: Vec<Vec<Seed>> = signer_seeds
        .iter()
        .map(|group| {
            group
                .iter()
                .map(|bytes| Seed {
                    address: bytes.as_ptr(),
                    len: bytes.len() as u64,
                })
                .collect()
        })
        .collect();
    let signers: Vec<SignerSeeds> = seed_buffers
        .iter()
        .map(|group| SignerSeeds {
            address: group.as_ptr(),
            len: group.len() as u64,
        })
        .collect();
    let instruction = Instruction {
        program,
        accounts: c_metas.as_ptr(),
        accounts_len: c_metas.len() as u64,
        data: data.as_ptr(),
        data_len: data.len() as u64,
    };
    #[cfg(target_os = "solana")]
    let status = unsafe {
        sol_invoke_signed_c(
            (&instruction as *const Instruction).cast(),
            infos.as_mut_ptr().cast(),
            infos.len() as u64,
            signers.as_ptr().cast(),
            signers.len() as u64,
        )
    };
    #[cfg(not(target_os = "solana"))]
    let status: u64 = {
        let _ = (&instruction, &signers);
        return Err(ProgramError::UnsupportedSysvar.into());
    };
    #[allow(unreachable_code)]
    if status != 0 {
        return Err(ProgramError::from(status).into());
    }
    // The C runtime updates its first matching descriptor. Rust AccountInfo
    // clones share one length cell; preserve that alias behavior here.
    for (index, account) in accounts.iter().enumerate() {
        if accounts[..index]
            .iter()
            .all(|other| other.raw() != account.raw())
        {
            unsafe {
                (*account.raw()).data_len = infos[index].data_len;
            }
        }
    }
    Ok(())
}
