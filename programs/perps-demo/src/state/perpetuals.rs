use {
    anchor_lang::prelude::*,
    anchor_spl::token::{Burn, MintTo, Transfer},
};

#[account]
#[derive(Default, Debug)]
pub struct Perpetuals {
    pub admin: Pubkey,
    pub pools: Vec<Pubkey>,

    pub transfer_authority_bump: u8,
    pub perpetuals_bump: u8,
}

impl anchor_lang::Id for Perpetuals {
    fn id() -> Pubkey {
        crate::ID
    }
}

impl Perpetuals {
    pub const LEN: usize = 8 + std::mem::size_of::<Perpetuals>();
    pub const BPS_DECIMALS: u8 = 4;
    pub const BPS_POWER: u128 = 10i64.pow(Self::BPS_DECIMALS as u32) as u128;
    pub const PRICE_DECIMALS: u8 = 6;
    pub const USD_DECIMALS: u8 = 6;
    pub const LP_DECIMALS: u8 = Self::USD_DECIMALS;

    pub fn get_time(&self) -> Result<i64> {
        let time = solana_program::sysvar::clock::Clock::get()?.unix_timestamp;
        if time > 0 {
            Ok(time)
        } else {
            Err(ProgramError::InvalidAccountData.into())
        }
    }

    pub fn transfer_tokens<'info>(
        &self,
        from: AccountInfo<'info>,
        to: AccountInfo<'info>,
        authority: AccountInfo<'info>,
        token_program: AccountInfo<'info>,
        amount: u64,
    ) -> Result<()> {
        let authority_seeds: &[&[&[u8]]] =
            &[&[b"transfer_authority", &[self.transfer_authority_bump]]];

        let context = CpiContext::new(
            token_program,
            Transfer {
                from,
                to,
                authority,
            },
        )
        .with_signer(authority_seeds);

        anchor_spl::token::transfer(context, amount)
    }

    pub fn transfer_tokens_from_user<'info>(
        &self,
        from: AccountInfo<'info>,
        to: AccountInfo<'info>,
        authority: AccountInfo<'info>,
        token_program: AccountInfo<'info>,
        amount: u64,
    ) -> Result<()> {
        let context = CpiContext::new(
            token_program,
            Transfer {
                from,
                to,
                authority,
            },
        );
        anchor_spl::token::transfer(context, amount)
    }

    pub fn mint_tokens<'info>(
        &self,
        mint: AccountInfo<'info>,
        to: AccountInfo<'info>,
        authority: AccountInfo<'info>,
        token_program: AccountInfo<'info>,
        amount: u64,
    ) -> Result<()> {
        let authority_seeds: &[&[&[u8]]] =
            &[&[b"transfer_authority", &[self.transfer_authority_bump]]];

        let context = CpiContext::new(
            token_program,
            MintTo {
                mint,
                to,
                authority,
            },
        )
        .with_signer(authority_seeds);

        anchor_spl::token::mint_to(context, amount)
    }

    pub fn burn_tokens<'info>(
        &self,
        mint: AccountInfo<'info>,
        from: AccountInfo<'info>,
        authority: AccountInfo<'info>,
        token_program: AccountInfo<'info>,
        amount: u64,
    ) -> Result<()> {
        let context = CpiContext::new(
            token_program,
            Burn {
                mint,
                from,
                authority,
            },
        );

        anchor_spl::token::burn(context, amount)
    }
}
