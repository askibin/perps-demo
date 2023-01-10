//! Swap instruction handler

use {
    crate::{
        error::PerpetualsError,
        math,
        state::{custody::Custody, oracle::OraclePrice, perpetuals::Perpetuals, pool::Pool},
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Token, TokenAccount},
    solana_program::program_error::ProgramError,
};

#[derive(Accounts)]
#[instruction(params: SwapParams)]
pub struct Swap<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        constraint = funding_account.mint == receiving_custody.mint,
        has_one = owner
    )]
    pub funding_account: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = receiving_account.mint == dispensing_custody.mint,
        has_one = owner
    )]
    pub receiving_account: Box<Account<'info, TokenAccount>>,

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
                 receiving_custody.mint.as_ref()],
        bump = receiving_custody.bump
    )]
    pub receiving_custody: Box<Account<'info, Custody>>,

    /// CHECK: oracle account for the received token
    #[account(
        constraint = receiving_custody_oracle_account.key() == receiving_custody.oracle.oracle_account
    )]
    pub receiving_custody_oracle_account: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [b"custody_token_account",
                 pool.key().as_ref(),
                 receiving_custody.mint.as_ref()],
        bump = receiving_custody.token_account_bump
    )]
    pub receiving_custody_token_account: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [b"custody",
                 pool.key().as_ref(),
                 dispensing_custody.mint.as_ref()],
        bump = dispensing_custody.bump
    )]
    pub dispensing_custody: Box<Account<'info, Custody>>,

    /// CHECK: oracle account for the returned token
    #[account(
        constraint = dispensing_custody_oracle_account.key() == dispensing_custody.oracle.oracle_account
    )]
    pub dispensing_custody_oracle_account: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [b"custody_token_account",
                 pool.key().as_ref(),
                 dispensing_custody.mint.as_ref()],
        bump = dispensing_custody.token_account_bump
    )]
    pub dispensing_custody_token_account: Box<Account<'info, TokenAccount>>,

    token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct SwapParams {
    amount_in: u64,
    min_amount_out: u64,
}

pub fn swap(ctx: Context<Swap>, params: &SwapParams) -> Result<()> {
    // validate inputs
    msg!("Validate inputs");
    if params.amount_in == 0 {
        return Err(ProgramError::InvalidArgument.into());
    }
    let perpetuals = ctx.accounts.perpetuals.as_mut();
    let pool = ctx.accounts.pool.as_mut();
    let receiving_custody = ctx.accounts.receiving_custody.as_mut();
    let dispensing_custody = ctx.accounts.dispensing_custody.as_mut();
    require_keys_neq!(receiving_custody.key(), dispensing_custody.key());

    // compute token amount returned to the user
    let curtime = perpetuals.get_time()?;

    let received_token_price = OraclePrice::new_from_oracle(
        receiving_custody.oracle.oracle_type,
        &ctx.accounts
            .receiving_custody_oracle_account
            .to_account_info(),
        receiving_custody.oracle.max_price_error,
        receiving_custody.oracle.max_price_age_sec,
        curtime,
    )?;

    let dispensed_token_price = OraclePrice::new_from_oracle(
        dispensing_custody.oracle.oracle_type,
        &ctx.accounts
            .dispensing_custody_oracle_account
            .to_account_info(),
        dispensing_custody.oracle.max_price_error,
        dispensing_custody.oracle.max_price_age_sec,
        curtime,
    )?;

    let amount_out = pool.get_swap_amount(
        &received_token_price,
        &dispensed_token_price,
        receiving_custody,
        dispensing_custody,
        params.amount_in,
    )?;
    msg!("Amount out: {}", amount_out);

    // check returned amount
    require_gte!(
        amount_out,
        params.min_amount_out,
        PerpetualsError::InsufficientAmountReturned
    );

    // transfer tokens
    msg!("Transfer tokens");
    perpetuals.transfer_tokens_from_user(
        ctx.accounts.funding_account.to_account_info(),
        ctx.accounts
            .receiving_custody_token_account
            .to_account_info(),
        ctx.accounts.owner.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        params.amount_in,
    )?;

    perpetuals.transfer_tokens(
        ctx.accounts
            .dispensing_custody_token_account
            .to_account_info(),
        ctx.accounts.receiving_account.to_account_info(),
        ctx.accounts.transfer_authority.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        amount_out,
    )?;

    // update custody stats
    msg!("Update custody stats");
    receiving_custody.assets.owned =
        math::checked_add(receiving_custody.assets.owned, params.amount_in)?;
    dispensing_custody.assets.owned =
        math::checked_sub(dispensing_custody.assets.owned, amount_out)?;

    Ok(())
}
