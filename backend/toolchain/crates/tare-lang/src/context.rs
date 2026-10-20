//! Contexts passed to generated cross-program invocation helpers.
use crate::{AccountInfo, ToAccountInfos, ToAccountMetas};
use solana_program::instruction::AccountMeta;

pub struct CpiContext<'a, 'b, 'c, 'info, T>
where
    T: ToAccountMetas + ToAccountInfos<'info>,
{
    pub accounts: T,
    pub remaining_accounts: Vec<AccountInfo<'info>>,
    pub program: AccountInfo<'info>,
    pub signer_seeds: &'a [&'b [&'c [u8]]],
}

impl<'a, 'b, 'c, 'info, T> CpiContext<'a, 'b, 'c, 'info, T>
where
    T: ToAccountMetas + ToAccountInfos<'info>,
{
    #[inline]
    pub fn new(program: AccountInfo<'info>, accounts: T) -> Self {
        Self {
            accounts,
            remaining_accounts: Vec::new(),
            program,
            signer_seeds: &[],
        }
    }
    #[inline]
    pub fn new_with_signer(
        program: AccountInfo<'info>,
        accounts: T,
        signer_seeds: &'a [&'b [&'c [u8]]],
    ) -> Self {
        Self {
            accounts,
            remaining_accounts: Vec::new(),
            program,
            signer_seeds,
        }
    }
    #[must_use]
    #[inline]
    pub fn with_signer(mut self, signer_seeds: &'a [&'b [&'c [u8]]]) -> Self {
        self.signer_seeds = signer_seeds;
        self
    }
    #[must_use]
    #[inline]
    pub fn with_remaining_accounts(mut self, remaining_accounts: Vec<AccountInfo<'info>>) -> Self {
        self.remaining_accounts = remaining_accounts;
        self
    }
}
impl<'info, T: ToAccountMetas + ToAccountInfos<'info>> ToAccountInfos<'info>
    for CpiContext<'_, '_, '_, 'info, T>
{
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        let mut infos = self.accounts.to_account_infos();
        infos.extend_from_slice(&self.remaining_accounts);
        infos.push(self.program.clone());
        infos
    }
}
impl<'info, T: ToAccountMetas + ToAccountInfos<'info>> ToAccountMetas
    for CpiContext<'_, '_, '_, 'info, T>
{
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
        let mut metas = self.accounts.to_account_metas(is_signer);
        metas.extend(self.remaining_accounts.iter().map(|account| {
            if account.is_writable {
                AccountMeta::new(*account.key, account.is_signer)
            } else {
                AccountMeta::new_readonly(*account.key, account.is_signer)
            }
        }));
        metas
    }
}
