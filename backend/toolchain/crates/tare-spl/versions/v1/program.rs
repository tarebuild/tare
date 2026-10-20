//! Preserve SPL instruction IDs independently of the caller's context program.
use anchor_lang::{AccountInfo, Result};
use anchor_lang::solana_program::instruction::Instruction;

pub(crate) fn invoke_signed(
    instruction: &Instruction,
    accounts: &[AccountInfo<'_>],
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    anchor_lang::cpi::invoke_dynamic_with_id(
        &instruction.program_id,
        accounts,
        &instruction.accounts,
        &instruction.data,
        signer_seeds,
    )
}

#[cfg(feature = "token_2022")]
pub(crate) fn invoke(instruction: &Instruction, accounts: &[AccountInfo<'_>]) -> Result<()> {
    invoke_signed(instruction, accounts, &[])
}
