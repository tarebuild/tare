//! Views over the upgradeable-loader input ABI; account descriptors are built on demand.
use crate::{error::ErrorCode, Pubkey, Result};
use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};
#[repr(C)]
pub struct RawAccount {
    pub(crate) marker: u8,
    pub(crate) signer: u8,
    pub(crate) writable: u8,
    pub(crate) executable: u8,
    pub(crate) original_len: u32,
    pub(crate) key: Pubkey,
    pub(crate) owner: Pubkey,
    pub(crate) lamports: u64,
    pub(crate) data_len: u64,
}
#[repr(C)]
struct BorrowState {
    data: i32,
    lamports: i32,
}
#[derive(Clone, Copy)]
pub struct AccountData<'info> {
    raw: *mut RawAccount,
    life: PhantomData<&'info mut [u8]>,
}
#[derive(Clone, Copy)]
pub struct AccountLamports<'info> {
    raw: *mut RawAccount,
    life: PhantomData<&'info mut u64>,
}
fn borrow_error() -> std::cell::BorrowError {
    let cell = std::cell::RefCell::new(());
    let _held = cell.borrow_mut();
    cell.try_borrow().unwrap_err()
}
fn borrow_mut_error() -> std::cell::BorrowMutError {
    let cell = std::cell::RefCell::new(());
    let _held = cell.borrow();
    cell.try_borrow_mut().unwrap_err()
}
impl<'info> AccountData<'info> {
    #[inline(always)]
    pub fn try_borrow(&self) -> std::result::Result<DataRef<'info>, std::cell::BorrowError> {
        unsafe { AccountInfo::new(self.raw, false) }
            .try_borrow_data()
            .map_err(|_| borrow_error())
    }
    #[inline(always)]
    pub fn try_borrow_mut(
        &self,
    ) -> std::result::Result<DataRefMut<'info>, std::cell::BorrowMutError> {
        unsafe { AccountInfo::new(self.raw, false) }
            .try_borrow_mut_data()
            .map_err(|_| borrow_mut_error())
    }
    #[track_caller]
    #[inline(always)]
    pub fn borrow(&self) -> DataRef<'info> {
        match self.try_borrow() {
            Ok(value) => value,
            Err(error) => panic!("already mutably borrowed: {:?}", error),
        }
    }
    #[track_caller]
    #[inline(always)]
    pub fn borrow_mut(&self) -> DataRefMut<'info> {
        match self.try_borrow_mut() {
            Ok(value) => value,
            Err(error) => panic!("already borrowed: {:?}", error),
        }
    }
}
impl<'info> AccountLamports<'info> {
    #[inline(always)]
    pub fn try_borrow(&self) -> std::result::Result<LamportsRef<'info>, std::cell::BorrowError> {
        unsafe { AccountInfo::new(self.raw, false) }
            .try_borrow_lamports()
            .map_err(|_| borrow_error())
    }
    #[inline(always)]
    pub fn try_borrow_mut(
        &self,
    ) -> std::result::Result<LamportsRefMut<'info>, std::cell::BorrowMutError> {
        unsafe { AccountInfo::new(self.raw, false) }
            .try_borrow_mut_lamports()
            .map_err(|_| borrow_mut_error())
    }
    #[track_caller]
    #[inline(always)]
    pub fn borrow(&self) -> LamportsRef<'info> {
        match self.try_borrow() {
            Ok(value) => value,
            Err(error) => panic!("already mutably borrowed: {:?}", error),
        }
    }
    #[track_caller]
    #[inline(always)]
    pub fn borrow_mut(&self) -> LamportsRefMut<'info> {
        match self.try_borrow_mut() {
            Ok(value) => value,
            Err(error) => panic!("already borrowed: {:?}", error),
        }
    }
}
#[derive(Clone, Copy)]
#[repr(C)]
pub struct AccountInfo<'info> {
    pub key: &'info Pubkey,
    pub owner: &'info Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
    pub executable: bool,
    pub(crate) aliased: bool,
    pub data: AccountData<'info>,
    pub lamports: AccountLamports<'info>,
}
impl<'info> AccountInfo<'info> {
    #[inline(always)] pub(crate) fn __aliased(&self)->bool {self.aliased}
    #[inline(always)] pub fn sdk(&self)->&Self {self}
    #[inline(always)] pub fn sdk_exposed(&self)->bool {false}
    #[inline(always)] pub fn from_sdk(info:&Self)->Self {*info}
    #[inline(always)] pub(crate) unsafe fn from_sdk_borrowed(info:&'info Self)->Self {*info}
    #[inline(always)] pub fn key_ref(&self)->&'info Pubkey {self.key}
    #[inline(always)] pub fn owner_ref(&self)->&'info Pubkey {self.owner}
    #[inline(always)] pub fn is_signer(&self)->bool {self.is_signer}
    #[inline(always)] pub fn is_writable(&self)->bool {self.is_writable}
    #[inline(always)] pub fn executable(&self)->bool {self.executable}
    #[inline(always)] pub fn data_handle(&self)->AccountData<'info> {self.data}
    #[inline(always)] pub fn lamports_handle(&self)->AccountLamports<'info> {self.lamports}
    #[inline(always)] pub fn __is_raw(&self)->bool {true}
    #[inline(always)] pub unsafe fn __try_borrow_raw_data(&self)->std::result::Result<DataRef<'info>,solana_program::program_error::ProgramError>{self.try_borrow_data()}
    #[inline(always)] pub(crate) fn raw_borrow_check(&self,writable:bool)->std::result::Result<(),solana_program::program_error::ProgramError>{
        let pointer=self.borrow_state();
        let failed=if writable {
            // BorrowState is two contiguous i32 counters at an eight-byte
            // aligned loader slot. Either nonzero counter blocks a write.
            unsafe {*pointer.cast::<u64>() != 0}
        } else {
            let state=unsafe{&*pointer};
            state.data<0||state.lamports<0||state.data==i32::MAX||state.lamports==i32::MAX
        };
        if failed {Err(solana_program::program_error::ProgramError::AccountBorrowFailed)}else{Ok(())}
    }
    #[inline(always)]
    unsafe fn new(raw: *mut RawAccount, aliased: bool) -> Self {
        Self {
            key: &(*raw).key,
            owner: &(*raw).owner,
            is_signer: *std::ptr::addr_of!((*raw).signer).cast::<bool>(),
            is_writable: *std::ptr::addr_of!((*raw).writable).cast::<bool>(),
            executable: *std::ptr::addr_of!((*raw).executable).cast::<bool>(),
            aliased,
            data: AccountData {
                raw,
                life: PhantomData,
            },
            lamports: AccountLamports {
                raw,
                life: PhantomData,
            },
        }
    }
    #[inline(always)]
    pub fn __tare_info(&self) -> Self {
        *self
    }
    #[inline(always)]
    pub fn lamports(&self) -> u64 {
        **self.lamports.borrow()
    }
    #[inline(always)]
    pub fn data_len(&self) -> usize {
        self.data.borrow().len()
    }
    #[inline(always)]
    pub fn data_is_empty(&self) -> bool {
        self.data_len() == 0
    }
    #[inline(always)]
    fn borrow_state(&self) -> *mut BorrowState {
        unsafe {
            let raw = self.data.raw;
            let end = raw
                .cast::<u8>()
                .add(std::mem::size_of::<RawAccount>() + (*raw).original_len as usize + 10240);
            end.add(end.align_offset(8)).cast()
        }
    }
    #[inline(always)]
    pub(crate) fn data_is_borrowed(&self) -> bool {
        unsafe { (*self.borrow_state()).data != 0 }
    }
    #[inline(always)]
    pub(crate) fn lamports_are_borrowed(&self) -> bool {
        unsafe { (*self.borrow_state()).lamports != 0 }
    }
    #[inline(always)]
    pub fn try_borrow_data(
        &self,
    ) -> std::result::Result<DataRef<'info>, solana_program::program_error::ProgramError> {
        unsafe {
            let state = &mut (*self.borrow_state()).data;
            if *state < 0 || *state == i32::MAX {
                return Err(solana_program::program_error::ProgramError::AccountBorrowFailed);
            }
            *state += 1;
            Ok(DataRef {
                raw: self.data.raw,
                state,
                _life: PhantomData,
            })
        }
    }
    #[inline(always)]
    pub fn try_borrow_mut_data(
        &self,
    ) -> std::result::Result<DataRefMut<'info>, solana_program::program_error::ProgramError> {
        unsafe {
            let state = &mut (*self.borrow_state()).data;
            if *state != 0 {
                return Err(solana_program::program_error::ProgramError::AccountBorrowFailed);
            }
            *state = -1;
            Ok(DataRefMut {
                raw: self.data.raw,
                state,
                _life: PhantomData,
            })
        }
    }
    #[inline(always)]
    pub fn try_borrow_lamports(
        &self,
    ) -> std::result::Result<LamportsRef<'info>, solana_program::program_error::ProgramError> {
        unsafe {
            let state = &mut (*self.borrow_state()).lamports;
            if *state < 0 || *state == i32::MAX {
                return Err(solana_program::program_error::ProgramError::AccountBorrowFailed);
            }
            *state += 1;
            Ok(LamportsRef {
                value: std::ptr::addr_of_mut!((*self.data.raw).lamports).cast(),
                state,
                life: PhantomData,
            })
        }
    }
    #[inline(always)]
    pub fn try_borrow_mut_lamports(
        &self,
    ) -> std::result::Result<LamportsRefMut<'info>, solana_program::program_error::ProgramError>
    {
        unsafe {
            let state = &mut (*self.borrow_state()).lamports;
            if *state != 0 {
                return Err(solana_program::program_error::ProgramError::AccountBorrowFailed);
            }
            *state = -1;
            Ok(LamportsRefMut {
                value: std::ptr::addr_of_mut!((*self.data.raw).lamports).cast(),
                state,
                life: PhantomData,
            })
        }
    }
    #[inline(always)]
    pub(crate) fn set_lamports(&self, value: u64) {
        unsafe {
            (*self.data.raw).lamports = value;
        }
    }
    #[inline(always)]
    pub fn assign(&self, owner: &Pubkey) {
        unsafe { std::ptr::write_volatile(std::ptr::addr_of_mut!((*self.data.raw).owner), *owner) }
    }
    #[inline(always)] pub fn resize(&self,new_len:usize)->Result<()> {self.realloc(new_len,true)}
    #[inline(always)]
    pub fn realloc(&self, new_len: usize, zero: bool) -> Result<()> {
        unsafe {
            let raw = self.data.raw;
            let original = (*raw).original_len as usize;
            if new_len.saturating_sub(original) > 10240 {
                return Err(solana_program::program_error::ProgramError::InvalidRealloc.into());
            }
            if self.data_is_borrowed() {
                return Err(
                    solana_program::program_error::ProgramError::AccountBorrowFailed.into(),
                );
            }
            let old = (*raw).data_len as usize;
            if zero && new_len > old {
                std::ptr::write_bytes(raw.add(1).cast::<u8>().add(old), 0, new_len - old);
            }
            (*raw).data_len = new_len as u64;
            Ok(())
        }
    }
    #[inline(always)]
    pub fn __data_ptr(&self) -> *mut u8 {
        unsafe { self.data.raw.add(1).cast() }
    }
    #[inline(always)]
    pub(crate) fn raw(&self) -> *mut RawAccount {
        self.data.raw
    }
}
#[repr(transparent)]
pub struct LamportsValue(u64);
impl std::fmt::Debug for LamportsValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}
impl Deref for LamportsValue {
    type Target = u64;
    #[inline(always)]
    fn deref(&self) -> &u64 {
        &self.0
    }
}
impl DerefMut for LamportsValue {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut u64 {
        &mut self.0
    }
}
pub struct LamportsRef<'info> {
    value: *mut LamportsValue,
    state: *mut i32,
    life: PhantomData<&'info u64>,
}
impl Deref for LamportsRef<'_> {
    type Target = LamportsValue;
    #[inline(always)]
    fn deref(&self) -> &LamportsValue {
        unsafe { &*self.value }
    }
}
impl Drop for LamportsRef<'_> {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            *self.state -= 1;
        }
    }
}
impl std::fmt::Debug for LamportsRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&**self, f)
    }
}
pub struct LamportsRefMut<'info> {
    value: *mut LamportsValue,
    state: *mut i32,
    life: PhantomData<&'info mut u64>,
}
impl Deref for LamportsRefMut<'_> {
    type Target = LamportsValue;
    #[inline(always)]
    fn deref(&self) -> &LamportsValue {
        unsafe { &*self.value }
    }
}
impl DerefMut for LamportsRefMut<'_> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut LamportsValue {
        unsafe { &mut *self.value }
    }
}
impl Drop for LamportsRefMut<'_> {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            *self.state = 0;
        }
    }
}
impl std::fmt::Debug for LamportsRefMut<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&**self, f)
    }
}
pub struct DataRef<'info> {
    raw: *mut RawAccount,
    state: *mut i32,
    _life: PhantomData<&'info [u8]>,
}
impl Deref for DataRef<'_> {
    type Target = [u8];
    #[inline(always)]
    fn deref(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.raw.add(1).cast(), (*self.raw).data_len as usize) }
    }
}
impl std::fmt::Debug for DataRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&**self, f)
    }
}
impl Drop for DataRef<'_> {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            *self.state -= 1;
        }
    }
}
pub struct DataRefMut<'info> {
    raw: *mut RawAccount,
    state: *mut i32,
    _life: PhantomData<&'info mut [u8]>,
}
impl Deref for DataRefMut<'_> {
    type Target = [u8];
    #[inline(always)]
    fn deref(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.raw.add(1).cast(), (*self.raw).data_len as usize) }
    }
}
impl DerefMut for DataRefMut<'_> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(self.raw.add(1).cast(), (*self.raw).data_len as usize)
        }
    }
}
impl std::fmt::Debug for DataRefMut<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&**self, f)
    }
}
impl Drop for DataRefMut<'_> {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            *self.state = 0;
        }
    }
}
// Defer account iteration state until dispatch has selected an instruction.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct AccountInput<'info> {
    base: *mut u8,
    has_duplicates: bool,
    _life: PhantomData<&'info mut [u8]>,
}
impl<'info> AccountInput<'info> {
    #[inline(always)]
    pub fn cursor_mode<const RAW:bool>(self)->AccountCursor<'info>{
        if RAW {AccountCursor{base:self.base,cursor:self.base,index:0,count:unsafe{*self.base.sub(8).cast::<u64>()as usize},has_duplicates:self.has_duplicates,_life:PhantomData}}else{self.cursor()}
    }
    #[inline(always)]
    pub fn cursor(self) -> AccountCursor<'info> {
        if self.base as usize&1!=0 {let h=unsafe{&*((self.base as usize&!1)as*const[usize;2])};unsafe{AccountCursor::from_sdk_header(h)}}else{AccountCursor{base:self.base,cursor:self.base,index:0,count:unsafe{*self.base.sub(8).cast::<u64>()as usize},has_duplicates:self.has_duplicates,_life:PhantomData}}
    }
}
#[derive(Clone, Copy)]
pub struct AccountCursor<'info> {
    base: *mut u8,
    cursor: *mut u8,
    index: usize,
    count: usize,
    has_duplicates: bool,
    _life: PhantomData<&'info mut [u8]>,
}
impl<'info> AccountCursor<'info> {
    #[inline(always)]
    pub fn into_input(self) -> AccountInput<'info> {
        AccountInput {base:self.base,has_duplicates:self.has_duplicates,_life:PhantomData}
    }
    #[inline(always)]
    pub unsafe fn from_input(input: *mut u8) -> (Self, &'info [u8], &'info Pubkey) {
        let count = *(input as *const u64) as usize;
        let base = input.add(8);
        let mut end = base;
        let mut has_duplicates = false;
        for _ in 0..count {
            if *end == 255 {
                (*(end as *mut RawAccount)).original_len =
                    (*(end as *const RawAccount)).data_len as u32;
                let next = advance(end);
                *next.sub(8).cast::<isize>() = 0;
                end = next;
            } else {
                has_duplicates = true;
                end = advance(end);
            }
        }
        let len = *(end as *const u64) as usize;
        let data = std::slice::from_raw_parts(end.add(8), len);
        let program_id = &*(end.add(8 + len) as *const Pubkey);
        (
            Self {
                base,
                cursor: base,
                index: 0,
                count,
                has_duplicates,
                _life: PhantomData,
            },
            data,
            program_id,
        )
    }
    pub unsafe fn from_sdk_header(header:&[usize;2])->Self {let base=(header.as_ptr() as usize|1)as*mut u8;Self{base,cursor:header[1]as*mut u8,index:0,count:header[0],has_duplicates:true,_life:PhantomData}}
    pub fn from_sdk(accounts:&'info[AccountInfo<'info>])->Self {let header=Box::leak(Box::new([accounts.len(),accounts.as_ptr()as usize]));unsafe{Self::from_sdk_header(header)}}
    #[inline(always)] pub fn next_mode<const RAW:bool>(&mut self)->Result<AccountInfo<'info>> {
        if !RAW {return self.next();}
        if self.index==self.count{return Err(ErrorCode::AccountNotEnoughKeys.into());}
        unsafe {let mut raw=self.cursor;self.cursor=advance(raw);self.index+=1;if *raw!=255{let target=*raw as usize;raw=self.base;for _ in 0..target{raw=advance(raw);}}Ok(AccountInfo::new(raw.cast(),self.has_duplicates))}
    }
    #[inline(always)] pub fn next_optional_mode<const RAW:bool>(&mut self,p:&Pubkey)->Result<Option<AccountInfo<'info>>>{
        if self.index==self.count{return if cfg!(feature="allow-missing-optionals"){Ok(None)}else{Err(ErrorCode::AccountNotEnoughKeys.into())};}
        let info=self.next_mode::<RAW>()?;if info.key==p{Ok(None)}else{Ok(Some(info))}
    }
    #[inline(always)] pub fn typed_prefix_is_unique_mode<const RAW:bool>(&self,n:usize)->bool{if !RAW{return self.typed_prefix_is_unique(n);}if !self.has_duplicates{return true;}let mut cursor=self.cursor;for _ in 0..n.min(self.count-self.index){unsafe{if *cursor!=255{return false;}cursor=advance(cursor);}}true}
    #[inline(always)]
    pub fn next(&mut self) -> Result<AccountInfo<'info>> {
        if self.index == self.count {return Err(ErrorCode::AccountNotEnoughKeys.into());}
        Ok(unsafe {self.next_unchecked()})
    }
    #[inline(always)]
    unsafe fn next_unchecked(&mut self)->AccountInfo<'info> {
        if self.base as usize&1!=0 {let info=*self.cursor.cast::<AccountInfo<'info>>();self.cursor=self.cursor.add(std::mem::size_of::<AccountInfo>());self.index+=1;return info;}
        let mut raw=self.cursor;
        self.cursor=advance(self.cursor);self.index+=1;
        if *raw!=255 {
            let target=*raw as usize;raw=self.base;
            for _ in 0..target {raw=advance(raw);}
        }
        AccountInfo::new(raw.cast(),self.has_duplicates)
    }
    #[inline(always)]
    pub fn typed_prefix_is_unique(&self,count:usize)->bool {
        if self.base as usize&1!=0{return false;}
        if !self.has_duplicates {return true;}
        let mut cursor=self.cursor;
        for _ in 0..count.min(self.count-self.index) {
            unsafe {if *cursor!=255 {return false;}cursor=advance(cursor);}
        }
        true
    }
    #[inline(always)]
    pub fn next_optional(&mut self, program_id: &Pubkey) -> Result<Option<AccountInfo<'info>>> {
        if self.index == self.count {
            return if cfg!(feature = "allow-missing-optionals") {
                Ok(None)
            } else {
                Err(ErrorCode::AccountNotEnoughKeys.into())
            };
        }
        let info = self.next()?;
        if info.key == program_id {
            Ok(None)
        } else {
            Ok(Some(info))
        }
    }
    #[inline(always)]
    pub fn remaining(&self) -> RemainingAccounts<'info> {
        RemainingAccounts(*self)
    }
    pub fn remaining_storage(&self)->RemainingStorage<'info> {
        match self.count-self.index {
            0=>RemainingStorage::Empty,
            1=>{let mut cursor=*self;RemainingStorage::One([unsafe {cursor.next_unchecked()}])},
            _=>RemainingStorage::Many(self.remaining().iter().collect()),
        }
    }
}
pub enum RemainingStorage<'info> {
    Empty,
    One([AccountInfo<'info>;1]),
    Many(Vec<AccountInfo<'info>>),
}
impl<'info> RemainingStorage<'info> {
    pub fn as_slice(&self)->&[AccountInfo<'info>] {
        match self {Self::Empty=>&[],Self::One(info)=>info,Self::Many(infos)=>infos}
    }
}
#[inline(always)]
unsafe fn advance(raw: *mut u8) -> *mut u8 {
    if *raw != 255 {
        return raw.add(8);
    }
    let len = (*(raw as *const RawAccount)).original_len as usize;
    let data_end = raw.add(std::mem::size_of::<RawAccount>() + len + 10240);
    data_end.add(data_end.align_offset(8) + 8)
}
#[derive(Clone, Copy)]
pub struct RemainingAccounts<'info>(AccountCursor<'info>);
impl<'info> RemainingAccounts<'info> {
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.0.count - self.0.index
    }
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    #[inline(always)]
    pub fn iter(&self) -> RemainingIter<'info> {
        RemainingIter(self.0)
    }
}
pub struct RemainingIter<'info>(AccountCursor<'info>);
impl<'info> Iterator for RemainingIter<'info> {
    type Item = AccountInfo<'info>;
    fn size_hint(&self)->(usize,Option<usize>){let len=self.0.count-self.0.index;(len,Some(len))}
    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.0.index == self.0.count {
            None
        } else {
            Some(unsafe {self.0.next_unchecked()})
        }
    }
}

impl ExactSizeIterator for RemainingIter<'_> {
    fn len(&self)->usize {self.0.count-self.0.index}
}

impl std::fmt::Debug for AccountInfo<'_> {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {
        let mut value=f.debug_struct("AccountInfo");
        value.field("key",&self.key).field("owner",&self.owner)
            .field("is_signer",&self.is_signer).field("is_writable",&self.is_writable)
            .field("executable",&self.executable)
            // The loader serializes the deprecated epoch as u64::MAX. Tare
            // reuses that ignored input word for borrow state after decoding.
            .field("rent_epoch",&u64::MAX).field("lamports",&self.lamports())
            .field("data.len",&self.data_len());
        solana_program::account_info::debug_account_data::debug_account_data(&self.data.borrow(),&mut value);
        value.finish_non_exhaustive()
    }
}
pub fn next_account_info<'a,'info,I>(iter:&mut I)->std::result::Result<&'a AccountInfo<'info>,solana_program::program_error::ProgramError>
where I:Iterator<Item=&'a AccountInfo<'info>>, 'info:'a {
    iter.next().ok_or(solana_program::program_error::ProgramError::NotEnoughAccountKeys)
}

pub type AccountView<'info> = AccountInfo<'info>;

impl<'info> AsRef<AccountInfo<'info>> for AccountInfo<'info> {fn as_ref(&self)->&Self{self}}
