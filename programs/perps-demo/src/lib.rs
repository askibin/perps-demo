//! Perpetuals demo program entrypoint

#![allow(clippy::result_large_err)]

mod error;
mod instructions;
mod math;
mod state;

use {anchor_lang::prelude::*, instructions::*};

declare_id!("FAXYuthnTA4m7bSivEoxFeNUCMACD5RTxKN99WNUNjAg");

#[program]
pub mod perps_demo {
    use super::*;

    // admin instructions
    pub fn init(ctx: Context<Init>, params: InitParams) -> Result<()> {
        instructions::init(ctx, &params)
    }

    pub fn add_pool(ctx: Context<AddPool>, params: AddPoolParams) -> Result<()> {
        instructions::add_pool(ctx, &params)
    }

    pub fn add_token(ctx: Context<AddToken>, params: AddTokenParams) -> Result<()> {
        instructions::add_token(ctx, &params)
    }

    // test instructions

    pub fn set_test_oracle_price(
        ctx: Context<SetTestOraclePrice>,
        params: SetTestOraclePriceParams,
    ) -> Result<()> {
        instructions::set_test_oracle_price(ctx, &params)
    }

    // public instructions

    pub fn swap(ctx: Context<Swap>, params: SwapParams) -> Result<()> {
        instructions::swap(ctx, &params)
    }

    pub fn add_liquidity(ctx: Context<AddLiquidity>, params: AddLiquidityParams) -> Result<()> {
        instructions::add_liquidity(ctx, &params)
    }

    pub fn remove_liquidity(
        ctx: Context<RemoveLiquidity>,
        params: RemoveLiquidityParams,
    ) -> Result<()> {
        instructions::remove_liquidity(ctx, &params)
    }

    pub fn open_position(ctx: Context<OpenPosition>, params: OpenPositionParams) -> Result<()> {
        instructions::open_position(ctx, &params)
    }

    pub fn close_position(ctx: Context<ClosePosition>, params: ClosePositionParams) -> Result<()> {
        instructions::close_position(ctx, &params)
    }

    pub fn liquidate(ctx: Context<Liquidate>, params: LiquidateParams) -> Result<()> {
        instructions::liquidate(ctx, &params)
    }
}
