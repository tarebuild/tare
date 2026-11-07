// An Anchor program's source as lines of blocks: the ground of the landing's first view.
// A sample program in the form every Anchor v1 workspace has; the build reads source like it unchanged.

import type { Stanza, Token } from './lines'

const SOURCE = `
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("Vau1t11111111111111111111111111111111111111");

#[program]
pub mod vault {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, bump: u8) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.authority = ctx.accounts.authority.key();
        vault.bump = bump;
        vault.total = 0;
        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::ZeroAmount);
        token::transfer(ctx.accounts.transfer_ctx(), amount)?;
        let vault = &mut ctx.accounts.vault;
        vault.total = vault.total.checked_add(amount).unwrap();
        emit!(Deposited { owner: ctx.accounts.owner.key(), amount });
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        require!(vault.total >= amount, VaultError::Insufficient);
        vault.total -= amount;
        token::transfer(ctx.accounts.signed_ctx(), amount)?;
        emit!(Withdrawn { owner: ctx.accounts.owner.key(), amount });
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut, seeds = [b"vault"], bump = vault.bump)]
    pub vault: Account<'info, Vault>,
    #[account(mut, token::authority = owner)]
    pub from: Account<'info, TokenAccount>,
    #[account(mut)]
    pub to: Account<'info, TokenAccount>,
    pub owner: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[account]
pub struct Vault {
    pub authority: Pubkey,
    pub total: u64,
    pub bump: u8,
}

#[event]
pub struct Deposited {
    pub owner: Pubkey,
    pub amount: u64,
}

#[error_code]
pub enum VaultError {
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    #[msg("Vault holds less than the amount")]
    Insufficient,
}
`

/** The source split into stanzas at its blank lines; each word a token, its indent kept. */
export const SOURCE_STANZAS: Stanza[] = SOURCE.trim()
  .split(/\n\s*\n/)
  .map((block) =>
    block.split('\n').map((raw): Token[] => {
      const indent = raw.length - raw.trimStart().length
      return raw
        .trim()
        .split(' ')
        .map((t, i) => (i === 0 && indent ? { t, gap: indent } : { t }))
    }),
  )

/** The source starting from stanza `from`, wrapped, `n` stanzas long. */
export function sourceFrom(from: number, n: number): Stanza[] {
  return Array.from({ length: n }, (_, i) => SOURCE_STANZAS[(from + i) % SOURCE_STANZAS.length]!)
}
