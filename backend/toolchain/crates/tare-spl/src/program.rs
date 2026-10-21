//! Preserve SPL instruction IDs and borrow checks using the SDK's native ABI.
use anchor_lang::{AccountInfo, Result};
use anchor_lang::solana_program::instruction::Instruction;

#[inline]
pub(crate) fn invoke_signed(
    instruction: &Instruction,
    accounts: &[AccountInfo<'_>],
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    #[cfg(tare_sdk)]
    {anchor_lang::solana_program::program::invoke_signed(instruction, accounts, signer_seeds).map_err(Into::into)}
    #[cfg(not(tare_sdk))]
    {anchor_lang::cpi::invoke_dynamic_with_id(&instruction.program_id,accounts,&instruction.accounts,&instruction.data,signer_seeds)}
}

#[cfg(feature = "token_2022")]
pub(crate) fn invoke(instruction: &Instruction, accounts: &[AccountInfo<'_>]) -> Result<()> {
    invoke_signed(instruction, accounts, &[])
}


// Fixed token CPI descriptors preserve the SDK's meta-ordered borrow checks.
#[cfg(all(tare_sdk, tare_external_entry))]
#[inline]
pub(crate) fn transfer_fixed<'info>(
    accounts: [AccountInfo<'info>; 3],
    signer_seeds: &[&[&[u8]]],
    data: &[u8; 9],
) -> Result<()> {
    use anchor_lang::solana_program::{program_error::ProgramError,pubkey::Pubkey};
    use std::mem::MaybeUninit;
    #[repr(C)] struct Meta { key:*const Pubkey, writable:bool, signer:bool }
    #[repr(C)] struct Instruction { program:*const Pubkey, accounts:*const Meta, accounts_len:u64, data:*const u8, data_len:u64 }
    #[repr(C)] struct Info { key:*const Pubkey, lamports:*mut u64, data_len:u64, data:*mut u8, owner:*const Pubkey, rent_epoch:u64, flags:u64 }
    #[repr(C)] struct Seed { address:*const u8, len:u64 }
    #[repr(C)] struct Seeds { address:*const Seed, len:u64 }
    if signer_seeds.len()>1 || signer_seeds.first().is_some_and(|s|s.len()>16) {
        let ix=spl_token::instruction::transfer(&spl_token::ID,accounts[0].key,accounts[1].key,accounts[2].key,&[],u64::from_le_bytes(data[1..].try_into().unwrap()))?;
        return invoke_signed(&ix,&accounts,signer_seeds);
    }
    let metas=[
        Meta{key:accounts[0].key,writable:true,signer:false},
        Meta{key:accounts[1].key,writable:true,signer:false},
        Meta{key:accounts[2].key,writable:false,signer:true},
    ];
    for meta in &metas {
        if let Some(info)=accounts.iter().find(|info|info.key==unsafe{&*meta.key}) {
            if meta.writable {drop(info.try_borrow_mut_lamports()?);drop(info.try_borrow_mut_data()?);}
            else {drop(info.try_borrow_lamports()?);drop(info.try_borrow_data()?);}
        }
    }
    let mut infos=accounts.each_ref().map(|info|unsafe{
        let lamports=&*info.lamports.as_ptr();let bytes=&*info.data.as_ptr();
        Info{key:info.key,lamports:(&**lamports as *const u64).cast_mut(),data_len:bytes.len() as u64,data:bytes.as_ptr().cast_mut(),owner:info.owner,rent_epoch:info.rent_epoch,flags:(info.is_signer as u64)|((info.is_writable as u64)<<8)|((info.executable as u64)<<16)}
    });
    let mut seeds:[MaybeUninit<Seed>;16]=[const{MaybeUninit::uninit()};16];
    let group=signer_seeds.first().copied().unwrap_or(&[]);
    for (out,seed) in seeds.iter_mut().zip(group) {out.write(Seed{address:seed.as_ptr(),len:seed.len() as u64});}
    let signer=Seeds{address:seeds.as_ptr().cast(),len:group.len() as u64};
    let ix=Instruction{program:&spl_token::ID,accounts:metas.as_ptr(),accounts_len:3,data:data.as_ptr(),data_len:9};
    #[cfg(target_os="solana")]
    let status=unsafe {
        extern "C" {fn sol_invoke_signed_c(ix:*const u8,infos:*const u8,len:u64,seeds:*const u8,seeds_len:u64)->u64;}
        sol_invoke_signed_c((&ix as *const Instruction).cast(),infos.as_mut_ptr().cast(),3,(&signer as *const Seeds).cast(),signer_seeds.len() as u64)
    };
    #[cfg(not(target_os="solana"))]
    let status:u64={let _=(&ix,&signer,&mut infos);return Err(ProgramError::UnsupportedSysvar.into());};
    #[allow(unreachable_code)]
    if status!=0{return Err(ProgramError::from(status).into());}
    for (account,info) in accounts.iter().zip(infos) {
        if account.data_len()!=info.data_len as usize {let mut data=account.data.borrow_mut();unsafe{*data=std::slice::from_raw_parts_mut(info.data,info.data_len as usize);}}
    }
    Ok(())
}
