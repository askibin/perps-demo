use {
    crate::{
        error::PerpetualsError,
        math,
        state::{custody::Custody, oracle::OraclePrice},
    },
    anchor_lang::prelude::*,
};

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct PoolToken {
    pub custody: Pubkey,
}

#[account]
#[derive(Default, Debug)]
pub struct Pool {
    pub name: String,
    pub tokens: Vec<PoolToken>,
    pub aum_usd: u128,

    pub bump: u8,
    pub lp_token_bump: u8,
}

/// Token Pool
impl Pool {
    pub const LEN: usize = 8 + std::mem::size_of::<Pool>();

    pub fn get_token_id(&self, custody: &Pubkey) -> Result<usize> {
        self.tokens
            .iter()
            .position(|&k| k.custody == *custody)
            .ok_or(PerpetualsError::UnsupportedToken.into())
    }

    pub fn get_swap_price(
        &self,
        token_in_price: &OraclePrice,
        token_out_price: &OraclePrice,
    ) -> Result<OraclePrice> {
        token_in_price.checked_div(token_out_price)
    }

    pub fn get_swap_amount(
        &self,
        token_in_price: &OraclePrice,
        token_out_price: &OraclePrice,
        custody_in: &Custody,
        custody_out: &Custody,
        amount_in: u64,
    ) -> Result<u64> {
        let swap_price = self.get_swap_price(token_in_price, token_out_price)?;
        math::checked_decimal_mul(
            amount_in,
            -(custody_in.decimals as i32),
            swap_price.price,
            swap_price.exponent,
            -(custody_out.decimals as i32),
        )
    }

    pub fn lock_funds(&self, amount: u64, custody: &mut Custody) -> Result<()> {
        custody.assets.locked = math::checked_add(custody.assets.locked, amount)?;
        if custody.assets.owned < custody.assets.locked {
            Err(ProgramError::InsufficientFunds.into())
        } else {
            Ok(())
        }
    }

    pub fn unlock_funds(&self, amount: u64, custody: &mut Custody) -> Result<()> {
        if amount > custody.assets.locked {
            custody.assets.locked = 0;
        } else {
            custody.assets.locked = math::checked_sub(custody.assets.locked, amount)?;
        }
        Ok(())
    }

    pub fn get_assets_under_management_usd(
        &self,
        accounts: &[AccountInfo],
        curtime: i64,
    ) -> Result<u128> {
        let mut pool_amount_usd: u128 = 0;
        for (idx, &token) in self.tokens.iter().enumerate() {
            let oracle_idx = idx + self.tokens.len();
            if oracle_idx >= accounts.len() {
                return Err(ProgramError::NotEnoughAccountKeys.into());
            }
            require_keys_eq!(accounts[idx].key(), token.custody);
            let custody = Account::<Custody>::try_from(&accounts[idx])?;
            require_keys_eq!(accounts[oracle_idx].key(), custody.oracle.oracle_account);
            let token_price = OraclePrice::new_from_oracle(
                custody.oracle.oracle_type,
                &accounts[oracle_idx],
                custody.oracle.max_price_error,
                custody.oracle.max_price_age_sec,
                curtime,
            )?;
            let token_amount_usd =
                token_price.get_asset_amount_usd(custody.assets.owned, custody.decimals)?;

            pool_amount_usd = math::checked_add(pool_amount_usd, token_amount_usd as u128)?;
        }
        Ok(pool_amount_usd)
    }
}
