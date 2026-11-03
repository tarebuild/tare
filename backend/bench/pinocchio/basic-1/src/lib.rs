#![no_std]
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey, ProgramResult};
use tare_handwritten_support as h;
pinocchio::program_entrypoint!(process, 3);
pinocchio::nostd_panic_handler!();
pinocchio::no_allocator!();
const ID: Pubkey = [
    218, 7, 92, 178, 255, 94, 198, 129, 118, 19, 222, 83, 11, 105, 42, 135, 53, 71, 119, 105, 218,
    71, 67, 12, 189, 129, 84, 51, 92, 74, 131, 39,
];
const INIT: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];
const UPDATE: [u8; 8] = [219, 200, 88, 176, 158, 63, 253, 127];
const DISC: [u8; 8] = [246, 28, 6, 87, 251, 45, 50, 42];
fn process(id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if id != &ID {
        return Err(h::error(4100, None));
    }
    if data.starts_with(&INIT) {
        pinocchio::msg!("Instruction: Initialize");
        if data.len() < 16 {
            return Err(h::error(102, None));
        }
        let account = h::at(accounts, 0, None)?;
        let user = h::at(accounts, 1, Some("user"))?;
        h::signer(user, "user")?;
        let system = h::at(accounts, 2, Some("system_program"))?;
        h::system(system)?;
        let rent = h::rent()?;
        h::create(user, account, &ID, 16, &rent, &[])?;
        h::state(account, &ID, &DISC, 16, "my_account", false)?;
        h::writable(account, "my_account")?;
        if !account.is_signer() {
            return Err(h::error(2002, Some("my_account")));
        }
        h::rent_exempt(account, &rent, "my_account")?;
        h::writable(user, "user")?;
        let bytes = unsafe { account.borrow_mut_data_unchecked() };
        bytes[..8].copy_from_slice(&DISC);
        bytes[8..16].copy_from_slice(&data[8..16]);
        return Ok(());
    }
    if data.starts_with(&UPDATE) {
        pinocchio::msg!("Instruction: Update");
        if data.len() < 16 {
            return Err(h::error(102, None));
        }
        let account = h::at(accounts, 0, Some("my_account"))?;
        h::state(account, &ID, &DISC, 16, "my_account", true)?;
        h::writable(account, "my_account")?;
        (unsafe { account.borrow_mut_data_unchecked() })[8..16].copy_from_slice(&data[8..16]);
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
