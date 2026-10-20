use crate::AccountInfo;
use crate::{AccountView, ErrorCode, FromAccountInfo, Key, Pubkey, Result, ToAccountInfo};
use std::ops::{Deref, DerefMut};
pub struct Sysvar<'info, T: solana_program::sysvar::Sysvar> {
    info: AccountView<'info>,
    account: T,
}
impl<'info, T: solana_program::sysvar::Sysvar> Sysvar<'info, T> {
    pub fn from_account_info(info: &AccountView<'info>) -> Result<Self> {
        Self::try_from_info(*info)
    }
    pub fn __tare_info(&self) -> AccountView<'info> {
        self.info
    }
}
impl<'info, T: solana_program::sysvar::Sysvar> FromAccountInfo<'info> for Sysvar<'info, T> {
    #[inline(always)]
    fn try_from_info(info: AccountView<'info>) -> Result<Self> {
        if !T::check_id(info.key_ref()) {
            return Err(ErrorCode::AccountSysvarMismatch.into());
        }
        let account = bincode::deserialize(&info.data_handle().borrow())
            .map_err(|_| ErrorCode::AccountSysvarMismatch)?;
        Ok(Self { info, account })
    }
}
impl<T: solana_program::sysvar::Sysvar> Clone for Sysvar<'_, T> {
    fn clone(&self) -> Self {
        Self::try_from_info(self.info).unwrap()
    }
}
impl<T: solana_program::sysvar::Sysvar> Deref for Sysvar<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.account
    }
}
impl<T: solana_program::sysvar::Sysvar> DerefMut for Sysvar<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.account
    }
}
impl<T: solana_program::sysvar::Sysvar> Key for Sysvar<'_, T> {
    fn key(&self) -> Pubkey {
        *self.info.key_ref()
    }
}
impl<'info, T: solana_program::sysvar::Sysvar> ToAccountInfo<'info> for Sysvar<'info, T> {
    fn to_account_info(&self) -> AccountInfo<'info> {
        self.info.sdk().clone()
    }
    fn __tare_meta(&self, _signer: Option<bool>) -> solana_program::instruction::AccountMeta {
        solana_program::instruction::AccountMeta::new_readonly(*self.info.key_ref(), false)
    }
}
impl<'info, T: solana_program::sysvar::Sysvar> AsRef<AccountInfo<'info>> for Sysvar<'info, T> {
    fn as_ref(&self) -> &AccountInfo<'info> {
        self.info.sdk()
    }
}

impl<T:solana_program::sysvar::Sysvar+std::fmt::Debug> std::fmt::Debug for Sysvar<'_,T> {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {f.debug_struct("Sysvar").field("info",&self.info).field("account",&self.account).finish()}
}
