//! AddToken instruction handler

use {
    crate::{
        error::PerpetualsError,
        state::{
            custody::{Custody, OracleParams, PricingParams},
            perpetuals::Perpetuals,
            pool::{Pool, PoolToken},
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token, TokenAccount},
};

#[derive(Accounts)]
pub struct AddToken<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    /// CHECK: empty PDA, authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = perpetuals.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    #[account(
        has_one = admin,
        seeds = [b"perpetuals"],
        bump = perpetuals.perpetuals_bump
    )]
    pub perpetuals: Box<Account<'info, Perpetuals>>,

    #[account(
        mut,
        realloc = Pool::LEN + (pool.tokens.len() + 1) * std::mem::size_of::<PoolToken>(),
        realloc::payer = admin,
        realloc::zero = false,
        seeds = [b"pool",
                 pool.name.as_bytes()],
        bump = pool.bump
    )]
    pub pool: Box<Account<'info, Pool>>,

    #[account(
        init_if_needed,
        payer = admin,
        space = Custody::LEN,
        seeds = [b"custody",
                 pool.key().as_ref(),
                 custody_token_mint.key().as_ref()],
        bump
    )]
    pub custody: Box<Account<'info, Custody>>,

    #[account(
        init_if_needed,
        payer = admin,
        token::mint = custody_token_mint,
        token::authority = transfer_authority,
        seeds = [b"custody_token_account",
                 pool.key().as_ref(),
                 custody_token_mint.key().as_ref()],
        bump
    )]
    pub custody_token_account: Box<Account<'info, TokenAccount>>,

    #[account()]
    pub custody_token_mint: Box<Account<'info, Mint>>,

    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    rent: Sysvar<'info, Rent>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct AddTokenParams {
    pub oracle: OracleParams,
    pub pricing: PricingParams,
}

pub fn add_token(ctx: Context<AddToken>, params: &AddTokenParams) -> Result<()> {
    // update pool data
    let pool = ctx.accounts.pool.as_mut();
    if let Ok(idx) = pool.get_token_id(&ctx.accounts.custody.key()) {
        pool.tokens[idx].custody = ctx.accounts.custody.key();
    } else {
        pool.tokens.push(PoolToken {
            custody: ctx.accounts.custody.key(),
        });
    }

    // record custody data
    let custody = ctx.accounts.custody.as_mut();
    custody.token_account = ctx.accounts.custody_token_account.key();
    custody.mint = ctx.accounts.custody_token_mint.key();
    custody.decimals = ctx.accounts.custody_token_mint.decimals;
    custody.oracle = params.oracle;
    custody.pricing = params.pricing;
    custody.bump = *ctx.bumps.get("custody").ok_or(ProgramError::InvalidSeeds)?;
    custody.token_account_bump = *ctx
        .bumps
        .get("custody_token_account")
        .ok_or(ProgramError::InvalidSeeds)?;

    if !custody.validate() {
        err!(PerpetualsError::InvalidTokenConfig)
    } else {
        Ok(())
    }
}
