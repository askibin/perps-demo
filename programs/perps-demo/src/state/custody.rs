use {crate::state::oracle::OracleType, anchor_lang::prelude::*};

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct Assets {
    pub collateral: u64,
    // owned = total_assets - collateral
    pub owned: u64,
    // locked funds for pnl payoff
    pub locked: u64,
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct OracleParams {
    pub oracle_account: Pubkey,
    pub oracle_type: OracleType,
    pub max_price_error: u64,
    pub max_price_age_sec: u32,
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct PricingParams {
    // pricing params have implied BPS_DECIMALS decimals
    pub min_initial_leverage: u64,
    pub max_leverage: u64,
}

#[account]
#[derive(Default, Debug)]
pub struct Custody {
    pub token_account: Pubkey,
    pub mint: Pubkey,
    pub decimals: u8,
    pub oracle: OracleParams,
    pub pricing: PricingParams,
    pub assets: Assets,
    pub bump: u8,
    pub token_account_bump: u8,
}

impl OracleParams {
    pub fn validate(&self) -> bool {
        self.oracle_type == OracleType::None || self.oracle_account != Pubkey::default()
    }
}

impl PricingParams {
    pub fn validate(&self) -> bool {
        self.min_initial_leverage <= self.max_leverage
    }
}

impl Custody {
    pub const LEN: usize = 8 + std::mem::size_of::<Custody>();

    pub fn validate(&self) -> bool {
        self.token_account != Pubkey::default()
            && self.mint != Pubkey::default()
            && self.oracle.validate()
            && self.pricing.validate()
    }
}
