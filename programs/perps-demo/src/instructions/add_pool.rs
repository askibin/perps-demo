//! AddPool instruction handler

use {
    crate::state::{perpetuals::Perpetuals, pool::Pool},
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token},
};

#[derive(Accounts)]
#[instruction(params: AddPoolParams)]
pub struct AddPool<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    /// CHECK: empty PDA, authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = perpetuals.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    #[account(
        mut,
        has_one = admin,
        realloc = Perpetuals::LEN + (perpetuals.pools.len() + 1) * std::mem::size_of::<Pubkey>(),
        realloc::payer = admin,
        realloc::zero = false,
        seeds = [b"perpetuals"],
        bump = perpetuals.perpetuals_bump
    )]
    pub perpetuals: Box<Account<'info, Perpetuals>>,

    #[account(
        init,
        payer = admin,
        space = Pool::LEN,
        seeds = [b"pool",
                 params.name.as_bytes()],
        bump
    )]
    pub pool: Box<Account<'info, Pool>>,

    #[account(
        init,
        payer = admin,
        mint::authority = transfer_authority,
        mint::freeze_authority = transfer_authority,
        mint::decimals = Perpetuals::LP_DECIMALS,
        seeds = [b"lp_token_mint",
                 pool.key().as_ref()],
        bump
    )]
    pub lp_token_mint: Box<Account<'info, Mint>>,

    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    rent: Sysvar<'info, Rent>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct AddPoolParams {
    pub name: String,
}

pub fn add_pool(ctx: Context<AddPool>, params: &AddPoolParams) -> Result<()> {
    // validate inputs
    if params.name.is_empty() {
        return Err(ProgramError::InvalidArgument.into());
    }

    // record pool data
    let perpetuals = ctx.accounts.perpetuals.as_mut();
    let pool = ctx.accounts.pool.as_mut();
    pool.name = params.name.clone();
    pool.bump = *ctx.bumps.get("pool").ok_or(ProgramError::InvalidSeeds)?;
    pool.lp_token_bump = *ctx
        .bumps
        .get("lp_token_mint")
        .ok_or(ProgramError::InvalidSeeds)?;

    perpetuals.pools.push(ctx.accounts.pool.key());

    Ok(())
}
