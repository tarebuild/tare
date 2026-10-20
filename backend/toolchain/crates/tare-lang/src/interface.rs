//! Account wrappers whose owner or program ID is selected from an accepted set.
use crate::AccountInfo;
use crate::{
    Account, AccountDeserialize, AccountView, AccountSerialize, AccountValue, AccountsClose,
    CheckId, CheckOwner, FromAccountInfo, Key, Pubkey, Result, ToAccountInfo,
};
use std::{
    cell::Cell,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

pub struct InterfaceAccount<'info, T> {
    account: Account<'info, T>,
    owner: Pubkey,
}
impl<'info, T: AccountDeserialize + CheckOwner> InterfaceAccount<'info, T> {
    #[inline(always)]
    fn load(info: AccountView<'info>, unchecked: bool) -> Result<Self> {
        if info.owner_ref() == &solana_program::system_program::ID && info.lamports() == 0 {
            return Err(crate::ErrorCode::AccountNotInitialized.into());
        }
        T::check_owner(info.owner_ref())?;
        let value = {
            let data = info.try_borrow_data()?;
            let mut bytes = &*data;
            if unchecked {
                T::try_deserialize_unchecked(&mut bytes)?
            } else {
                T::try_deserialize(&mut bytes)?
            }
        };
        Ok(Self {
            owner: *info.owner_ref(),
            account: Account {
                value: AccountValue::Snapshot(value),
                info,
                #[cfg(not(all(tare_sdk,tare_external_entry)))]
                dirty: Cell::new(unchecked || info.__aliased()),
                #[cfg(not(all(tare_sdk,tare_external_entry)))]
                initialize: unchecked,
            },
        })
    }
    #[inline(always)]
    pub fn try_from_unchecked(info: AccountView<'info>) -> Result<Self> {
        Self::load(info, true)
    }
    #[inline(always)]
    pub fn __try_from_unchecked(info: AccountView<'info>, _direct: bool) -> Result<Self> {
        Self::load(info, true)
    }
    pub fn reload(&mut self) -> Result<()> {
        let value = T::try_deserialize(&mut &*self.account.info.try_borrow_data()?)?;
        self.account.value = AccountValue::Snapshot(value);
        #[cfg(not(all(tare_sdk,tare_external_entry)))]
        self.account.dirty.set(true);
        Ok(())
    }
}
impl<'info, T: AccountDeserialize + CheckOwner> FromAccountInfo<'info>
    for InterfaceAccount<'info, T>
{
    #[inline(always)]
    fn try_from_info(info: AccountView<'info>) -> Result<Self> {
        Self::load(info, false)
    }
}
impl<T> Deref for InterfaceAccount<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.account
    }
}
impl<T> DerefMut for InterfaceAccount<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.account
    }
}
impl<T> Key for InterfaceAccount<'_, T> {
    fn key(&self) -> Pubkey {
        self.account.key()
    }
}
impl<'info, T> ToAccountInfo<'info> for InterfaceAccount<'info, T> {
    #[inline(always)]
    fn __tare_info(&self) -> AccountView<'info> { self.account.__tare_info() }
    fn to_account_info(&self) -> AccountInfo<'info> {
        self.account.to_account_info()
    }
}
impl<'info, T> InterfaceAccount<'info, T> {
    pub fn __tare_info(&self) -> AccountView<'info> {
        self.account.__tare_info()
    }
    pub fn set_inner(&mut self, value: T) {
        self.account.value = AccountValue::Snapshot(value);
        #[cfg(not(all(tare_sdk,tare_external_entry)))]
        self.account.dirty.set(true);
    }
    pub fn into_inner(self) -> T {
        match self.account.value {
            AccountValue::Snapshot(value) => value,
            #[cfg(not(tare_sdk))]
            AccountValue::Direct(ptr) => unsafe { ptr.read() },
        }
    }
}
impl<T: AccountSerialize> InterfaceAccount<'_, T> {
    pub fn exit(&self, program_id: &Pubkey) -> Result<()> {
        let info = self.account.info;
        if crate::TARE_ANCHOR_MAJOR >= 1 && self.owner == *program_id
            && info.owner_ref() != program_id
            && !(info.owner_ref() == &solana_program::system_program::ID && info.data_is_empty())
        {
            return self.account.__exit_unowned(program_id);
        }
        if (self.account.__dirty() || info.sdk_exposed())
            && self.owner == *program_id
            && !(info.owner_ref() == &solana_program::system_program::ID && info.data_is_empty())
        {
            let mut data = info.try_borrow_mut_data()?;
            match &self.account.value {
                AccountValue::Snapshot(value) => value.try_serialize_slice(&mut data)?,
                #[cfg(not(tare_sdk))]
            AccountValue::Direct(ptr) => unsafe { (&**ptr).try_serialize_slice(&mut data)? },
            }
        }
        Ok(())
    }
}
impl<'info, T> AccountsClose<'info> for InterfaceAccount<'info, T> {
    fn close(&self, destination: AccountInfo<'info>) -> Result<()> {
        self.account.close(destination)
    }
}

pub struct Interface<'info, T> {
    info: AccountView<'info>,
    marker: PhantomData<T>,
}
impl<'info, T: CheckId> FromAccountInfo<'info> for Interface<'info, T> {
    #[inline(always)]
    fn try_from_info(info: AccountView<'info>) -> Result<Self> {
        T::check_id(info.key_ref())?;
        if !info.executable() {
            return Err(crate::ErrorCode::InvalidProgramExecutable.into());
        }
        Ok(Self {
            info,
            marker: PhantomData,
        })
    }
}
impl<'info, T> Deref for Interface<'info, T> {
    type Target = AccountInfo<'info>;
    fn deref(&self) -> &Self::Target {
        self.info.sdk()
    }
}
impl<T> Key for Interface<'_, T> {
    fn key(&self) -> Pubkey {
        *self.info.key_ref()
    }
}
impl<'info, T> ToAccountInfo<'info> for Interface<'info, T> {
    fn to_account_info(&self) -> AccountInfo<'info> {
        self.info.sdk().clone()
    }
}
impl<'info, T> Interface<'info, T> {
    pub fn __tare_info(&self) -> AccountView<'info> {
        self.info
    }
}

impl<T:AccountSerialize+AccountDeserialize+Clone+std::fmt::Debug> std::fmt::Debug for InterfaceAccount<'_,T> {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {f.debug_struct("InterfaceAccount").field("account",&**self).field("info",&self.account.info).finish()}
}
