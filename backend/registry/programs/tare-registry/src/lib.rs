use anchor_lang::prelude::*;
use anchor_lang::system_program::{self, Transfer};

declare_id!("7viqDjx5k8ffnfnZiBv9dMhxzVhcGSYLMb5bKdDP1XtP");

pub const MAX_REPOSITORY: usize = 256;
pub const MAX_COMMIT: usize = 32;
pub const MAX_VERSION: usize = 64;
pub const MAX_LOCATION: usize = 512;

#[program]
pub mod tare_registry {
    use super::*;

    pub fn init_config(ctx: Context<InitConfig>, admin: Pubkey, fee_recipient: Pubkey, attest_fee_lamports: u64) -> Result<()> {
        ctx.accounts.config.set_inner(Config {
            admin,
            fee_recipient,
            attest_fee_lamports,
        });
        emit_cpi!(ConfigInitialized {
            admin,
            fee_recipient,
            attest_fee_lamports,
        });
        Ok(())
    }

    pub fn set_config(ctx: Context<SetConfig>, new_admin: Pubkey, fee_recipient: Pubkey, attest_fee_lamports: u64) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.admin = new_admin;
        config.fee_recipient = fee_recipient;
        config.attest_fee_lamports = attest_fee_lamports;
        emit_cpi!(ConfigUpdated {
            admin: new_admin,
            fee_recipient,
            attest_fee_lamports,
        });
        Ok(())
    }

    pub fn attest(ctx: Context<Attest>, args: AttestArgs) -> Result<()> {
        require!(!args.source_repository.is_empty() && args.source_repository.len() <= MAX_REPOSITORY, RegistryError::RepositoryLength);
        require!(args.source_commit.len() == 20 || args.source_commit.len() == 32, RegistryError::CommitLength);
        require!(!args.toolchain_version.is_empty() && args.toolchain_version.len() <= MAX_VERSION, RegistryError::VersionLength);
        require!(!args.report_location.is_empty() && args.report_location.len() <= MAX_LOCATION, RegistryError::LocationLength);
        let fee_lamports = ctx.accounts.config.attest_fee_lamports;
        system_program::transfer(
            CpiContext::new(ctx.accounts.system_program.key(), Transfer {
                from: ctx.accounts.attester.to_account_info(),
                to: ctx.accounts.fee_recipient.to_account_info(),
            }),
            fee_lamports,
        )?;
        let record = &mut ctx.accounts.record;
        record.set_inner(Record {
            program: args.program,
            stock_build_hash: args.stock_build_hash,
            tare_build_hash: args.tare_build_hash,
            source_repository: args.source_repository,
            source_commit: args.source_commit,
            toolchain_version: args.toolchain_version,
            corpus_hash: args.corpus_hash,
            tests_count: args.tests_count,
            generated_count: args.generated_count,
            mainnet_count: args.mainnet_count,
            gross: args.gross,
            net: args.net,
            report_hash: args.report_hash,
            report_location: args.report_location,
            attester: ctx.accounts.attester.key(),
            slot: Clock::get()?.slot,
        });
        let event = RecordAttested {
            program: record.program,
            stock_build_hash: record.stock_build_hash,
            tare_build_hash: record.tare_build_hash,
            source_repository: record.source_repository.clone(),
            source_commit: hex_commit(&record.source_commit),
            toolchain_version: record.toolchain_version.clone(),
            corpus_hash: record.corpus_hash,
            tests_count: record.tests_count,
            generated_count: record.generated_count,
            mainnet_count: record.mainnet_count,
            gross: record.gross,
            net: record.net,
            report_hash: record.report_hash,
            report_location: record.report_location.clone(),
            attester: record.attester,
            slot: record.slot,
            fee_recipient: ctx.accounts.fee_recipient.key(),
            fee_lamports,
        };
        emit_cpi!(event);
        Ok(())
    }

    pub fn flag(ctx: Context<FlagRecord>, input_hash: [u8; 32], input_location: String) -> Result<()> {
        require!(!input_location.is_empty() && input_location.len() <= MAX_LOCATION, RegistryError::LocationLength);
        ctx.accounts.flag.set_inner(Flag {
            record: ctx.accounts.record.key(),
            input_hash,
            input_location,
            flagger: ctx.accounts.flagger.key(),
            slot: Clock::get()?.slot,
        });
        let flag = &ctx.accounts.flag;
        let event = RecordFlagged {
            record: flag.record,
            input_hash: flag.input_hash,
            input_location: flag.input_location.clone(),
            flagger: flag.flagger,
            slot: flag.slot,
        };
        emit_cpi!(event);
        Ok(())
    }
}

#[event_cpi]
#[derive(Accounts)]
pub struct InitConfig<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(constraint = registry_program.programdata_address()? == Some(program_data.key()) @ RegistryError::ProgramDataMismatch)]
    pub registry_program: Program<'info, crate::program::TareRegistry>,
    #[account(constraint = program_data.upgrade_authority_address == Some(authority.key()) @ RegistryError::Unauthorized)]
    pub program_data: Account<'info, ProgramData>,
    #[account(init, payer = authority, space = Config::SPACE, seeds = [b"config"], bump)]
    pub config: Account<'info, Config>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct SetConfig<'info> {
    pub admin: Signer<'info>,
    #[account(mut, seeds = [b"config"], bump, has_one = admin @ RegistryError::Unauthorized)]
    pub config: Account<'info, Config>,
}

#[event_cpi]
#[derive(Accounts)]
#[instruction(args: AttestArgs)]
pub struct Attest<'info> {
    #[account(mut)]
    pub attester: Signer<'info>,
    #[account(seeds = [b"config"], bump, has_one = fee_recipient @ RegistryError::FeeRecipientMismatch)]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub fee_recipient: SystemAccount<'info>,
    #[account(init, payer = attester, space = Record::SPACE,
        seeds = [b"record", args.program.as_ref(), args.report_hash.as_ref(), attester.key().as_ref()], bump)]
    pub record: Account<'info, Record>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
#[instruction(input_hash: [u8; 32])]
pub struct FlagRecord<'info> {
    #[account(mut)]
    pub flagger: Signer<'info>,
    pub record: Account<'info, Record>,
    #[account(init, payer = flagger, space = Flag::SPACE,
        seeds = [b"flag", record.key().as_ref(), input_hash.as_ref()], bump)]
    pub flag: Account<'info, Flag>,
    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct AttestArgs {
    pub program: Pubkey,
    pub stock_build_hash: [u8; 32],
    pub tare_build_hash: [u8; 32],
    pub source_repository: String,
    pub source_commit: Vec<u8>,
    pub toolchain_version: String,
    pub corpus_hash: [u8; 32],
    pub tests_count: u64,
    pub generated_count: u64,
    pub mainnet_count: u64,
    pub gross: u64,
    pub net: u64,
    pub report_hash: [u8; 32],
    pub report_location: String,
}

#[account]
pub struct Config {
    pub admin: Pubkey,
    pub fee_recipient: Pubkey,
    pub attest_fee_lamports: u64,
}
impl Config { pub const SPACE: usize = 8 + 32 + 32 + 8; }

#[account]
pub struct Record {
    pub program: Pubkey,
    pub stock_build_hash: [u8; 32],
    pub tare_build_hash: [u8; 32],
    pub source_repository: String,
    pub source_commit: Vec<u8>,
    pub toolchain_version: String,
    pub corpus_hash: [u8; 32],
    pub tests_count: u64,
    pub generated_count: u64,
    pub mainnet_count: u64,
    pub gross: u64,
    pub net: u64,
    pub report_hash: [u8; 32],
    pub report_location: String,
    pub attester: Pubkey,
    pub slot: u64,
}
impl Record {
    pub const SPACE: usize = 8 + 6 * 32 + (4 + MAX_REPOSITORY) + (4 + MAX_COMMIT)
        + (4 + MAX_VERSION) + (4 + MAX_LOCATION) + 6 * 8;
}

#[account]
pub struct Flag {
    pub record: Pubkey,
    pub input_hash: [u8; 32],
    pub input_location: String,
    pub flagger: Pubkey,
    pub slot: u64,
}
impl Flag { pub const SPACE: usize = 8 + 3 * 32 + 4 + MAX_LOCATION + 8; }

#[event]
pub struct ConfigInitialized {
    pub admin: Pubkey,
    pub fee_recipient: Pubkey,
    pub attest_fee_lamports: u64,
}
#[event]
pub struct ConfigUpdated {
    pub admin: Pubkey,
    pub fee_recipient: Pubkey,
    pub attest_fee_lamports: u64,
}
#[event]
pub struct RecordAttested {
    pub program: Pubkey,
    pub stock_build_hash: [u8; 32],
    pub tare_build_hash: [u8; 32],
    pub source_repository: String,
    pub source_commit: String,
    pub toolchain_version: String,
    pub corpus_hash: [u8; 32],
    pub tests_count: u64,
    pub generated_count: u64,
    pub mainnet_count: u64,
    pub gross: u64,
    pub net: u64,
    pub report_hash: [u8; 32],
    pub report_location: String,
    pub attester: Pubkey,
    pub slot: u64,
    pub fee_recipient: Pubkey,
    pub fee_lamports: u64,
}
#[event]
pub struct RecordFlagged {
    pub record: Pubkey,
    pub input_hash: [u8; 32],
    pub input_location: String,
    pub flagger: Pubkey,
    pub slot: u64,
}

fn hex_commit(bytes: &[u8]) -> String {
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(b"0123456789abcdef"[(byte >> 4) as usize] as char);
        result.push(b"0123456789abcdef"[(byte & 15) as usize] as char);
    }
    result
}

#[error_code]
pub enum RegistryError {
    #[msg("Only the configured authority may perform this action")]
    Unauthorized,
    #[msg("The program-data account does not belong to this registry")]
    ProgramDataMismatch,
    #[msg("Source repository must contain between 1 and 256 UTF-8 bytes")]
    RepositoryLength,
    #[msg("Source commit must contain 20 or 32 bytes")]
    CommitLength,
    #[msg("Toolchain version must contain between 1 and 64 UTF-8 bytes")]
    VersionLength,
    #[msg("Pinned location must contain between 1 and 512 UTF-8 bytes")]
    LocationLength,
    #[msg("The fee recipient is not the one the registry config names")]
    FeeRecipientMismatch,
}
