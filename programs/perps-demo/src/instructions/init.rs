//! Init instruction handler

use {
    crate::state::perpetuals::Perpetuals, anchor_lang::prelude::*, anchor_spl::token::Token,
    solana_program::program_error::ProgramError,
};

#[derive(Accounts)]
pub struct Init<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    /// CHECK: empty PDA, will be set as authority for token accounts
    #[account(
        init,
        payer = admin,
        space = 0,
        seeds = [b"transfer_authority"],
        bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    #[account(
        init,
        payer = admin,
        space = Perpetuals::LEN,
        seeds = [b"perpetuals"],
        bump
    )]
    pub perpetuals: Box<Account<'info, Perpetuals>>,

    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitParams {}

pub fn init(ctx: Context<Init>, _params: &InitParams) -> Result<()> {
    let perpetuals = ctx.accounts.perpetuals.as_mut();
    perpetuals.admin = ctx.accounts.admin.key();
    perpetuals.transfer_authority_bump = *ctx
        .bumps
        .get("transfer_authority")
        .ok_or(ProgramError::InvalidSeeds)?;
    perpetuals.perpetuals_bump = *ctx
        .bumps
        .get("perpetuals")
        .ok_or(ProgramError::InvalidSeeds)?;

    Ok(())
}
