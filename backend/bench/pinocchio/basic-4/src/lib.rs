#![no_std]
use pinocchio::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, ProgramResult,
};
use tare_handwritten_support as h;
pinocchio::program_entrypoint!(process, 3);
pinocchio::nostd_panic_handler!();
pinocchio::no_allocator!();
const ID: Pubkey = [
    177, 126, 69, 205, 141, 254, 171, 102, 168, 250, 252, 66, 218, 99, 87, 166, 199, 188, 18, 44,
    228, 108, 24, 96, 33, 141, 217, 15, 32, 45, 67, 101,
];
const INIT: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];
const INCREMENT: [u8; 8] = [11, 18, 104, 9, 104, 174, 59, 33];
const DISC: [u8; 8] = [255, 176, 4, 245, 188, 253, 124, 25];
fn process(id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if id != &ID {
        return Err(h::error(4100, None));
    }
    if data.starts_with(&INIT) {
        pinocchio::msg!("Instruction: Initialize");
        let counter = h::at(accounts, 0, None)?;
        let authority = h::at(accounts, 1, Some("authority"))?;
        h::signer(authority, "authority")?;
        let system = h::at(accounts, 2, Some("system_program"))?;
        h::system(system)?;
        let rent = h::rent()?;
        let (expected, bump) = pinocchio::pubkey::find_program_address(&[b"counter"], &ID);
        if counter.key() != &expected {
            return Err(h::keys(2006, "counter", counter.key(), &expected));
        }
        let bump_bytes = [bump];
        let seeds = [
            pinocchio::instruction::Seed::from(b"counter"),
            pinocchio::instruction::Seed::from(&bump_bytes),
        ];
        h::create(authority, counter, &ID, 49, &rent, &seeds)?;
        h::state(counter, &ID, &DISC, 49, "counter", false)?;
        h::writable(counter, "counter")?;
        h::rent_exempt(counter, &rent, "counter")?;
        h::writable(authority, "authority")?;
        let bytes = unsafe { counter.borrow_mut_data_unchecked() };
        bytes[..8].copy_from_slice(&DISC);
        bytes[8..40].copy_from_slice(authority.key());
        bytes[40..48].copy_from_slice(&0u64.to_le_bytes());
        bytes[48] = bump;
        return Ok(());
    }
    if data.starts_with(&INCREMENT) {
        pinocchio::msg!("Instruction: Increment");
        let counter = h::at(accounts, 0, Some("counter"))?;
        h::state(counter, &ID, &DISC, 49, "counter", true)?;
        let authority = h::at(accounts, 1, Some("authority"))?;
        h::signer(authority, "authority")?;
        let bytes = unsafe { counter.borrow_data_unchecked() };
        let expected = pinocchio::pubkey::create_program_address(&[b"counter", &[bytes[48]]], &ID)
            .map_err(|_| h::error(2006, Some("counter")))?;
        if counter.key() != &expected {
            return Err(h::keys(2006, "counter", counter.key(), &expected));
        }
        h::writable(counter, "counter")?;
        let owner: Pubkey = bytes[8..40].try_into().unwrap();
        if authority.key() != &owner {
            pinocchio::msg!(concat!("AnchorError thrown in ", env!("REFERENCE_SOURCE"), ":24. Error Code: Unauthorized. Error Number: 6000. Error Message: You are not authorized to perform this action.."));
            pinocchio::msg!("Left:");
            pinocchio::pubkey::log(authority.key());
            pinocchio::msg!("Right:");
            pinocchio::pubkey::log(&owner);
            return Err(ProgramError::Custom(6000));
        }
        let count = u64::from_le_bytes(bytes[40..48].try_into().unwrap());
        let next = count.checked_add(1).unwrap_or_else(|| {
            pinocchio::msg!(concat!("panicked at ", env!("REFERENCE_SOURCE"), ":30:9:\nattempt to add with overflow"));
            unsafe { pinocchio::syscalls::abort() }
        });
        (unsafe { counter.borrow_mut_data_unchecked() })[40..48]
            .copy_from_slice(&next.to_le_bytes());
        return Ok(());
    }
    Err(h::error(
        if data.starts_with(&0x1d9acb512ea545e4u64.to_le_bytes()) {
            1500
        } else {
            101
        },
        None,
    ))
}
