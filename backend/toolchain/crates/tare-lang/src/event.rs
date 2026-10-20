use crate::error::{Error, ErrorCode};
use crate::{AccountCursor, AccountView, Key, Pubkey, Result};
pub const EVENT_IX_TAG: u64 = 0x1d9acb512ea545e4;
pub const EVENT_IX_TAG_LE: &[u8] = &EVENT_IX_TAG.to_le_bytes();

#[inline(always)]
pub fn dispatch(program_id: &Pubkey, accounts: &mut AccountCursor) -> Result<()> {
    #[cfg(not(feature = "event-cpi"))]
    {
        let _ = (program_id, accounts);
        Err(ErrorCode::EventInstructionStub.into())
    }
    #[cfg(feature = "event-cpi")]
    {
        let authority = accounts
            .next()
            .map_err(|_| solana_program::program_error::ProgramError::NotEnoughAccountKeys)?;
        if !authority.is_signer() {
            return Err(
                Error::from(ErrorCode::ConstraintSigner).__tare_with_account_name_static("event_authority")
            );
        }
        let expected = Pubkey::find_program_address(&[b"__event_authority"], program_id).0;
        if authority.key() != expected {
            return Err(Error::from(ErrorCode::ConstraintSeeds)
                .__tare_with_account_name_static("event_authority")
                .with_pubkeys((authority.key(), expected)));
        }
        Ok(())
    }
}
/// Anchor 1.x declares the event authority at compile time.
#[inline(always)]
pub fn dispatch_known(accounts: &mut AccountCursor, expected: &Pubkey) -> Result<()> {
    #[cfg(not(feature = "event-cpi"))]
    {
        let _ = (accounts, expected);
        Err(ErrorCode::EventInstructionStub.into())
    }
    #[cfg(feature = "event-cpi")]
    {
        let authority = accounts
            .next()
            .map_err(|_| solana_program::program_error::ProgramError::NotEnoughAccountKeys)?;
        if !authority.is_signer() {
            return Err(Error::from(ErrorCode::ConstraintSigner)
                .__tare_with_account_name_static("event_authority"));
        }
        if authority.key() != *expected {
            return Err(Error::from(ErrorCode::ConstraintSeeds)
                .__tare_with_account_name_static("event_authority")
                .with_pubkeys((authority.key(), *expected)));
        }
        Ok(())
    }
}
#[cfg_attr(not(tare_sdk), inline(always))]
pub fn emit_cpi(
    program_id: &Pubkey,
    authority: AccountView,
    bump: u8,
    mut event_data: Vec<u8>,
) -> Result<()> {
    let length = event_data.len();
    event_data.resize(length + 8, 0);
    event_data.copy_within(..length, 8);
    event_data[..8].copy_from_slice(EVENT_IX_TAG_LE);
    crate::cpi::invoke_self(
        program_id,
        authority,
        &event_data,
        &[b"__event_authority", &[bump]],
    )
}
