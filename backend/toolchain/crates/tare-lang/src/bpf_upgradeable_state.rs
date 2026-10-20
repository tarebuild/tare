//! Typed views of the upgradeable loader's account states.
use crate::AccountInfo;
use crate::{AccountDeserialize,AccountSerialize,ErrorCode,Owner,Pubkey,Result};
use crate::solana_program::{bpf_loader_upgradeable::{self,UpgradeableLoaderState},program_error::ProgramError};
#[derive(Clone)]
pub struct ProgramData {pub slot:u64,pub upgrade_authority_address:Option<Pubkey>}
impl AccountDeserialize for ProgramData {
    fn try_deserialize(buf:&mut &[u8])->Result<Self>{Self::try_deserialize_unchecked(buf)}
    fn try_deserialize_unchecked(buf:&mut &[u8])->Result<Self>{
        match UpgradeableLoaderState::try_deserialize_unchecked(buf)? {
            UpgradeableLoaderState::ProgramData{slot,upgrade_authority_address}=>Ok(Self{slot,upgrade_authority_address}),
            _=>Err(ErrorCode::AccountNotProgramData.into()),
        }
    }
}
impl AccountSerialize for ProgramData {}
impl Owner for ProgramData {fn owner()->Pubkey{bpf_loader_upgradeable::ID}}
impl AccountDeserialize for UpgradeableLoaderState {
    fn try_deserialize(buf:&mut &[u8])->Result<Self>{Self::try_deserialize_unchecked(buf)}
    fn try_deserialize_unchecked(buf:&mut &[u8])->Result<Self>{bincode::deserialize(buf).map_err(|_|ProgramError::InvalidAccountData.into())}
}
impl AccountSerialize for UpgradeableLoaderState {}
impl Owner for UpgradeableLoaderState {fn owner()->Pubkey{bpf_loader_upgradeable::ID}}
pub(crate) fn programdata_address(info:crate::AccountView<'_>)->Result<Option<Pubkey>> {
    if info.owner_ref()!=&bpf_loader_upgradeable::ID {return Ok(None);}
    let data=info.try_borrow_data()?;
    match UpgradeableLoaderState::try_deserialize_unchecked(&mut &data[..])? {
        UpgradeableLoaderState::Program{programdata_address}=>Ok(Some(programdata_address)),
        _=>unreachable!(),
    }
}
