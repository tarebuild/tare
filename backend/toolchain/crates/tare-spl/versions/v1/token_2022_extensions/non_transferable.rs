// Avoiding AccountInfo deprecated msg in anchor context
#![allow(deprecated)]
use anchor_lang::AccountInfo;
use {
    crate::compat::spl_token_2022_interface,
    anchor_lang::{
        context::CpiContext,
        solana_program::{ pubkey::Pubkey},
        Accounts, Result,
    },
    spl_token_2022_interface as spl_token_2022,
};

pub fn non_transferable_mint_initialize<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, NonTransferableMintInitialize<'info>>,
) -> Result<()> {
    let ix = spl_token_2022::instruction::initialize_non_transferable_mint(
        ctx.accounts.token_program_id.key,
        ctx.accounts.mint.key,
    )?;
    crate::program::invoke_signed(
        &ix,
        &[ctx.accounts.token_program_id, ctx.accounts.mint],
        ctx.signer_seeds,
    )
    .map_err(Into::into)
}

#[derive(Accounts)]
pub struct NonTransferableMintInitialize<'info> {
    pub token_program_id: AccountInfo<'info>,
    pub mint: AccountInfo<'info>,
}
