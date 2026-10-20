use crate::AccountInfo;
use crate::{
    AccountView, AccountsClose, Discriminator, Error, ErrorCode, FromAccountInfo, Key, Owner,
    Pubkey, Result, ToAccountInfo, ZeroCopy,
};
use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};
#[derive(Clone)]
pub struct AccountLoader<'info, T: ZeroCopy + Owner> {
    info: AccountView<'info>,
    marker: PhantomData<T>,
}
impl<'info, T: ZeroCopy + Owner> AccountLoader<'info, T> {
    pub fn try_from(info: &AccountInfo<'info>) -> Result<Self> {
        Self::try_from_info(AccountView::from_sdk(info))
    }
    pub fn try_from_unchecked(_program_id: &Pubkey, info: &AccountInfo<'info>) -> Result<Self> {
        Self::__try_from_unchecked(AccountView::from_sdk(info), false)
    }
    pub fn __try_from_unchecked(info: AccountView<'info>, _direct: bool) -> Result<Self> {
        if info.owner_ref() != &T::owner() {
            return Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
                .with_pubkeys((*info.owner_ref(), T::owner())));
        }
        Ok(Self {
            info,
            marker: PhantomData,
        })
    }
    pub fn load(&self) -> Result<std::cell::Ref<'_, T>> {
        let data = self.info.sdk().try_borrow_data()?;
        check::<T>(&data)?;
        self.check_size(&data)?;
        Ok(std::cell::Ref::map(data,|data|bytemuck::from_bytes::<T>(&data[T::DISCRIMINATOR.len()..T::DISCRIMINATOR.len()+std::mem::size_of::<T>()])))
    }
    pub fn load_mut(&self) -> Result<std::cell::RefMut<'_, T>> {
        if !self.info.is_writable() {
            return Err(ErrorCode::AccountNotMutable.into());
        }
        let mut data = self.info.sdk().try_borrow_mut_data()?;
        check::<T>(&data)?;
        self.check_size(&data)?;
        Ok(std::cell::RefMut::map(data,|data|bytemuck::from_bytes_mut::<T>(&mut data[T::DISCRIMINATOR.len()..T::DISCRIMINATOR.len()+std::mem::size_of::<T>()])))
    }
    pub fn load_init(&self) -> Result<std::cell::RefMut<'_, T>> {
        if !self.info.is_writable() {
            return Err(ErrorCode::AccountNotMutable.into());
        }
        let mut data = self.info.sdk().try_borrow_mut_data()?;
        self.check_size(&data)?;
        if data[..T::DISCRIMINATOR.len()].iter().any(|b| *b != 0) {
            return Err(ErrorCode::AccountDiscriminatorAlreadySet.into());
        }
        Ok(std::cell::RefMut::map(data,|data|bytemuck::from_bytes_mut::<T>(&mut data[T::DISCRIMINATOR.len()..T::DISCRIMINATOR.len()+std::mem::size_of::<T>()])))
    }
    #[inline(always)]
    fn check_size(&self, data: &[u8]) -> Result<()> {
        if crate::TARE_ANCHOR_MAJOR >= 1 {
            let required = T::DISCRIMINATOR.len()
                .checked_add(std::mem::size_of::<T>())
                .ok_or(ErrorCode::AccountDidNotDeserialize)?;
            if data.len() < required {
                return Err(ErrorCode::AccountDidNotDeserialize.into());
            }
        }
        Ok(())
    }
    pub fn exit(&self, program: &Pubkey) -> Result<()> {
        if &T::owner() == program
            && !(self.info.owner_ref() == &crate::System::id() && self.info.data_is_empty())
        {
            if crate::TARE_ANCHOR_MAJOR >= 1 {
                self.check_size(&self.info.try_borrow_data()?)?;
            }
            use std::io::Write;
            (&mut *self.info.try_borrow_mut_data()?)
                .write_all(T::DISCRIMINATOR)
                .unwrap();
        }
        Ok(())
    }
    pub fn __tare_info(&self) -> AccountView<'info> {
        self.info
    }
}
fn check<T: Discriminator>(data: &[u8]) -> Result<()> {
    if data.len() < T::DISCRIMINATOR.len() {
        return Err(ErrorCode::AccountDiscriminatorNotFound.into());
    }
    if &data[..T::DISCRIMINATOR.len()] != T::DISCRIMINATOR {
        return Err(ErrorCode::AccountDiscriminatorMismatch.into());
    }
    Ok(())
}
impl<'info, T: ZeroCopy + Owner> FromAccountInfo<'info> for AccountLoader<'info, T> {
    fn try_from_info(info: AccountView<'info>) -> Result<Self> {
        let value = Self::__try_from_unchecked(info, false)?;
        check::<T>(&info.try_borrow_data()?)?;
        Ok(value)
    }
}
impl<'info, T: ZeroCopy + Owner> Key for AccountLoader<'info, T> {
    fn key(&self) -> Pubkey {
        *self.info.key_ref()
    }
}
impl<'info, T: ZeroCopy + Owner> ToAccountInfo<'info> for AccountLoader<'info, T> {
    fn to_account_info(&self) -> AccountInfo<'info> {
        self.info.sdk().clone()
    }
}
impl<'info, T: ZeroCopy + Owner> AsRef<AccountInfo<'info>> for AccountLoader<'info, T> {
    fn as_ref(&self) -> &AccountInfo<'info> {
        self.info.sdk()
    }
}
impl<'info, T: ZeroCopy + Owner> AccountsClose<'info> for AccountLoader<'info, T> {
    fn close(&self, destination: AccountInfo<'info>) -> Result<()> {
        crate::close_view(self.info, destination)
    }
}
pub struct LoaderRef<'info, T> {
    _data: crate::input::DataRef<'info>,
    pointer: *const T,
}
impl<T> Deref for LoaderRef<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.pointer }
    }
}
pub struct LoaderRefMut<'info, T> {
    _data: crate::input::DataRefMut<'info>,
    pointer: *mut T,
}
impl<T> Deref for LoaderRefMut<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.pointer }
    }
}
impl<T> DerefMut for LoaderRefMut<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.pointer }
    }
}
use crate::Id;

impl<T:ZeroCopy+Owner+std::fmt::Debug> std::fmt::Debug for AccountLoader<'_,T> {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {
        f.debug_struct("AccountLoader").field("acc_info",&self.info).field("phantom",&PhantomData::<&T>).finish()
    }
}
