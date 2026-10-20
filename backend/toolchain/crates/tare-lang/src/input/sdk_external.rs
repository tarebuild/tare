//! Views over the upgradeable-loader input ABI; account descriptors are built on demand.
use crate::{error::ErrorCode, Key, Pubkey, Result, ToAccountInfo};
use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};
#[repr(C)]
pub struct RawAccount {
    marker: u8,
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
#[repr(transparent)]
struct SdkCache<'info> {
    info: crate::AccountInfo<'info>,
}
// Two low bits are available in both input representations. The SDK object
// itself remains unchanged; this is an internal pointer encoding only.
const _: () = assert!(std::mem::align_of::<RawAccount>() >= 4);
const _: () = assert!(std::mem::align_of::<SdkCache<'static>>() >= 4);
const SDK_CACHE_TAG: usize=1;
// Raw RefCell counters occupy 00/01/11 in the top two bits: a mutable
// lamport borrow is -1, never i32::MIN. The 10 tag marks a cache pointer.
const SDK_CACHE_FLAG: usize=1usize<<63;
const SDK_CACHE_MASK: usize=3usize<<62;

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
        unsafe { AccountView{raw:self.raw,life:PhantomData} }
            .try_borrow_data()
            .map_err(|_| borrow_error())
    }
    #[inline(always)]
    pub fn try_borrow_mut(
        &self,
    ) -> std::result::Result<DataRefMut<'info>, std::cell::BorrowMutError> {
        unsafe { AccountView{raw:self.raw,life:PhantomData} }
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
        unsafe { AccountView{raw:self.raw,life:PhantomData} }
            .try_borrow_lamports()
            .map_err(|_| borrow_error())
    }
    #[inline(always)]
    pub fn try_borrow_mut(
        &self,
    ) -> std::result::Result<LamportsRefMut<'info>, std::cell::BorrowMutError> {
        unsafe { AccountView{raw:self.raw,life:PhantomData} }
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
pub struct AccountView<'info> {
    raw:*mut RawAccount,
    life:PhantomData<&'info mut [u8]>,
}
impl<'info> AccountView<'info> {
    #[inline(always)]
    fn external_cache(&self)->bool {true}
    #[inline(always)]
    fn cache_slot(&self)->*mut usize {
        unsafe {let raw=self.raw();let end=raw.cast::<u8>().add(std::mem::size_of::<RawAccount>()+(*raw).original_len as usize+10240);end.add(end.align_offset(8)).cast()}
    }
    #[inline(always)]
    fn sdk_cache(&self)->Option<&'info SdkCache<'info>> {unsafe{Some(&*(self.raw as *const SdkCache<'info>))}}
    #[inline(always)]pub fn sdk_exposed(&self)->bool{true}
    #[inline(always)]pub fn sdk(&self)->&'info crate::AccountInfo<'info>{&self.sdk_cache().unwrap().info}
    pub(crate) unsafe fn from_sdk_borrowed(info:&'info crate::AccountInfo<'info>)->Self {
        Self{raw:info as *const crate::AccountInfo<'info> as *mut RawAccount,life:PhantomData}
    }
    pub fn from_sdk(info:&crate::AccountInfo<'info>)->Self {
        let cache=Box::into_raw(Box::new(SdkCache{info:info.clone()}));
        Self{raw:cache.cast(),life:PhantomData}
    }
    #[inline(never)] unsafe fn new(raw:*mut RawAccount,_aliased:bool)->Self {
        let end=raw.cast::<u8>().add(std::mem::size_of::<RawAccount>()+(*raw).original_len as usize+10240);
        let slot=end.add(end.align_offset(8)).cast::<usize>();
        let tagged=*slot;
        let pointer=if tagged&SDK_CACHE_MASK==SDK_CACHE_FLAG {(tagged&!SDK_CACHE_FLAG)as *mut SdkCache<'info>}else{
            let pointer=Box::into_raw(Box::<SdkCache<'info>>::new_uninit()).cast::<SdkCache<'info>>();
            let info=std::ptr::addr_of_mut!((*pointer).info);
            std::ptr::addr_of_mut!((*info).key).write(&(*raw).key);
            std::ptr::addr_of_mut!((*info).owner).write(&(*raw).owner);
            std::ptr::addr_of_mut!((*info).is_signer).write((*raw).signer!=0);
            std::ptr::addr_of_mut!((*info).is_writable).write((*raw).writable!=0);
            std::ptr::addr_of_mut!((*info).executable).write((*raw).executable!=0);
            std::ptr::addr_of_mut!((*info).rent_epoch).write(u64::MAX);
            std::ptr::addr_of_mut!((*info).lamports).write(std::rc::Rc::new(std::cell::RefCell::new(&mut(*raw).lamports)));
            std::ptr::addr_of_mut!((*info).data).write(std::rc::Rc::new(std::cell::RefCell::new(std::slice::from_raw_parts_mut(raw.add(1).cast(),(*raw).data_len as usize))));
            *slot=pointer as usize|SDK_CACHE_FLAG;pointer
        };
        Self{raw:pointer.cast(),life:PhantomData}
    }
    #[inline(always)] pub fn key_ref(&self)->&'info Pubkey {unsafe{if self.external_cache(){self.sdk_cache().unwrap().info.key}else{&(*self.raw()).key}}}
    #[inline(always)] pub fn owner_ref(&self)->&'info Pubkey {unsafe{if self.external_cache(){self.sdk_cache().unwrap().info.owner}else{&(*self.raw()).owner}}}
    #[inline(always)] pub fn is_signer(&self)->bool {unsafe{if self.external_cache(){self.sdk_cache().unwrap().info.is_signer}else{(*self.raw()).signer!=0}}}
    #[inline(always)] pub fn is_writable(&self)->bool {unsafe{if self.external_cache(){self.sdk_cache().unwrap().info.is_writable}else{(*self.raw()).writable!=0}}}
    #[inline(always)] pub fn executable(&self)->bool {unsafe{if self.external_cache(){self.sdk_cache().unwrap().info.executable}else{(*self.raw()).executable!=0}}}
    #[inline(always)] pub fn data_handle(&self)->AccountData<'info>{AccountData{raw:self.raw,life:PhantomData}}
    #[inline(always)] pub fn lamports_handle(&self)->AccountLamports<'info>{AccountLamports{raw:self.raw,life:PhantomData}}
    #[inline(always)]
    pub fn __tare_info(&self) -> Self {
        *self
    }
    #[inline(always)]
    pub fn lamports(&self) -> u64 {
        if let Some(cache)=self.sdk_cache(){return cache.info.lamports();}
        unsafe {
            if (*self.borrow_state()).lamports<0 {panic!("already mutably borrowed: BorrowError");}
            (*self.raw()).lamports
        }
    }
    #[inline(always)]
    pub fn data_len(&self) -> usize {
        if let Some(cache)=self.sdk_cache(){return cache.info.data_len();}
        unsafe {
            if (*self.borrow_state()).data<0 {panic!("already mutably borrowed: BorrowError");}
            (*self.raw()).data_len as usize
        }
    }
    #[inline(always)]
    pub fn data_is_empty(&self) -> bool {
        self.data_len() == 0
    }
    #[inline(always)]
    fn borrow_state(&self) -> *mut BorrowState {
        unsafe {
            let raw = self.raw();
            let end = raw
                .cast::<u8>()
                .add(std::mem::size_of::<RawAccount>() + (*raw).original_len as usize + 10240);
            end.add(end.align_offset(8)).cast()
        }
    }
    #[inline(always)] pub(crate) fn raw_borrow_check(&self,writable:bool)->std::result::Result<(),solana_program::program_error::ProgramError> {
        unsafe {let state=&*self.borrow_state();if (writable && (state.data!=0||state.lamports!=0))||(!writable && (state.data<0||state.lamports<0||state.data==i32::MAX||state.lamports==i32::MAX)){Err(solana_program::program_error::ProgramError::AccountBorrowFailed)}else{Ok(())}}
    }
    #[inline(always)]
    pub(crate) fn data_is_borrowed(&self) -> bool {
        if let Some(cache)=self.sdk_cache(){return cache.info.try_borrow_mut_data().is_err();}
        unsafe { (*self.borrow_state()).data != 0 }
    }
    #[inline(always)]
    pub(crate) fn lamports_are_borrowed(&self) -> bool {
        if let Some(cache)=self.sdk_cache(){return cache.info.try_borrow_mut_lamports().is_err();}
        unsafe { (*self.borrow_state()).lamports != 0 }
    }
    #[inline(always)] pub unsafe fn __try_borrow_raw_data(&self)->std::result::Result<DataRef<'info>,solana_program::program_error::ProgramError> {
        let state=&mut (*self.borrow_state()).data;
        if *state<0 || *state==i32::MAX {return Err(solana_program::program_error::ProgramError::AccountBorrowFailed);}
        *state+=1;
        Ok(DataRef::Raw{raw:self.raw(),state,_life:PhantomData})
    }
    #[inline(always)] pub fn __is_raw(&self)->bool {!self.external_cache() && !self.sdk_exposed()}
    #[inline(always)]
    pub fn try_borrow_data(
        &self,
    ) -> std::result::Result<DataRef<'info>, solana_program::program_error::ProgramError> {
        if let Some(cache)=self.sdk_cache() {
            return Ok(DataRef::Sdk(cache.info.try_borrow_data()?));
        }
        unsafe {
            let state = &mut (*self.borrow_state()).data;
            if *state < 0 || *state == i32::MAX {
                return Err(solana_program::program_error::ProgramError::AccountBorrowFailed);
            }
            *state += 1;
            Ok(DataRef::Raw {raw:self.raw(),state,_life:PhantomData})
        }
    }
    #[inline(always)]
    pub fn try_borrow_mut_data(
        &self,
    ) -> std::result::Result<DataRefMut<'info>, solana_program::program_error::ProgramError> {
        if let Some(cache)=self.sdk_cache() {
            return Ok(DataRefMut::Sdk(cache.info.try_borrow_mut_data()?));
        }
        unsafe {
            let state = &mut (*self.borrow_state()).data;
            if *state != 0 {
                return Err(solana_program::program_error::ProgramError::AccountBorrowFailed);
            }
            *state = -1;
            Ok(DataRefMut::Raw {raw:self.raw(),state,_life:PhantomData})
        }
    }
    #[inline(always)]
    pub fn try_borrow_lamports(
        &self,
    ) -> std::result::Result<LamportsRef<'info>, solana_program::program_error::ProgramError> {
        if let Some(cache)=self.sdk_cache() {
            return Ok(LamportsRef::Sdk(cache.info.try_borrow_lamports()?));
        }
        unsafe {
            let state = &mut (*self.borrow_state()).lamports;
            if *state < 0 || *state == i32::MAX {
                return Err(solana_program::program_error::ProgramError::AccountBorrowFailed);
            }
            *state += 1;
            Ok(LamportsRef::Raw {value:std::ptr::addr_of_mut!((*self.raw()).lamports).cast(),state,life:PhantomData})
        }
    }
    #[inline(always)]
    pub fn try_borrow_mut_lamports(
        &self,
    ) -> std::result::Result<LamportsRefMut<'info>, solana_program::program_error::ProgramError>
    {
        if let Some(cache)=self.sdk_cache() {
            return Ok(LamportsRefMut::Sdk(cache.info.try_borrow_mut_lamports()?));
        }
        unsafe {
            let state = &mut (*self.borrow_state()).lamports;
            if *state != 0 {
                return Err(solana_program::program_error::ProgramError::AccountBorrowFailed);
            }
            *state = -1;
            Ok(LamportsRefMut::Raw {value:std::ptr::addr_of_mut!((*self.raw()).lamports).cast(),state,life:PhantomData})
        }
    }
    #[inline(always)]
    pub(crate) fn set_lamports(&self, value: u64) {
        if let Some(cache)=self.sdk_cache(){**cache.info.lamports.borrow_mut()=value;return;}
        unsafe {
            (*self.raw()).lamports = value;
        }
    }
    #[inline(always)]
    pub fn assign(&self, owner: &Pubkey) {
        if let Some(cache)=self.sdk_cache(){cache.info.assign(owner);return;}
        unsafe { std::ptr::write_volatile(std::ptr::addr_of_mut!((*self.raw()).owner), *owner) }
    }
    #[inline(always)]
    pub fn realloc(&self, new_len: usize, zero: bool) -> Result<()> {
        if let Some(cache)=self.sdk_cache(){return cache.info.realloc(new_len,zero).map_err(Into::into);}
        unsafe {
            let raw = self.raw();
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
        if self.external_cache(){return self.sdk_cache().unwrap().info.data.borrow_mut().as_mut_ptr();}
        unsafe { self.raw().add(1).cast() }
    }
    #[inline(always)]
    pub(crate) fn raw(&self) -> *mut RawAccount {
        self.raw
    }
    #[inline(always)] pub(crate) fn __aliased(&self)->bool {true}
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
pub enum LamportsRef<'info> {
    Sdk(std::cell::Ref<'info,&'info mut u64>),
    Raw{value:*mut LamportsValue,state:*mut i32,life:PhantomData<&'info u64>},
}
impl Deref for LamportsRef<'_>{type Target=LamportsValue;#[inline(always)] fn deref(&self)->&LamportsValue {match self {Self::Sdk(value)=>unsafe{&*((&***value as *const u64).cast())},Self::Raw{value,..}=>unsafe{&**value}}}}
impl Drop for LamportsRef<'_>{#[inline(always)]fn drop(&mut self){if let Self::Raw{state,..}=self {unsafe{**state-=1;}}}}
impl std::fmt::Debug for LamportsRef<'_>{fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{std::fmt::Debug::fmt(&**self,f)}}
pub enum LamportsRefMut<'info> {
    Sdk(std::cell::RefMut<'info,&'info mut u64>),
    Raw{value:*mut LamportsValue,state:*mut i32,life:PhantomData<&'info mut u64>},
}
impl Deref for LamportsRefMut<'_>{type Target=LamportsValue;#[inline(always)] fn deref(&self)->&LamportsValue {match self {Self::Sdk(value)=>unsafe{&*((&***value as *const u64).cast())},Self::Raw{value,..}=>unsafe{&**value}}}}
impl DerefMut for LamportsRefMut<'_>{#[inline(always)] fn deref_mut(&mut self)->&mut LamportsValue {match self {Self::Sdk(value)=>unsafe{&mut *((&mut ***value as *mut u64).cast())},Self::Raw{value,..}=>unsafe{&mut **value}}}}
impl Drop for LamportsRefMut<'_>{#[inline(always)]fn drop(&mut self){if let Self::Raw{state,..}=self {unsafe{**state=0;}}}}
impl std::fmt::Debug for LamportsRefMut<'_>{fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{std::fmt::Debug::fmt(&**self,f)}}
impl Key for AccountView<'_> {
    #[inline(always)]
    fn key(&self) -> Pubkey {
        *self.key_ref()
    }
}
impl<'info> ToAccountInfo<'info> for AccountView<'info> {
    #[inline(always)]
    fn to_account_info(&self) -> crate::AccountInfo<'info> {self.sdk().clone()}
}
impl<'info> crate::FromAccountInfo<'info> for AccountView<'info> {
    #[inline(always)]
    fn try_from_info(info: Self) -> Result<Self> {
        Ok(info)
    }
}
pub enum DataRef<'info> {
    Sdk(std::cell::Ref<'info,&'info mut [u8]>),
    Raw{raw:*mut RawAccount,state:*mut i32,_life:PhantomData<&'info [u8]>},
}
impl Deref for DataRef<'_>{type Target=[u8];#[inline(always)]fn deref(&self)->&[u8]{match self{Self::Sdk(value)=>&**value,Self::Raw{raw,..}=>unsafe{std::slice::from_raw_parts(raw.add(1).cast(),(**raw).data_len as usize)}}}}
impl Drop for DataRef<'_>{#[inline(always)]fn drop(&mut self){if let Self::Raw{state,..}=self{unsafe{**state-=1;}}}}
impl std::fmt::Debug for DataRef<'_>{fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{std::fmt::Debug::fmt(&**self,f)}}
pub enum DataRefMut<'info> {
    Sdk(std::cell::RefMut<'info,&'info mut [u8]>),
    Raw{raw:*mut RawAccount,state:*mut i32,_life:PhantomData<&'info mut [u8]>},
}
impl Deref for DataRefMut<'_>{type Target=[u8];#[inline(always)]fn deref(&self)->&[u8]{match self{Self::Sdk(value)=>&**value,Self::Raw{raw,..}=>unsafe{std::slice::from_raw_parts(raw.add(1).cast(),(**raw).data_len as usize)}}}}
impl DerefMut for DataRefMut<'_>{#[inline(always)]fn deref_mut(&mut self)->&mut[u8]{match self{Self::Sdk(value)=>&mut**value,Self::Raw{raw,..}=>unsafe{std::slice::from_raw_parts_mut(raw.add(1).cast(),(**raw).data_len as usize)}}}}
impl Drop for DataRefMut<'_>{#[inline(always)]fn drop(&mut self){if let Self::Raw{state,..}=self{unsafe{**state=0;}}}}
impl std::fmt::Debug for DataRefMut<'_>{fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{std::fmt::Debug::fmt(&**self,f)}}
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
        AccountCursor {base:self.base,cursor:unsafe {if self.base as usize&1!=0 {*((self.base as usize&!1) as *const usize).add(1) as *mut u8}else{self.base}},index:0,count:unsafe {if self.base as usize&1!=0 {*((self.base as usize&!1) as *const usize)}else{*self.base.sub(8).cast::<u64>() as usize}},has_duplicates:self.has_duplicates,_life:PhantomData}
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
    pub unsafe fn from_sdk_header(header:&[usize;2])->Self {
        let base=(header.as_ptr() as usize|1) as *mut u8;
        Self{base,cursor:header[1] as *mut u8,index:0,count:header[0],has_duplicates:true,_life:PhantomData}
    }
    pub fn from_sdk(accounts:&'info [crate::AccountInfo<'info>])->Self {
        let header=Box::into_raw(Box::new([accounts.len(),accounts.as_ptr() as usize]));
        let base=(header as usize|1) as *mut u8;
        Self{base,cursor:accounts.as_ptr() as *mut u8,index:0,count:accounts.len(),has_duplicates:true,_life:PhantomData}
    }
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
    #[inline(always)] pub fn typed_prefix_is_unique_mode<const RAW:bool>(&self,count:usize)->bool {
        if RAW {
            if !self.has_duplicates{return true;}let mut cursor=self.cursor;
            for _ in 0..count.min(self.count-self.index){unsafe{if *cursor!=255{return false;}cursor=advance(cursor);}}
            true
        }else{self.typed_prefix_is_unique(count)}
    }
    #[inline(always)] pub fn next_mode<const RAW:bool>(&mut self)->Result<AccountView<'info>> {
        if self.index==self.count{return Err(ErrorCode::AccountNotEnoughKeys.into());}
        if RAW {
            unsafe {let mut raw=self.cursor;self.cursor=advance(raw);self.index+=1;
                if *raw!=255 {let target=*raw as usize;raw=self.base;for _ in 0..target{raw=advance(raw);}}
                Ok(AccountView::new(raw.cast(),self.has_duplicates))}
        } else {self.next()}
    }
    #[inline(always)] pub fn next_optional_mode<const RAW:bool>(&mut self,program_id:&Pubkey)->Result<Option<AccountView<'info>>> {
        if self.index==self.count{return if cfg!(feature="allow-missing-optionals"){Ok(None)}else{Err(ErrorCode::AccountNotEnoughKeys.into())};}
        if RAW || self.base as usize&1==0 {
            unsafe{
                let mut raw=self.cursor;self.cursor=advance(raw);self.index+=1;
                if *raw!=255 {let target=*raw as usize;raw=self.base;for _ in 0..target{raw=advance(raw);}}
                if &(*(raw as *const RawAccount)).key==program_id {return Ok(None);}
                return Ok(Some(AccountView::new(raw.cast(),self.has_duplicates)));
            }
        }
        let info=self.next_mode::<RAW>()?;if info.key_ref()==program_id {Ok(None)}else{Ok(Some(info))}
    }
    #[inline(always)]
    pub fn next(&mut self) -> Result<AccountView<'info>> {
        if self.index == self.count {return Err(ErrorCode::AccountNotEnoughKeys.into());}
        Ok(unsafe {self.next_unchecked()})
    }
    #[inline(always)]
    unsafe fn next_unchecked(&mut self)->AccountView<'info> {
        if self.base as usize&1!=0 {
            let info=AccountView::from_sdk_borrowed(&*self.cursor.cast::<crate::AccountInfo<'info>>());
            self.cursor=self.cursor.add(std::mem::size_of::<crate::AccountInfo>());self.index+=1;return info;
        }
        let mut raw=self.cursor;
        self.cursor=advance(self.cursor);self.index+=1;
        if *raw!=255 {
            let target=*raw as usize;raw=self.base;
            for _ in 0..target {raw=advance(raw);}
        }
        AccountView::new(raw.cast(),self.has_duplicates)
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
    pub fn next_optional(&mut self, program_id: &Pubkey) -> Result<Option<AccountView<'info>>> {
        if self.index == self.count {
            return if cfg!(feature = "allow-missing-optionals") {
                Ok(None)
            } else {
                Err(ErrorCode::AccountNotEnoughKeys.into())
            };
        }
        let info = self.next()?;
        if info.key_ref() == program_id {
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
            1=>{let mut cursor=*self;RemainingStorage::One([unsafe {cursor.next_unchecked()}.sdk().clone()])},
            _=>RemainingStorage::Many(self.remaining().iter().map(|info|info.sdk().clone()).collect()),
        }
    }
}
pub enum RemainingStorage<'info> {
    Empty,
    One([crate::AccountInfo<'info>;1]),
    Many(Vec<crate::AccountInfo<'info>>),
}
impl<'info> RemainingStorage<'info> {
    pub fn as_slice(&self)->&[crate::AccountInfo<'info>] {
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
    type Item = AccountView<'info>;
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

impl std::fmt::Debug for AccountView<'_> {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {
        let mut value=f.debug_struct("AccountInfo");
        value.field("key",&self.key_ref()).field("owner",&self.owner_ref())
            .field("is_signer",&self.is_signer()).field("is_writable",&self.is_writable())
            .field("executable",&self.executable())
            // The loader serializes the deprecated epoch as u64::MAX. Tare
            // reuses that ignored input word for borrow state after decoding.
            .field("rent_epoch",&u64::MAX).field("lamports",&self.lamports())
            .field("data.len",&self.data_len());
        solana_program::account_info::debug_account_data::debug_account_data(&self.data_handle().borrow(),&mut value);
        value.finish_non_exhaustive()
    }
}
pub fn next_account_info<'a,'info,I>(iter:&mut I)->std::result::Result<&'a AccountView<'info>,solana_program::program_error::ProgramError>
where I:Iterator<Item=&'a AccountView<'info>>, 'info:'a {
    iter.next().ok_or(solana_program::program_error::ProgramError::NotEnoughAccountKeys)
}
