#![no_std]
use core::fmt::{self, Write};
pub use pinocchio;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey::{self, Pubkey},
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};
pub const SYSTEM: Pubkey = [0; 32];
struct Text {
    bytes: [u8; 512],
    len: usize,
}
impl Write for Text {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if self.len + text.len() > 512 {
            return Err(fmt::Error);
        }
        self.bytes[self.len..self.len + text.len()].copy_from_slice(text.as_bytes());
        self.len += text.len();
        Ok(())
    }
}
fn log_fmt(args: fmt::Arguments) {
    let mut out = Text {
        bytes: [0; 512],
        len: 0,
    };
    out.write_fmt(args).unwrap();
    pinocchio::log::sol_log(unsafe { core::str::from_utf8_unchecked(&out.bytes[..out.len]) });
}
#[cold]
#[inline(never)]
pub fn error(code: u32, field: Option<&str>) -> ProgramError {
    let (name, msg) = match code {
        101 => (
            "InstructionFallbackNotFound",
            "Fallback functions are not supported",
        ),
        102 => (
            "InstructionDidNotDeserialize",
            "The program could not deserialize the given instruction",
        ),
        1500 => (
            "EventInstructionStub",
            "The program was compiled without `event-cpi` feature",
        ),
        2000 => ("ConstraintMut", "A mut constraint was violated"),
        2002 => ("ConstraintSigner", "A signer constraint was violated"),
        2005 => (
            "ConstraintRentExempt",
            "A rent exemption constraint was violated",
        ),
        2006 => ("ConstraintSeeds", "A seeds constraint was violated"),
        3001 => (
            "AccountDiscriminatorNotFound",
            "No discriminator was found on the account",
        ),
        3002 => (
            "AccountDiscriminatorMismatch",
            "Account discriminator did not match what was expected",
        ),
        3003 => (
            "AccountDidNotDeserialize",
            "Failed to deserialize the account",
        ),
        3005 => (
            "AccountNotEnoughKeys",
            "Not enough account keys given to the instruction",
        ),
        3007 => (
            "AccountOwnedByWrongProgram",
            "The given account is owned by a different program than expected",
        ),
        3008 => ("InvalidProgramId", "Program ID was not as expected"),
        3009 => (
            "InvalidProgramExecutable",
            "Program account is not executable",
        ),
        3010 => ("AccountNotSigner", "The given account did not sign"),
        3012 => (
            "AccountNotInitialized",
            "The program expected this account to be already initialized",
        ),
        4100 => (
            "DeclaredProgramIdMismatch",
            "The declared program id does not match the actual program id",
        ),
        _ => panic!("unhandled reference error"),
    };
    if let Some(field) = field {
        log_fmt(format_args!("AnchorError caused by account: {}. Error Code: {}. Error Number: {}. Error Message: {}.",field,name,code,msg));
    } else {
        log_fmt(format_args!(
            "AnchorError occurred. Error Code: {}. Error Number: {}. Error Message: {}.",
            name, code, msg
        ));
    }
    ProgramError::Custom(code)
}
#[cold]
#[inline(never)]
pub fn keys(code: u32, field: &str, left: &Pubkey, right: &Pubkey) -> ProgramError {
    let err = error(code, Some(field));
    pinocchio::msg!("Left:");
    pubkey::log(left);
    pinocchio::msg!("Right:");
    pubkey::log(right);
    err
}
#[inline(always)]
pub fn at<'a>(
    accounts: &'a [AccountInfo],
    index: usize,
    field: Option<&str>,
) -> Result<&'a AccountInfo, ProgramError> {
    accounts.get(index).ok_or_else(|| error(3005, field))
}
#[inline(always)]
pub fn signer(account: &AccountInfo, field: &str) -> ProgramResult {
    if !account.is_signer() {
        return Err(error(3010, Some(field)));
    }
    Ok(())
}
#[inline(always)]
pub fn writable(account: &AccountInfo, field: &str) -> ProgramResult {
    if !account.is_writable() {
        return Err(error(2000, Some(field)));
    }
    Ok(())
}
#[inline(always)]
pub fn system(account: &AccountInfo) -> ProgramResult {
    if account.key() != &SYSTEM {
        return Err(keys(3008, "system_program", account.key(), &SYSTEM));
    }
    if !account.executable() {
        return Err(error(3009, Some("system_program")));
    }
    Ok(())
}
#[inline(always)]
pub fn state(
    account: &AccountInfo,
    owner: &Pubkey,
    discriminator: &[u8; 8],
    len: usize,
    field: &str,
    checked: bool,
) -> ProgramResult {
    if account.owner() == &SYSTEM && account.lamports() == 0 {
        return Err(error(3012, Some(field)));
    }
    if account.owner() != owner {
        return Err(keys(3007, field, account.owner(), owner));
    }
    let bytes = unsafe { account.borrow_data_unchecked() };
    if checked {
        if bytes.len() < 8 {
            return Err(error(3001, Some(field)));
        }
        if &bytes[..8] != discriminator {
            return Err(error(3002, Some(field)));
        }
    }
    if bytes.len() < len {
        return Err(error(3003, Some(field)));
    }
    Ok(())
}
#[cold]
#[inline(never)]
pub fn program_error(err: ProgramError) -> ProgramError {
    let number = u64::from(err);
    let mut out = Text {
        bytes: [0; 512],
        len: 0,
    };
    let message = match err {
        ProgramError::InvalidArgument => {
            "The arguments provided to a program instruction were invalid"
        }
        ProgramError::InvalidInstructionData => "An instruction's data contents was invalid",
        ProgramError::InvalidAccountData => "An account's data contents was invalid",
        ProgramError::InsufficientFunds => {
            "An account's balance was too small to complete the instruction"
        }
        ProgramError::IncorrectProgramId => "The account did not have the expected program id",
        ProgramError::MissingRequiredSignature => "A signature was required but not found",
        ProgramError::AccountAlreadyInitialized => {
            "An initialize instruction was sent to an account that has already been initialized"
        }
        ProgramError::InvalidSeeds => "Provided seeds do not result in a valid address",
        _ => "",
    };
    match err {ProgramError::Custom(code)=>write!(out,"ProgramError occurred. Error Code: {:?}. Error Number: {}. Error Message: Custom program error: {:#x}.",err,number,code).unwrap(),_=>write!(out,"ProgramError occurred. Error Code: {:?}. Error Number: {}. Error Message: {}.",err,number,message).unwrap()}
    pinocchio::log::sol_log(unsafe { core::str::from_utf8_unchecked(&out.bytes[..out.len]) });
    err
}
#[inline(always)]
pub fn create(
    payer: &AccountInfo,
    target: &AccountInfo,
    owner: &Pubkey,
    space: usize,
    rent: &Rent,
    seeds: &[Seed],
) -> ProgramResult {
    let signer = Signer::from(seeds);
    let signers = if seeds.is_empty() {
        &[][..]
    } else {
        core::slice::from_ref(&signer)
    };
    let amount = target.lamports();
    if amount == 0 {
        pinocchio_system::instructions::CreateAccount {
            from: payer,
            to: target,
            lamports: rent.minimum_balance(space),
            space: space as u64,
            owner,
        }
        .invoke_signed(signers)
        .map_err(program_error)?;
    } else {
        let missing = rent.minimum_balance(space).max(1).saturating_sub(amount);
        if missing > 0 {
            pinocchio_system::instructions::Transfer {
                from: payer,
                to: target,
                lamports: missing,
            }
            .invoke()
            .map_err(program_error)?;
        }
        pinocchio_system::instructions::Allocate {
            account: target,
            space: space as u64,
        }
        .invoke_signed(signers)
        .map_err(program_error)?;
        pinocchio_system::instructions::Assign {
            account: target,
            owner,
        }
        .invoke_signed(signers)
        .map_err(program_error)?;
    }
    Ok(())
}
#[inline(always)]
pub fn rent() -> Result<Rent, ProgramError> {
    Rent::get().map_err(program_error)
}
#[inline(always)]
pub fn rent_exempt(account: &AccountInfo, rent: &Rent, field: &str) -> ProgramResult {
    if !rent.is_exempt(account.lamports(), account.data_len()) {
        return Err(error(2005, Some(field)));
    }
    Ok(())
}
