//! Anchor 0.31 built-in IDL account instructions over borrowed runtime accounts.
//! Semantics follow Anchor v0.31.1 lang/syn/src/codegen/program/idl.rs (Apache-2.0).
use crate::{
    error::{Error, ErrorCode, Source},
    AccountCursor, AccountView, AnchorDeserialize, AnchorSerialize, Pubkey, Result,
};
use solana_program::{rent::Rent, sysvar::Sysvar};

pub const IDL_IX_TAG: u64 = 0x0a69e9a778bcf440;
pub const IDL_IX_TAG_LE: &[u8] = &IDL_IX_TAG.to_le_bytes();
pub const ERASED_AUTHORITY: Pubkey = Pubkey::new_from_array([0; 32]);
const HEADER_LEN: usize = 44;
#[cfg(target_os = "solana")]
extern "C" {
    fn abort() -> !;
    fn sol_panic_(file:*const u8,len:u64,line:u64,column:u64)->!;
}
const DISCRIMINATOR: &[u8] = &[24, 70, 98, 191, 58, 144, 123, 158];

#[derive(AnchorSerialize, AnchorDeserialize)]
pub enum IdlInstruction {
    Create { data_len: u64 },
    CreateBuffer,
    Write { data: Vec<u8> },
    SetBuffer,
    SetAuthority { new_authority: Pubkey },
    Close,
    Resize { data_len: u64 },
}
#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct IdlAccount {
    pub authority: Pubkey,
    pub data_len: u32,
}
impl IdlAccount {
    pub fn address(program_id: &Pubkey) -> Pubkey {
        let base = Pubkey::find_program_address(&[], program_id).0;
        Pubkey::create_with_seed(&base, Self::seed(), program_id).expect("Seed is always valid")
    }
    pub fn seed() -> &'static str {
        "anchor:idl"
    }
}
impl crate::Discriminator for IdlAccount {
    const DISCRIMINATOR: &'static [u8] = DISCRIMINATOR;
}
impl crate::AccountSerialize for IdlAccount {
    fn try_serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<()> {
        writer
            .write_all(DISCRIMINATOR)
            .map_err(|_| ErrorCode::AccountDidNotSerialize)?;
        AnchorSerialize::serialize(self, writer)
            .map_err(|_| ErrorCode::AccountDidNotSerialize.into())
    }
}
impl crate::AccountDeserialize for IdlAccount {
    fn try_deserialize(buf: &mut &[u8]) -> Result<Self> {
        if buf.len() < 8 {
            return Err(ErrorCode::AccountDiscriminatorNotFound.into());
        }
        if &buf[..8] != DISCRIMINATOR {
            return Err(Error::from(ErrorCode::AccountDiscriminatorMismatch)
                .__tare_with_account_name_static("IdlAccount"));
        }
        Self::try_deserialize_unchecked(buf)
    }
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self> {
        let mut body = &buf[8..];
        Self::deserialize(&mut body).map_err(|_| ErrorCode::AccountDidNotDeserialize.into())
    }
}

struct State<'info> {
    info: AccountView<'info>,
    authority: Pubkey,
    len: u32,
}
impl<'info> State<'info> {
    fn load(
        info: AccountView<'info>,
        program: &Pubkey,
        name: &'static str,
        checked: bool,
    ) -> Result<Self> {
        if info.owner_ref() == &solana_program::system_program::ID && info.lamports() == 0 {
            return Err(Error::from(ErrorCode::AccountNotInitialized).__tare_with_account_name_static(name));
        }
        if info.owner_ref() != program {
            return Err(Error::from(ErrorCode::AccountOwnedByWrongProgram)
                .__tare_with_account_name_static(name)
                .with_pubkeys((*info.owner_ref(), *program)));
        }
        let bytes = info
            .try_borrow_data()
            .map_err(|e| Error::from(e).__tare_with_account_name_static(name))?;
        if checked {
            if bytes.len() < 8 {
                return Err(
                    Error::from(ErrorCode::AccountDiscriminatorNotFound).__tare_with_account_name_static(name)
                );
            }
            if &bytes[..8] != DISCRIMINATOR {
                return Err(
                    Error::from(ErrorCode::AccountDiscriminatorMismatch).__tare_with_account_name_static(name)
                );
            }
        }
        if bytes.len() < HEADER_LEN {
            return Err(Error::from(ErrorCode::AccountDidNotDeserialize).__tare_with_account_name_static(name));
        }
        let authority = Pubkey::new_from_array(bytes[8..40].try_into().unwrap());
        let len = u32::from_le_bytes(bytes[40..44].try_into().unwrap());
        Ok(Self {
            info,
            authority,
            len,
        })
    }
    fn save(&self, name: &'static str) -> Result<()> {
        let mut bytes = self
            .info
            .try_borrow_mut_data()
            .map_err(|e| Error::from(e).__tare_with_account_name_static(name))?;
        if bytes.len() < HEADER_LEN {
            return Err(Error::from(ErrorCode::AccountDidNotSerialize).__tare_with_account_name_static(name));
        }
        bytes[..8].copy_from_slice(DISCRIMINATOR);
        bytes[8..40].copy_from_slice(self.authority.as_ref());
        bytes[40..44].copy_from_slice(&self.len.to_le_bytes());
        Ok(())
    }
}
fn signer<'info>(
    cursor: &mut AccountCursor<'info>,
    name: &'static str,
) -> Result<AccountView<'info>> {
    let info = cursor.next().map_err(|e| e.__tare_with_account_name_static(name))?;
    if !info.is_signer() {
        return Err(Error::from(ErrorCode::AccountNotSigner).__tare_with_account_name_static(name));
    }
    Ok(info)
}
fn program<'info>(cursor: &mut AccountCursor<'info>) -> Result<AccountView<'info>> {
    let info = cursor
        .next()
        .map_err(|e| e.__tare_with_account_name_static("system_program"))?;
    let system = solana_program::system_program::ID;
    if info.key_ref() != &system {
        return Err(Error::from(ErrorCode::InvalidProgramId)
            .__tare_with_account_name_static("system_program")
            .with_pubkeys((*info.key_ref(), system)));
    }
    if !info.executable() {
        return Err(
            Error::from(ErrorCode::InvalidProgramExecutable).__tare_with_account_name_static("system_program")
        );
    }
    Ok(info)
}
fn writable(info: AccountView<'_>, name: &'static str) -> Result<()> {
    if !info.is_writable() {
        return Err(Error::from(ErrorCode::ConstraintMut).__tare_with_account_name_static(name));
    }
    Ok(())
}
fn has_one(state: &State<'_>, authority: AccountView<'_>, name: &'static str) -> Result<()> {
    if state.authority != *authority.key_ref() {
        return Err(Error::from(ErrorCode::ConstraintHasOne)
            .__tare_with_account_name_static(name)
            .with_pubkeys((state.authority, *authority.key_ref())));
    }
    Ok(())
}
fn not_erased(authority: AccountView<'_>) -> Result<()> {
    if authority.key_ref() == &ERASED_AUTHORITY {
        return Err(Error::from(ErrorCode::ConstraintRaw).__tare_with_account_name_static("authority"));
    }
    Ok(())
}
fn read_u64(bytes: &[u8]) -> Result<u64> {
    if bytes.len() < 8 {
        return Err(ErrorCode::InstructionDidNotDeserialize.into());
    }
    Ok(u64::from_le_bytes(bytes[..8].try_into().unwrap()))
}
fn log(enabled: bool, value: &'static str) {
    if enabled {
        solana_program::log::sol_log(value);
    }
}

// The stock-generated checks panic at the #[program] call site on unchecked
// slice/overflow failures. Keep that public location instead of leaking this file.
fn panic_at(source: Source, message: std::fmt::Arguments<'_>, new_zero_semantics: bool) -> ! {
    if new_zero_semantics {
        #[cfg(target_os="solana")]
        unsafe {
            if let Some(text)=message.as_str(){solana_program::log::sol_log(text);}
            sol_panic_(source.filename.as_ptr(),source.filename.len() as u64,source.line as u64,1);
        }
        #[cfg(not(target_os="solana"))]
        panic!("{message}");
    }
    crate::log_fmt(format_args!(
        "panicked at {}:{}:1:\n{}",
        source.filename, source.line, message
    ));
    #[cfg(target_os = "solana")]
    unsafe {
        abort()
    }
    #[cfg(not(target_os = "solana"))]
    panic!("{message}")
}
fn checked_add(a: usize, b: usize, source: Source, new_zero_semantics:bool) -> usize {
    a.checked_add(b).unwrap_or_else(|| {
        panic_at(
            source,
            format_args!("called `Option::unwrap()` on a `None` value"), new_zero_semantics)
    })
}
fn require_range(end: usize, len: usize, source: Source, new_zero_semantics:bool) {
    if end > len {
        panic_at(
            source,
            format_args!("range end index {end} out of range for slice of length {len}"), new_zero_semantics);
    }
}

#[inline(never)]
pub fn dispatch<'info>(
    program_id: &Pubkey,
    cursor: &mut AccountCursor<'info>,
    data: &[u8],
    source: Source,
    log_names: bool,
    new_zero_semantics: bool,
) -> Result<()> {
    let Some((&tag, body)) = data.split_first() else {
        return Err(ErrorCode::InstructionDidNotDeserialize.into());
    };
    // Deserialize the entire instruction before extracting any accounts.
    let length = match tag {
        0 | 6 => read_u64(body)?,
        _ => 0,
    };
    let payload = if tag == 2 {
        if body.len() < 4 {
            return Err(ErrorCode::InstructionDidNotDeserialize.into());
        }
        let len = u32::from_le_bytes(body[..4].try_into().unwrap()) as usize;
        if len > body.len() - 4 {
            return Err(ErrorCode::InstructionDidNotDeserialize.into());
        }
        &body[4..4 + len]
    } else {
        &[]
    };
    let new_authority = if tag == 4 {
        if body.len() < 32 {
            return Err(ErrorCode::InstructionDidNotDeserialize.into());
        }
        Pubkey::new_from_array(body[..32].try_into().unwrap())
    } else {
        ERASED_AUTHORITY
    };
    match tag {
        0 => create(program_id, cursor, length, source, log_names, new_zero_semantics),
        1 => create_buffer(program_id, cursor, source, log_names, new_zero_semantics),
        2 | 4 => write_or_set_authority(
            program_id,
            cursor,
            tag,
            payload,
            new_authority,
            source,
            log_names,
            new_zero_semantics,
        ),
        3 => set_buffer(program_id, cursor, source, log_names, new_zero_semantics),
        5 => close(program_id, cursor, source, log_names, new_zero_semantics),
        6 => resize(program_id, cursor, length, source, log_names, new_zero_semantics),
        _ => Err(ErrorCode::InstructionDidNotDeserialize.into()),
    }
}

#[inline(never)]
fn create<'info>(
    program_id: &Pubkey,
    cursor: &mut AccountCursor<'info>,
    length: u64,
    source: Source,
    log_names: bool,
    new_zero_semantics: bool,
) -> Result<()> {
    // Stock deserializes all fields (including Program<System>) first.
    let from = cursor.next().map_err(|e| e.__tare_with_account_name_static("from"))?;
    let to = cursor.next().map_err(|e| e.__tare_with_account_name_static("to"))?;
    let base = cursor.next().map_err(|e| e.__tare_with_account_name_static("base"))?;
    let system = program(cursor)?;
    let program_account = cursor.next().map_err(|e| e.__tare_with_account_name_static("program"))?;
    if !from.is_signer() {
        return Err(Error::from(ErrorCode::ConstraintSigner).__tare_with_account_name_static("from"));
    }
    writable(to, "to")?;
    let (expected_base, bump) = Pubkey::find_program_address(&[], program_id);
    if base.key_ref() != &expected_base {
        return Err(Error::from(ErrorCode::ConstraintSeeds)
            .__tare_with_account_name_static("base")
            .with_pubkeys((*base.key_ref(), expected_base)));
    }
    if !program_account.executable() {
        return Err(Error::from(ErrorCode::ConstraintExecutable).__tare_with_account_name_static("program"));
    }
    log(log_names, "Instruction: IdlCreateAccount");
    if program_id != program_account.key_ref() {
        return Err(ErrorCode::IdlInstructionInvalidProgram.into());
    }
    let expected_to = Pubkey::create_with_seed(&expected_base, "anchor:idl", program_id)
        .unwrap_or_else(|_| {
            panic_at(
                source,
                format_args!("called `Result::unwrap()` on an `Err` value: {:?}",solana_program::program_error::ProgramError::IllegalOwner), new_zero_semantics)
        });
    let space = HEADER_LEN
        .checked_add(length as usize)
        .unwrap_or_else(|| panic_at(source, format_args!("attempt to add with overflow"), new_zero_semantics))
        .min(10_000);
    let rent = Rent::get()?;
    // SystemInstruction::CreateAccountWithSeed, bincode fixed-int layout.
    let mut instruction = [0u8; 102];
    instruction[..4].copy_from_slice(&3u32.to_le_bytes());
    instruction[4..36].copy_from_slice(expected_base.as_ref());
    instruction[36..44].copy_from_slice(&10u64.to_le_bytes());
    instruction[44..54].copy_from_slice(b"anchor:idl");
    instruction[54..62].copy_from_slice(&rent.minimum_balance(space).to_le_bytes());
    instruction[62..70].copy_from_slice(&(space as u64).to_le_bytes());
    instruction[70..102].copy_from_slice(program_id.as_ref());
    crate::cpi::invoke_idl_with_keys(
        system,
        [from, to, base],
        [*from.key_ref(), expected_to, expected_base],
        [true, false, true],
        [true, true, false],
        &instruction,
        &[&[bump]],
    )?;
    let mut state = State::load(to, program_id, "to", false)?;
    state.authority = *from.key_ref();
    state.save("to")
}

#[inline(never)]
fn create_buffer<'info>(
    program_id: &Pubkey,
    cursor: &mut AccountCursor<'info>,
    source: Source,
    log_names: bool,
    new_zero_semantics: bool,
) -> Result<()> {
    let buffer = cursor.next()?;
    let authority = signer(cursor, "authority")?;
    let rent = Rent::get()?;
    {
        let bytes = buffer.try_borrow_data()?;
        if new_zero_semantics && bytes.len()<8 {return Err(Error::from(ErrorCode::AccountDidNotDeserialize).__tare_with_account_name_static("buffer"));}
        require_range(8, bytes.len(), source, new_zero_semantics);
        if bytes[..8].iter().any(|byte| *byte != 0) {
            return Err(Error::from(ErrorCode::ConstraintZero).__tare_with_account_name_static("buffer"));
        }
    }
    let mut state = State::load(buffer, program_id, "buffer", false)?;
    writable(buffer, "buffer")?;
    if new_zero_semantics && !buffer.is_signer() {
        return Err(Error::from(ErrorCode::ConstraintSigner).__tare_with_account_name_static("buffer"));
    }
    if !rent.is_exempt(buffer.lamports(), buffer.data_len()) {
        return Err(Error::from(ErrorCode::ConstraintRentExempt).__tare_with_account_name_static("buffer"));
    }
    not_erased(authority)?;
    log(log_names, "Instruction: IdlCreateBuffer");
    state.authority = *authority.key_ref();
    state.save("buffer")
}

#[inline(never)]
fn write_or_set_authority<'info>(
    program_id: &Pubkey,
    cursor: &mut AccountCursor<'info>,
    tag: u8,
    payload: &[u8],
    new_authority: Pubkey,
    source: Source,
    log_names: bool,
    new_zero_semantics: bool,
) -> Result<()> {
    let raw = cursor.next().map_err(|e| e.__tare_with_account_name_static("idl"))?;
    let mut state = State::load(raw, program_id, "idl", true)?;
    let authority = signer(cursor, "authority")?;
    writable(raw, "idl")?;
    has_one(&state, authority, "idl")?;
    not_erased(authority)?;
    if tag == 2 {
        log(log_names, "Instruction: IdlWrite");
        let start = state.len as usize;
        let end = checked_add(start, payload.len(), source, new_zero_semantics);
        state.len = state
            .len
            .checked_add(payload.len() as u32)
            .unwrap_or_else(|| {
                panic_at(
                    source,
                    format_args!("called `Option::unwrap()` on a `None` value"), new_zero_semantics)
            });
        let mut bytes = raw.try_borrow_mut_data().unwrap_or_else(|_| {
            panic_at(
                source,
                format_args!("called `Result::unwrap()` on an `Err` value: {:?}",solana_program::program_error::ProgramError::AccountBorrowFailed), new_zero_semantics)
        });
        require_range(end, bytes.len() - HEADER_LEN, source, new_zero_semantics);
        bytes[HEADER_LEN + start..HEADER_LEN + end].copy_from_slice(payload);
    } else {
        log(log_names, "Instruction: IdlSetAuthority");
        state.authority = new_authority;
    }
    state.save("idl")
}

#[inline(never)]
fn set_buffer<'info>(
    program_id: &Pubkey,
    cursor: &mut AccountCursor<'info>,
    source: Source,
    log_names: bool,
    new_zero_semantics: bool,
) -> Result<()> {
    let buffer_info = cursor.next().map_err(|e| e.__tare_with_account_name_static("buffer"))?;
    let buffer = State::load(buffer_info, program_id, "buffer", true)?;
    let idl_info = cursor.next().map_err(|e| e.__tare_with_account_name_static("idl"))?;
    let mut idl = State::load(idl_info, program_id, "idl", true)?;
    let authority = signer(cursor, "authority")?;
    if new_zero_semantics && buffer_info.key_ref()==idl_info.key_ref() {
        return Err(Error::from_static(&crate::error::ErrorDescriptor {code:2040,name:"ConstraintDuplicateMutableAccount",message:"A duplicate mutable account constraint was violated"}).__tare_with_account_name_static("idl"));
    }
    writable(buffer_info, "buffer")?;
    if buffer.authority != idl.authority {
        return Err(Error::from(ErrorCode::ConstraintRaw).__tare_with_account_name_static("buffer"));
    }
    writable(idl_info, "idl")?;
    has_one(&idl, authority, "idl")?;
    not_erased(authority)?;
    log(log_names, "Instruction: IdlSetBuffer");
    idl.len = buffer.len;
    {
        let mut target = idl_info.try_borrow_mut_data().unwrap_or_else(|_| {
            panic_at(
                source,
                format_args!("called `Result::unwrap()` on an `Err` value: {:?}",solana_program::program_error::ProgramError::AccountBorrowFailed), new_zero_semantics)
        });
        let bytes = buffer_info.try_borrow_data().unwrap_or_else(|_| {
            panic_at(
                source,
                format_args!("called `Result::unwrap()` on an `Err` value: {:?}",solana_program::program_error::ProgramError::AccountBorrowFailed), new_zero_semantics)
        });
        let len = buffer.len as usize;
        require_range(len, bytes.len() - HEADER_LEN, source, new_zero_semantics);
        if target.len() - HEADER_LEN < len {
            return Err(Error::from(ErrorCode::RequireGteViolated)
                .with_source(source)
                .with_values((target.len() - HEADER_LEN, len)));
        }
        target[HEADER_LEN..HEADER_LEN + len].copy_from_slice(&bytes[HEADER_LEN..HEADER_LEN + len]);
        if new_zero_semantics {target[HEADER_LEN+len..].fill(0);}
    }
    buffer.save("buffer")?;
    idl.save("idl")
}

#[inline(never)]
fn close<'info>(
    program_id: &Pubkey,
    cursor: &mut AccountCursor<'info>,
    source: Source,
    log_names: bool,
    new_zero_semantics: bool,
) -> Result<()> {
    let raw = cursor.next().map_err(|e| e.__tare_with_account_name_static("account"))?;
    let state = State::load(raw, program_id, "account", true)?;
    let authority = signer(cursor, "authority")?;
    let destination = cursor
        .next()
        .map_err(|e| e.__tare_with_account_name_static("sol_destination"))?;
    writable(raw, "account")?;
    has_one(&state, authority, "account")?;
    if raw.key_ref() == destination.key_ref() {
        return Err(Error::from(ErrorCode::ConstraintClose).__tare_with_account_name_static("account"));
    }
    not_erased(authority)?;
    writable(destination, "sol_destination")?;
    log(log_names, "Instruction: IdlCloseAccount");
    destination.set_lamports(
        destination
            .lamports()
            .checked_add(raw.lamports())
            .unwrap_or_else(|| {
                panic_at(
                    source,
                    format_args!("called `Option::unwrap()` on a `None` value"), new_zero_semantics)
            }),
    );
    raw.set_lamports(0);
    raw.assign(&solana_program::system_program::ID);
    raw.realloc(0, false)
        .map_err(|e| e.__tare_with_account_name_static("account"))
}

#[inline(never)]
fn resize<'info>(
    program_id: &Pubkey,
    cursor: &mut AccountCursor<'info>,
    length: u64,
    source: Source,
    log_names: bool,
    new_zero_semantics: bool,
) -> Result<()> {
    let raw = cursor.next().map_err(|e| e.__tare_with_account_name_static("idl"))?;
    let state = State::load(raw, program_id, "idl", true)?;
    let authority = signer(cursor, "authority")?;
    let system = program(cursor)?;
    writable(raw, "idl")?;
    has_one(&state, authority, "idl")?;
    writable(authority, "authority")?;
    not_erased(authority)?;
    log(log_names, "Instruction: IdlResizeAccount");
    if state.len != 0 {
        return Err(ErrorCode::IdlAccountNotEmpty.into());
    }
    let increase = (length as usize)
        .checked_sub(raw.data_len())
        .unwrap_or_else(|| {
            panic_at(
                source,
                format_args!("data_len should always be >= the current account space"), new_zero_semantics)
        })
        .min(10_000);
    let space = checked_add(raw.data_len(), increase, source, new_zero_semantics);
    if space > raw.data_len() {
        let rent = Rent::get()?;
        let lamports = rent
            .minimum_balance(space)
            .checked_sub(raw.lamports())
            .unwrap_or_else(|| {
                panic_at(
                    source,
                    format_args!("called `Option::unwrap()` on a `None` value"), new_zero_semantics)
            });
        let mut transfer = [0u8; 12];
        transfer[..4].copy_from_slice(&2u32.to_le_bytes());
        transfer[4..].copy_from_slice(&lamports.to_le_bytes());
        crate::cpi::invoke_idl(
            system,
            [authority, raw],
            [true, false],
            [true, true],
            &transfer,
            &[],
        )?;
        raw.realloc(space, new_zero_semantics)?;
    }
    state.save("idl")
}
