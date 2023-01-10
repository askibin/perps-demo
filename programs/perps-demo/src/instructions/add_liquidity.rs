//! AddLiquidity instruction handler

use {
    crate::{
        math,
        state::{custody::Custody, oracle::OraclePrice, perpetuals::Perpetuals, pool::Pool},
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token, TokenAccount},
    solana_program::program_error::ProgramError,
};

#[derive(Accounts)]
#[instruction(params: AddLiquidityParams)]
pub struct AddLiquidity<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        constraint = funding_account.mint == custody.mint,
        has_one = owner
    )]
    pub funding_account: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = lp_token_account.mint == lp_token_mint.key(),
        has_one = owner
    )]
    pub lp_token_account: Box<Account<'info, TokenAccount>>,

    /// CHECK: empty PDA, authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = perpetuals.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    #[account(
        seeds = [b"perpetuals"],
        bump = perpetuals.perpetuals_bump
    )]
    pub perpetuals: Box<Account<'info, Perpetuals>>,

    #[account(
        mut,
        seeds = [b"pool",
                 pool.name.as_bytes()],
        bump = pool.bump
    )]
    pub pool: Box<Account<'info, Pool>>,

    #[account(
        mut,
        seeds = [b"custody",
                 pool.key().as_ref(),
                 custody.mint.as_ref()],
        bump = custody.bump
    )]
    pub custody: Box<Account<'info, Custody>>,

    /// CHECK: oracle account for the receiving token
    #[account(
        constraint = custody_oracle_account.key() == custody.oracle.oracle_account
    )]
    pub custody_oracle_account: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [b"custody_token_account",
                 pool.key().as_ref(),
                 custody.mint.as_ref()],
        bump = custody.token_account_bump
    )]
    pub custody_token_account: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [b"lp_token_mint",
                 pool.key().as_ref()],
        bump = pool.lp_token_bump
    )]
    pub lp_token_mint: Box<Account<'info, Mint>>,

    token_program: Program<'info, Token>,
    // remaining accounts:
    //   pool.tokens.len() custody accounts (read-only, unsigned)
    //   pool.tokens.len() custody oracles (read-only, unsigned)
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct AddLiquidityParams {
    amount: u64,
}

pub fn add_liquidity(ctx: Context<AddLiquidity>, params: &AddLiquidityParams) -> Result<()> {
    // validate inputs
    msg!("Validate inputs");
    if params.amount == 0 {
        return Err(ProgramError::InvalidArgument.into());
    }
    let perpetuals = ctx.accounts.perpetuals.as_mut();
    let custody = ctx.accounts.custody.as_mut();
    let pool = ctx.accounts.pool.as_mut();

    // transfer tokens
    msg!("Transfer tokens");
    perpetuals.transfer_tokens_from_user(
        ctx.accounts.funding_account.to_account_info(),
        ctx.accounts.custody_token_account.to_account_info(),
        ctx.accounts.owner.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        params.amount,
    )?;

    // compute assets under management
    msg!("Compute assets under management");
    let curtime = perpetuals.get_time()?;
    let token_price = OraclePrice::new_from_oracle(
        custody.oracle.oracle_type,
        &ctx.accounts.custody_oracle_account.to_account_info(),
        custody.oracle.max_price_error,
        custody.oracle.max_price_age_sec,
        curtime,
    )?;

    let pool_amount_usd = pool.get_assets_under_management_usd(ctx.remaining_accounts, curtime)?;
    msg!("Pool AUM USD: {}", pool_amount_usd);

    // compute amount of lp tokens to mint
    let token_amount_usd = token_price.get_asset_amount_usd(params.amount, custody.decimals)?;
    let lp_amount = if pool_amount_usd == 0 {
        token_amount_usd
    } else {
        math::checked_as_u64(math::checked_div(
            math::checked_mul(
                token_amount_usd as u128,
                ctx.accounts.lp_token_mint.supply as u128,
            )?,
            pool_amount_usd,
        )?)?
    };
    msg!("LP tokens to mint: {}", lp_amount);

    // mint lp tokens
    perpetuals.mint_tokens(
        ctx.accounts.lp_token_mint.to_account_info(),
        ctx.accounts.lp_token_account.to_account_info(),
        ctx.accounts.transfer_authority.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        lp_amount,
    )?;

    // update custody stats
    msg!("Update custody stats");
    custody.assets.owned = math::checked_add(custody.assets.owned, params.amount)?;

    // update pool stats
    msg!("Update pool stats");
    pool.aum_usd = pool_amount_usd;

    Ok(())
}
