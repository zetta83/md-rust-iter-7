use anchor_lang::prelude::*;

use crate::{constants::*, error::ErrorCode, state::OracleState};

#[derive(Accounts)]
#[instruction(initialize_price: u64)]
pub struct InitializeOracle<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = 8 + OracleState::INIT_SPACE,
        seeds = [ORACLE_SEED],
        bump
    )]
    pub oracle: Account<'info, OracleState>,
    pub system_program: Program<'info, System>,
}

pub fn handle_initialize_oracle(
    ctx: Context<InitializeOracle>,
    initialize_price: u64,
) -> Result<()> {
    require!(initialize_price > 0, ErrorCode::InvalidPrice);
    let oracle = &mut ctx.accounts.oracle;
    oracle.admin = ctx.accounts.admin.key();
    oracle.price = initialize_price;
    oracle.decimals = EXPECTED_DECIMALS;
    oracle.last_updated_slot = Clock::get()?.slot;
    oracle.bump = ctx.bumps.oracle;
    Ok(())
}
