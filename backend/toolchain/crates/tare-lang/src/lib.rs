//! Tare's dependency-compatible Anchor 0.31 runtime.
extern crate self as anchor_lang;
#[cfg(feature = "anchor-debug")]
compile_error!("Tare does not implement the anchor-debug feature");
pub use borsh;
pub use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
pub use solana_program;
pub use solana_program::pubkey::Pubkey;
use std::{
    cell::Cell,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};
pub use tare_lang_macros::{
    access_control, account, constant, declare_id, declare_program, pubkey, emit_cpi, error_code, event, event_cpi, instruction, program, zero_copy,
    Accounts, InitSpace,
};
#[cfg(feature="interface-instructions")]
pub use tare_lang_macros::interface;
pub mod bpf_upgradeable_state;
pub use bpf_upgradeable_state::ProgramData;
pub mod context;
pub mod cpi;
pub use context::CpiContext;
pub mod error;
pub use error::{Error, ErrorCode, Source};
pub mod event;
pub mod idl;
pub mod input;
pub mod interface;
#[cfg_attr(not(tare_sdk), path="loader_raw.rs")]
pub mod loader;
pub mod system_program;
pub mod sysvar;
pub use loader::AccountLoader;
pub use sysvar::Sysvar;
pub trait ZeroCopy: Discriminator + Copy + Clone + bytemuck::Zeroable + bytemuck::Pod {}
pub use interface::{Interface, InterfaceAccount};
pub mod rent;
pub use input::{AccountCursor, AccountView, RemainingAccounts};
#[cfg(tare_sdk)]
pub use solana_program::account_info::{AccountInfo,next_account_info};
#[cfg(not(tare_sdk))]
pub use input::{AccountInfo,next_account_info};
pub type Result<T> = std::result::Result<T, error::Error>;
impl<'info> FromAccountInfo<'info> for AccountInfo<'info> {
    #[inline(always)] fn try_from_info(info:AccountView<'info>)->Result<Self>{Ok(info.sdk().clone())}
}
impl Key for AccountInfo<'_>{#[inline(always)] fn key(&self)->Pubkey{*self.key}}
impl<'info> ToAccountInfo<'info> for AccountInfo<'info>{#[inline(always)] fn to_account_info(&self)->AccountInfo<'info>{self.clone()} #[inline(always)] fn __tare_info(&self)->AccountView<'info>{AccountView::from_sdk(self)}}

pub trait Id {
    fn id() -> Pubkey;
}
pub trait Owners {
    fn owners() -> &'static [Pubkey];
}
pub trait Ids {
    fn ids() -> &'static [Pubkey];
}
pub trait CheckOwner {
    fn check_owner(owner: &Pubkey) -> Result<()>;
}
pub trait CheckId {
    fn check_id(id: &Pubkey) -> Result<()>;
}
impl<T: Owners> CheckOwner for T {
    fn check_owner(owner: &Pubkey) -> Result<()> {
        if T::owners().contains(owner) {
            Ok(())
        } else {
            Err(Error::from(ErrorCode::AccountOwnedByWrongProgram).with_account_key(*owner))
        }
    }
}
impl<T: Ids> CheckId for T {
    fn check_id(id: &Pubkey) -> Result<()> {
        if T::ids().contains(id) {
            Ok(())
        } else {
            Err(Error::from(ErrorCode::InvalidProgramId).with_account_key(*id))
        }
    }
}
pub trait Owner {
    fn owner() -> Pubkey;
}
pub trait Key {
    fn key(&self) -> Pubkey;
}
impl Key for Pubkey {
    #[inline(always)]
    fn key(&self) -> Pubkey { *self }
}
pub(crate) const TARE_ANCHOR_MAJOR: u8 = 0;
pub trait Event: AnchorSerialize + AnchorDeserialize + Discriminator {
    fn data(&self) -> Vec<u8>;
}
pub trait Discriminator {
    const DISCRIMINATOR: &'static [u8];
}
pub trait Space {
    const INIT_SPACE: usize;
}
macro_rules! fixed_space { ($($ty:ty => $size:expr),* $(,)?)=>{$(impl Space for $ty {const INIT_SPACE:usize=$size;})*}; }
fixed_space!(bool=>1,u8=>1,i8=>1,u16=>2,i16=>2,u32=>4,i32=>4,u64=>8,i64=>8,u128=>16,i128=>16,f32=>4,f64=>8,Pubkey=>32);
impl<T: Space, const N: usize> Space for [T; N] {
    const INIT_SPACE: usize = T::INIT_SPACE * N;
}
impl<T: Space> Space for Option<T> {
    const INIT_SPACE: usize = 1 + T::INIT_SPACE;
}

pub trait AccountDeserialize: Sized {
    #[doc(hidden)] const __TARE_CANONICAL_DIRECT: bool=false;
    fn try_deserialize(buf: &mut &[u8]) -> Result<Self>;
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self>;
    /// Returns a direct view only for bit-valid fields at their unchanged Rust offsets.
    /// The caller owns the writable account and excludes observable raw aliases.
    unsafe fn try_direct(_info: &AccountView, _unchecked: bool) -> Result<Option<*mut Self>> {
        Ok(None)
    }
    #[inline(always)]
    fn discriminator() -> &'static [u8] {
        &[]
    }
}
pub trait AccountSerialize {
    fn try_serialize<W: std::io::Write>(&self, _writer: &mut W) -> Result<()> {
        Ok(())
    }
    fn try_serialize_slice(&self, mut data: &mut [u8]) -> Result<()> {
        self.try_serialize(&mut data)
    }
}
pub trait Bumps {
    type Bumps: Default;
}
pub trait DuplicateMutableAccountKeys {
    fn duplicate_mutable_account_keys(&self) -> Vec<Pubkey>;
}
pub trait AccountsClose<'info> {
    fn close(&self, destination: AccountInfo<'info>) -> Result<()>;
}
pub trait Accounts<'info>: Bumps + Sized {
    fn try_accounts(
        program_id: &Pubkey,
        accounts: &mut AccountCursor<'info>,
        data: &[u8],
        bumps: &mut Self::Bumps,
        reallocs: &mut std::collections::BTreeSet<Pubkey>,
        direct: bool,
    ) -> Result<Self>;
    /// # Safety
    /// Caller supplies uninitialized storage. `Ok(())` initializes `output`;
    /// an error leaves it untouched. Generated implementations write only
    /// after every validation succeeds.
    #[doc(hidden)]
    unsafe fn __try_accounts_into(
        program_id: &Pubkey,
        accounts: &mut AccountCursor<'info>,
        data: &[u8],
        bumps: &mut Self::Bumps,
        reallocs: &mut std::collections::BTreeSet<Pubkey>,
        direct: bool,
        output: &mut std::mem::MaybeUninit<Self>,
    ) -> Result<()> {
        output.write(Self::try_accounts(program_id, accounts, data, bumps, reallocs, direct)?);
        Ok(())
    }
    #[doc(hidden)]
    unsafe fn __try_accounts_into_mode<const RAW:bool>(program_id:&Pubkey,accounts:&mut AccountCursor<'info>,data:&[u8],bumps:&mut Self::Bumps,reallocs:&mut std::collections::BTreeSet<Pubkey>,direct:bool,output:&mut std::mem::MaybeUninit<Self>)->Result<()> {
        Self::__try_accounts_into(program_id,accounts,data,bumps,reallocs,direct,output)
    }
    #[inline(always)]
    fn exit(&self, _program_id: &Pubkey) -> Result<()> {
        Ok(())
    }
}
pub trait FromAccountInfo<'info>: Sized {
    fn try_from_info(info: AccountView<'info>) -> Result<Self>;
}
pub trait ToAccountInfo<'info> {
    fn to_account_info(&self) -> AccountInfo<'info>;
    fn __tare_info(&self)->AccountView<'info>{AccountView::from_sdk(&self.to_account_info())}
    fn __tare_meta(&self, is_signer: Option<bool>) -> solana_program::instruction::AccountMeta {
        let info = self.to_account_info();
        let signer = is_signer.unwrap_or(info.is_signer);
        if info.is_writable {
            solana_program::instruction::AccountMeta::new(*info.key, signer)
        } else {
            solana_program::instruction::AccountMeta::new_readonly(*info.key, signer)
        }
    }
}
pub trait CpiAccounts<'info> {
    type Cpi: ToAccountInfos<'info> + ToAccountMetas + CpiInvoke<'info>;
}
pub trait CpiInvoke<'info> {
    fn __invoke(
        &self,
        program: AccountInfo<'info>,
        data: &[u8],
        signers: &[&[&[u8]]],
        remaining: &[AccountInfo<'info>],
    ) -> Result<()>;
}
pub trait ToAccountInfos<'info> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>>;
}
pub trait ToAccountMetas {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<solana_program::instruction::AccountMeta>;
}
pub trait InstructionData: Discriminator + AnchorSerialize {
    fn data(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(256);
        bytes.extend_from_slice(Self::DISCRIMINATOR);
        self.serialize(&mut bytes).expect("Should always serialize");
        bytes
    }
    fn write_to(&self, data: &mut Vec<u8>) {
        data.clear();
        data.extend_from_slice(Self::DISCRIMINATOR);
        self.serialize(data).expect("Should always serialize");
    }
}

impl<'info, T: ToAccountInfo<'info>> ToAccountInfos<'info> for T {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.to_account_info()]
    }
}
impl<'info, T: ToAccountInfo<'info>> ToAccountMetas for T {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<solana_program::instruction::AccountMeta> {
        vec![self.__tare_meta(is_signer)]
    }
}
pub struct Context<'a, 'b, 'c, 'info, T: Bumps> {
    pub program_id: &'a Pubkey,
    pub accounts: &'b mut T,
    pub remaining_accounts: &'c [AccountInfo<'info>],
    pub bumps: T::Bumps,
}
impl<'a, 'b, 'c, 'info, T: Bumps> Context<'a, 'b, 'c, 'info, T> {
    #[inline(always)]
    pub fn new(
        program_id: &'a Pubkey,
        accounts: &'b mut T,
        remaining_accounts: &'c [AccountInfo<'info>],
        bumps: T::Bumps,
    ) -> Self {
        Self {
            program_id,
            accounts,
            remaining_accounts,
            bumps,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct System;
impl Id for System {
    fn id() -> Pubkey {
        solana_program::system_program::ID
    }
}
macro_rules! info_wrapper {
    ($name:ident, $check:expr) => {
        #[derive(Clone, Copy)]
        pub struct $name<'info> {
            info: AccountView<'info>,
        }
        impl<'info> FromAccountInfo<'info> for $name<'info> {
            #[inline(always)]
            fn try_from_info(info: AccountView<'info>) -> Result<Self> {
                ($check)(&info)?;
                Ok(Self { info })
            }
        }
        impl std::fmt::Debug for $name<'_> {
            fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {
                if stringify!($name)=="UncheckedAccount" {f.debug_tuple("UncheckedAccount").field(&self.info).finish()}
                else {f.debug_struct(stringify!($name)).field("info",&self.info).finish()}
            }
        }
        impl<'info> Deref for $name<'info> {
            type Target = AccountInfo<'info>;
            #[inline(always)]
            fn deref(&self) -> &Self::Target {
                self.info.sdk()
            }
        }
        impl<'info> $name<'info> {
            #[inline(always)]
            pub fn __tare_info(&self) -> AccountView<'info> {
                self.info
            }
            #[inline(always)] pub fn lamports(&self)->u64 {self.info.lamports()}
            #[inline(always)] pub fn data_len(&self)->usize {self.info.data_len()}
            #[inline(always)] pub fn data_is_empty(&self)->bool {self.info.data_is_empty()}
        }
        impl<'info> AsRef<AccountInfo<'info>> for $name<'info> {
            fn as_ref(&self) -> &AccountInfo<'info> {
                self.info.sdk()
            }
        }
        impl Key for $name<'_> {
            #[inline(always)]
            fn key(&self) -> Pubkey {
                *self.info.key_ref()
            }
        }
        impl<'info> ToAccountInfo<'info> for $name<'info> {
            #[inline(always)]
            fn __tare_info(&self)->AccountView<'info>{self.info}
            #[inline(always)]
            fn to_account_info(&self) -> AccountInfo<'info> {
                self.info.sdk().clone()
            }
        }
    };
}
info_wrapper!(Signer, |i: &AccountView| if i.is_signer() {
    Ok::<(), error::Error>(())
} else {
    Err(error::ErrorCode::AccountNotSigner.into())
});
info_wrapper!(UncheckedAccount, |_i: &AccountView| Ok::<(), error::Error>(
    ()
));
info_wrapper!(
    SystemAccount,
    |i: &AccountView| if i.owner_ref() == &solana_program::system_program::ID {
        Ok::<(), error::Error>(())
    } else {
        Err(error::ErrorCode::AccountNotSystemOwned.into())
    }
);
#[derive(Clone, Copy)]
pub struct Program<'info, T> {
    info: AccountView<'info>,
    _type: PhantomData<T>,
}
impl<'info, T: Id> FromAccountInfo<'info> for Program<'info, T> {
    #[inline(always)]
    fn try_from_info(info: AccountView<'info>) -> Result<Self> {
        if info.key_ref() != &T::id() {
            return Err(error::Error::from(error::ErrorCode::InvalidProgramId)
                .with_pubkeys((*info.key_ref(), T::id())));
        }
        if !info.executable() {
            return Err(error::ErrorCode::InvalidProgramExecutable.into());
        }
        Ok(Self {
            info,
            _type: PhantomData,
        })
    }
}
impl<'info> FromAccountInfo<'info> for Program<'info, ()> {
    #[inline(always)]
    fn try_from_info(info: AccountView<'info>) -> Result<Self> {
        if !info.executable() { return Err(error::ErrorCode::InvalidProgramExecutable.into()); }
        Ok(Self { info, _type: PhantomData })
    }
}
impl<'info, T> Deref for Program<'info, T> {
    type Target = AccountInfo<'info>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.info.sdk()
    }
}
impl<'info, T> Program<'info, T> {
    pub fn programdata_address(&self)->Result<Option<Pubkey>> {crate::bpf_upgradeable_state::programdata_address(self.info)}
    #[inline(always)]
    pub fn __tare_info(&self) -> AccountView<'info> {
        self.info
    }
}
impl<T> Key for Program<'_, T> {
    #[inline(always)]
    fn key(&self) -> Pubkey {
        *self.info.key_ref()
    }
}
impl<'info, T> ToAccountInfo<'info> for Program<'info, T> {
    #[inline(always)]
    fn to_account_info(&self) -> AccountInfo<'info> {
        self.info.sdk().clone()
    }
}
#[cfg_attr(not(tare_sdk), repr(u64))]
enum AccountValue<T> {
    Snapshot(T),
    #[cfg(not(tare_sdk))]
    Direct(*mut T),
}
#[repr(C)]
pub struct Account<'info, T> {
    value: AccountValue<T>,
    info: AccountView<'info>,
    #[cfg(not(all(tare_sdk,tare_external_entry)))]
    dirty: Cell<bool>,
    #[cfg(not(all(tare_sdk,tare_external_entry)))]
    initialize: bool,
}
impl<'info, T: AccountDeserialize + Owner> FromAccountInfo<'info> for Account<'info, T> {
    #[inline(always)]
    fn try_from_info(info: AccountView<'info>) -> Result<Self> {
        Self::load(info, false, false)
    }
}
impl<'info, T: AccountDeserialize + Owner> Account<'info, T> {
    #[inline(always)]
    fn load(info: AccountView<'info>, unchecked: bool, direct: bool) -> Result<Self> {
        if info.owner_ref() == &solana_program::system_program::ID && info.lamports() == 0 {
            return Err(error::ErrorCode::AccountNotInitialized.into());
        }
        if info.owner_ref() != &T::owner() {
            return Err(
                error::Error::from(error::ErrorCode::AccountOwnedByWrongProgram)
                    .with_pubkeys((*info.owner_ref(), T::owner())),
            );
        }
        // The generated validator requests this only for a mutable field and
        // still runs Anchor's mut check before the handler can write through it.
        #[cfg(not(tare_sdk))]
        if direct {
            if let Some(pointer) = unsafe { T::try_direct(&info, unchecked)? } {
                return Ok(Self {
                    value: AccountValue::Direct(pointer),
                    info,
                    #[cfg(not(all(tare_sdk,tare_external_entry)))]
                    dirty: Cell::new(unchecked),
                    #[cfg(not(all(tare_sdk,tare_external_entry)))]
                    initialize: unchecked,
                });
            }
        }
        let data = info.try_borrow_data()?;
        let mut bytes = &*data;
        let value = if unchecked {
            T::try_deserialize_unchecked(&mut bytes)?
        } else {
            T::try_deserialize(&mut bytes)?
        };
        drop(data);
        Ok(Self {
            value: AccountValue::Snapshot(value),
            info,
            #[cfg(not(all(tare_sdk,tare_external_entry)))]
            dirty: Cell::new(unchecked || info.__aliased()),
            #[cfg(not(all(tare_sdk,tare_external_entry)))]
            initialize: unchecked,
        })
    }
    pub fn try_from(info:&'info AccountInfo<'info>)->Result<Self> {Self::load(unsafe{AccountView::from_sdk_borrowed(info)},false,false)}
    #[inline(always)]
    pub fn try_from_unchecked(info: &'info AccountInfo<'info>) -> Result<Self> {
        Self::load(unsafe{AccountView::from_sdk_borrowed(info)}, true, false)
    }
    #[inline(always)]
    pub fn __try_from_info(info: AccountView<'info>, direct: bool) -> Result<Self> {
        Self::load(info, false, direct)
    }
    #[inline(always)]
    pub fn __try_from_unchecked(info: AccountView<'info>, direct: bool) -> Result<Self> {
        Self::load(info, true, direct)
    }
    pub fn reload(&mut self) -> Result<()> {
        if TARE_ANCHOR_MAJOR >= 1 && self.info.owner_ref() != &T::owner() {
            return Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
                .with_pubkeys((*self.info.owner_ref(), T::owner())));
        }
        self.value =
            AccountValue::Snapshot(T::try_deserialize(&mut &*self.info.try_borrow_data()?)?);
        #[cfg(not(all(tare_sdk,tare_external_entry)))]
        self.dirty.set(true);
        Ok(())
    }
    pub fn set_inner(&mut self, value: T) {
        match &mut self.value {
            AccountValue::Snapshot(slot) => *slot = value,
            #[cfg(not(tare_sdk))]
            AccountValue::Direct(pointer) => unsafe { **pointer = value },
        }
        #[cfg(not(all(tare_sdk,tare_external_entry)))]
        self.dirty.set(true);
    }
    pub fn into_inner(self) -> T {
        match self.value {
            AccountValue::Snapshot(value) => value,
            #[cfg(not(tare_sdk))]
            AccountValue::Direct(pointer) => unsafe { pointer.read() },
        }
    }
}
impl<T> Deref for Account<'_, T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &T {
        match &self.value {
            AccountValue::Snapshot(value) => value,
            #[cfg(not(tare_sdk))]
            AccountValue::Direct(pointer) => unsafe { &**pointer },
        }
    }
}
impl<T> DerefMut for Account<'_, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut T {
        #[cfg(not(all(tare_sdk,tare_external_entry)))]
        self.dirty.set(true);
        match &mut self.value {
            AccountValue::Snapshot(value) => value,
            #[cfg(not(tare_sdk))]
            AccountValue::Direct(pointer) => unsafe { &mut **pointer },
        }
    }
}
impl<T> Key for Account<'_, T> {
    #[inline(always)]
    fn key(&self) -> Pubkey {
        *self.info.key_ref()
    }
}
impl<'info, T> ToAccountInfo<'info> for Account<'info, T> {
    #[inline(always)]
    fn __tare_info(&self) -> AccountView<'info> { self.info }
    #[inline(always)]
    fn to_account_info(&self) -> AccountInfo<'info> {
        #[cfg(not(all(tare_sdk,tare_external_entry)))]
        self.dirty.set(true);
        self.info.sdk().clone()
    }
}
impl<'info, T> Account<'info, T> {
    #[inline(always)] fn __dirty(&self)->bool {
        #[cfg(all(tare_sdk,tare_external_entry))]{true}
        #[cfg(not(all(tare_sdk,tare_external_entry)))]{self.dirty.get()}
    }

    #[inline(always)]
    pub fn __tare_info(&self) -> AccountView<'info> {
        self.info
    }
}
impl<T: AccountSerialize + AccountDeserialize + Owner> Account<'_, T> {
    #[cfg_attr(tare_sdk, inline(never))]
    #[cfg_attr(not(tare_sdk), inline(always))]
    pub fn exit(&self, program_id: &Pubkey) -> Result<()> {
        // Only generated canonical layouts admit direct storage. Their bytes
        // already are the serialized value; custom implementations opt out.
        #[cfg(not(tare_sdk))]
        if T::__TARE_CANONICAL_DIRECT && matches!(self.value,AccountValue::Direct(_)) && !self.initialize {return Ok(());}
        if TARE_ANCHOR_MAJOR >= 1 && &T::owner() == program_id
            && self.info.owner_ref() != program_id
            && !(self.info.owner_ref() == &solana_program::system_program::ID && self.info.data_is_empty())
        {
            return self.__exit_unowned(program_id);
        }
        #[cfg(not(tare_sdk))]
        if matches!(self.value,AccountValue::Direct(_)) && !self.initialize {return Ok(());}
        if (self.__dirty() || self.info.sdk_exposed())
            && &T::owner() == program_id
            && !(self.info.owner_ref() == &solana_program::system_program::ID
                && self.info.data_is_empty())
        {
            match &self.value {
                AccountValue::Snapshot(value) => {
                    let mut data = self.info.try_borrow_mut_data()?;
                    value.try_serialize_slice(&mut data)?;
                }
                #[cfg(not(tare_sdk))]
            AccountValue::Direct(_) => {
                    if self.initialize {
                        let mut data = self.info.try_borrow_mut_data()?;
                        let disc = T::discriminator();
                        data[..disc.len()].copy_from_slice(disc);
                    }
                }
            }
        }
        Ok(())
    }
}
impl<T: AccountSerialize> Account<'_, T> {
    pub(crate) fn __exit_unowned(&self, program_id: &Pubkey) -> Result<()> {
        let data = self.info.try_borrow_data()?;
        let mut writer = CompareWriter { data: &data, pos: 0, matches: true };
        match &self.value {
            AccountValue::Snapshot(value) => value.try_serialize(&mut writer)?,
            #[cfg(not(tare_sdk))]
            AccountValue::Direct(value) => unsafe { (&**value).try_serialize(&mut writer)? },
        }
        if writer.matches {
            Ok(())
        } else {
            Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
                .with_pubkeys((*self.info.owner_ref(), *program_id)))
        }
    }
}
struct CompareWriter<'a> {
    data: &'a [u8],
    pos: usize,
    matches: bool,
}
impl std::io::Write for CompareWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let end = self.pos.saturating_add(bytes.len());
        self.matches &= self.data.get(self.pos..end) == Some(bytes);
        self.pos = end;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}
impl<'info, T> AccountsClose<'info> for Account<'info, T> {
    fn close(&self, destination: AccountInfo<'info>) -> Result<()> {
        crate::close_view(self.info, destination)?;
        Ok(())
    }
}
#[macro_export]
macro_rules! source {
    () => {
        $crate::error::Source {
            filename: file!(),
            line: line!(),
        }
    };
}
pub fn log_fmt(args: std::fmt::Arguments<'_>) {
    use std::fmt::Write;
    let mut output = error::StackString::<1024>::new();
    if output.write_fmt(args).is_err() {
        solana_program::log::sol_log(&std::fmt::format(args));
    } else {
        solana_program::log::sol_log(output.as_str());
    }
}
#[macro_export]
macro_rules! msg { ($msg:expr) => { $crate::solana_program::log::sol_log($msg) }; ($($arg:tt)*) => { $crate::solana_program::msg!($($arg)*) }; }
#[macro_export]
macro_rules! error {
    ($err:expr) => {
        $crate::error::Error::from($err).with_source($crate::error::Source {
            filename: file!(),
            line: line!(),
        })
    };
}
#[macro_export]
macro_rules! err {
    ($err:expr) => {
        Err($crate::error!($err))
    };
}
#[macro_export]
macro_rules! require {
    ($test:expr, $error:tt $(,)?) => {
        if !($test) {
            return Err($crate::error!($crate::error::ErrorCode::$error));
        }
    };
    ($test:expr, $error:expr $(,)?) => {
        if !($test) {
            return Err($crate::error!($error));
        }
    };
}
#[macro_export]
macro_rules! require_eq {
    ($left:expr, $right:expr, $error:expr $(,)?) => {
        if $left != $right {
            return Err(error!($error).with_values(($left, $right)));
        }
    };
    ($left:expr, $right:expr $(,)?) => {
        $crate::require_eq!($left, $right, $crate::error::ErrorCode::RequireEqViolated)
    };
}
#[macro_export]
macro_rules! require_neq {
    ($left:expr, $right:expr, $error:expr $(,)?) => {
        if $left == $right {
            return Err(error!($error).with_values(($left, $right)));
        }
    };
    ($left:expr, $right:expr $(,)?) => {
        $crate::require_neq!($left, $right, $crate::error::ErrorCode::RequireNeqViolated)
    };
}
#[macro_export]
macro_rules! require_gt {
    ($left:expr, $right:expr, $error:expr $(,)?) => {
        if $left <= $right {
            return Err(error!($error).with_values(($left, $right)));
        }
    };
    ($left:expr, $right:expr $(,)?) => {
        $crate::require_gt!($left, $right, $crate::error::ErrorCode::RequireGtViolated)
    };
}
#[macro_export]
macro_rules! require_gte {
    ($left:expr, $right:expr, $error:expr $(,)?) => {
        if $left < $right {
            return Err(error!($error).with_values(($left, $right)));
        }
    };
    ($left:expr, $right:expr $(,)?) => {
        $crate::require_gte!($left, $right, $crate::error::ErrorCode::RequireGteViolated)
    };
}
#[macro_export]
macro_rules! require_keys_eq {
    ($left:expr, $right:expr, $error:expr $(,)?) => {
        if $left != $right {
            return Err(error!($error).with_pubkeys(($left, $right)));
        }
    };
    ($left:expr, $right:expr $(,)?) => {
        $crate::require_keys_eq!(
            $left,
            $right,
            $crate::error::ErrorCode::RequireKeysEqViolated
        )
    };
}
#[macro_export]
macro_rules! require_keys_neq {
    ($left:expr, $right:expr, $error:expr $(,)?) => {
        if $left == $right {
            return Err(error!($error).with_pubkeys(($left, $right)));
        }
    };
    ($left:expr, $right:expr $(,)?) => {
        $crate::require_keys_neq!(
            $left,
            $right,
            $crate::error::ErrorCode::RequireKeysNeqViolated
        )
    };
}

pub mod prelude {
    #[cfg(feature="interface-instructions")]
    pub use crate::interface;
    pub use crate::{
        access_control, account, borsh, constant, declare_id, declare_program, emit, emit_cpi, err, error,
        error_code, event, event_cpi, instruction, msg, next_account_info, program, pubkey, require, require_eq, require_gt,
        require_gte, require_keys_eq, require_keys_neq, require_neq, source, zero_copy, Account,
        AccountDeserialize, AccountInfo, AccountLoader, AccountSerialize, Accounts, AccountsClose,
        AnchorDeserialize, AnchorSerialize, Bumps, CheckId, CheckOwner, Context, CpiAccounts,
        CpiContext, Discriminator, DuplicateMutableAccountKeys, Error, ErrorCode, Event, FromAccountInfo, Id, Ids, InitSpace,
        InstructionData, Interface, InterfaceAccount, Key, Lamports, Owner, Owners, Program,
        Pubkey, ProgramData, Result, Signer, Source, Space, System, SystemAccount, Sysvar, ToAccountInfo,
        ToAccountInfos, ToAccountMetas, UncheckedAccount, ZeroCopy,
    };
    pub use solana_program::{
        bpf_loader_upgradeable::UpgradeableLoaderState,
        clock::Clock,
        instruction::AccountMeta,
        program_error::ProgramError,
        rent::Rent,
        sysvar::Sysvar as SolanaSysvar,
        sysvar::{
            epoch_schedule::EpochSchedule, instructions::Instructions, rewards::Rewards,
            slot_hashes::SlotHashes, slot_history::SlotHistory, stake_history::StakeHistory,
        },
    };
}
#[macro_export]
macro_rules! emit {
    ($event:expr $(,)?) => {{
        $crate::solana_program::log::sol_log_data(&[&$crate::Event::data(&$event)]);
    }};
}

pub mod __private {
    pub use bytemuck;
    pub trait ZeroCopyAccessor<T> {
        fn get(&self) -> T;
        fn set(input: &T) -> Self;
    }
    impl ZeroCopyAccessor<crate::Pubkey> for [u8; 32] {
        fn get(&self) -> crate::Pubkey {
            crate::Pubkey::from(*self)
        }
        fn set(input: &crate::Pubkey) -> Self {
            input.to_bytes()
        }
    }
}

fn close_view(info:AccountView<'_>,destination:AccountInfo<'_>)->Result<()> {
    if TARE_ANCHOR_MAJOR >= 1 {
        if info.key_ref() == destination.key {
            return Err(ErrorCode::ConstraintClose.into());
        }
        destination.add_lamports(info.lamports())?;
        **info.lamports_handle().borrow_mut() = 0;
        info.assign(&solana_program::system_program::ID);
        return AccountView::realloc(&info, 0, false);
    }
    let balance=destination.lamports();
    **destination.lamports.borrow_mut()=balance.checked_add(info.lamports()).unwrap();
    **info.lamports_handle().borrow_mut()=0;
    info.assign(&solana_program::system_program::ID);
    info.realloc(0,false)
}
pub fn close(info: AccountInfo<'_>, destination: AccountInfo<'_>) -> Result<()> {
    if TARE_ANCHOR_MAJOR >= 1 {
        if info.key == destination.key {
            return Err(ErrorCode::ConstraintClose.into());
        }
        destination.add_lamports(info.lamports())?;
        **info.lamports.borrow_mut() = 0;
        info.assign(&solana_program::system_program::ID);
        return info.realloc(0, false).map_err(Into::into);
    }
    let balance = destination.lamports();
    **destination.lamports.borrow_mut() = balance.checked_add(info.lamports()).unwrap();
    **info.lamports.borrow_mut() = 0;
    info.assign(&solana_program::system_program::ID);
    info.realloc(0, false).map_err(Into::into)
}
pub trait Lamports<'info>: AsRef<AccountInfo<'info>> {
    fn get_lamports(&self) -> u64 {
        self.as_ref().lamports()
    }
    fn add_lamports(&self, amount: u64) -> Result<&Self> {
        **self.as_ref().try_borrow_mut_lamports()? = self
            .get_lamports()
            .checked_add(amount)
            .ok_or(solana_program::program_error::ProgramError::ArithmeticOverflow)?;
        Ok(self)
    }
    fn sub_lamports(&self, amount: u64) -> Result<&Self> {
        **self.as_ref().try_borrow_mut_lamports()? = self
            .get_lamports()
            .checked_sub(amount)
            .ok_or(solana_program::program_error::ProgramError::ArithmeticOverflow)?;
        Ok(self)
    }
}
impl<'info, T: AsRef<AccountInfo<'info>>> Lamports<'info> for T {}

impl<'info, T> AsRef<AccountInfo<'info>> for Account<'info, T> {
    fn as_ref(&self) -> &AccountInfo<'info> {
        #[cfg(not(all(tare_sdk,tare_external_entry)))]
        self.dirty.set(true);
        self.info.sdk()
    }
}
impl<'info, T> AsRef<AccountInfo<'info>> for Program<'info, T> {
    fn as_ref(&self) -> &AccountInfo<'info> {
        self.info.sdk()
    }
}

impl<'info, T: ToAccountInfo<'info>> ToAccountInfo<'info> for Box<T> {
    #[inline(always)]
    fn __tare_info(&self) -> AccountView<'info> { T::__tare_info(self) }
    #[inline(always)]
    fn to_account_info(&self) -> AccountInfo<'info> {
        T::to_account_info(self)
    }
}
impl<'info, T: ToAccountInfos<'info>> ToAccountInfos<'info> for Option<T> {
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        self.as_ref()
            .map_or_else(Vec::new, |v| v.to_account_infos())
    }
}
impl<T: ToAccountMetas> ToAccountMetas for Option<T> {
    fn to_account_metas(
        &self,
        is_signer: Option<bool>,
    ) -> Vec<solana_program::instruction::AccountMeta> {
        self.as_ref()
            .expect("Cannot run `to_account_metas` on None")
            .to_account_metas(is_signer)
    }
}
impl<'info, T: AccountsClose<'info>> AccountsClose<'info> for Box<T> {
    fn close(&self, destination: AccountInfo<'info>) -> Result<()> {
        T::close(self, destination)
    }
}

impl<T:std::fmt::Debug> std::fmt::Debug for Program<'_,T> {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {f.debug_struct("Program").field("info",&self.info).finish()}
}
impl<T:AccountSerialize+AccountDeserialize+Clone+std::fmt::Debug> std::fmt::Debug for Account<'_,T> {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {f.debug_struct("Account").field("account",&**self).field("info",&self.info).finish()}
}

/// Anchor's canonical module paths for the implemented account wrappers.
pub mod accounts {
    pub mod account { pub use crate::Account; }
    pub mod account_info {}
    pub mod account_loader { pub use crate::AccountLoader; }
    pub mod boxed {}
    pub mod interface { pub use crate::Interface; }
    pub mod interface_account { pub use crate::InterfaceAccount; }
    pub mod option {}
    pub mod program { pub use crate::Program; }
    pub mod signer { pub use crate::Signer; }
    pub mod system_account { pub use crate::SystemAccount; }
    pub mod sysvar { pub use crate::Sysvar; }
    pub mod unchecked_account { pub use crate::UncheckedAccount; }
}

/// Shared field load boundary: one instantiation per account type and input mode.
#[doc(hidden)]
#[inline(never)]
pub unsafe fn __load_field_into<'info, T: FromAccountInfo<'info>, const RAW: bool>(
    cursor: &mut AccountCursor<'info>,
    name: &'static str,
    output: &mut std::mem::MaybeUninit<T>,
) -> Result<()> {
    let raw = cursor.next_mode::<RAW>().map_err(|e|e.__tare_with_account_name_static(name))?;
    let value = T::try_from_info(raw).map_err(|e|e.__tare_with_account_name_static(name))?;
    output.write(value);
    Ok(())
}

#[doc(hidden)]
#[inline(never)]
pub fn __load_boxed_field<'info, T: FromAccountInfo<'info>, const RAW: bool>(
    cursor: &mut AccountCursor<'info>, name: &'static str,
) -> Result<Box<T>> {
    let raw=cursor.next_mode::<RAW>().map_err(|e|e.__tare_with_account_name_static(name))?;
    let value=T::try_from_info(raw).map_err(|e|e.__tare_with_account_name_static(name))?;
    Ok(Box::new(value))
}

#[doc(hidden)]
#[inline(never)]
pub unsafe fn __load_optional_field_into<'info,T:FromAccountInfo<'info>,const RAW:bool>(
    cursor:&mut AccountCursor<'info>,program:&Pubkey,name:&'static str,
    output:&mut std::mem::MaybeUninit<Option<T>>,
)->Result<()> {
    let value=match cursor.next_optional_mode::<RAW>(program).map_err(|e|e.__tare_with_account_name_static(name))? {
        Some(raw)=>Some(T::try_from_info(raw).map_err(|e|e.__tare_with_account_name_static(name))?),
        None=>None,
    };
    output.write(value); Ok(())
}
#[doc(hidden)]
#[inline(never)]
pub fn __load_optional_boxed_field<'info,T:FromAccountInfo<'info>,const RAW:bool>(
    cursor:&mut AccountCursor<'info>,program:&Pubkey,name:&'static str,
)->Result<Option<Box<T>>> {
    match cursor.next_optional_mode::<RAW>(program).map_err(|e|e.__tare_with_account_name_static(name))? {
        Some(raw)=>Ok(Some(Box::new(T::try_from_info(raw).map_err(|e|e.__tare_with_account_name_static(name))?))),
        None=>Ok(None),
    }
}
