use anchor_lang::prelude::*;

use crate::{constants::*, error::ErrorCode, state::OracleState};

#[derive(Accounts)]
#[instruction(new_price: u64)]
pub struct UpdatePrice<'info> {
    #[account(
        mut,
        seeds = [ORACLE_SEED],
        bump = oracle.bump,
        has_one = admin
    )]
    pub oracle: Account<'info, OracleState>,
    pub admin: Signer<'info>,
}

#[event]
pub struct PriceUpdated {
    pub price: u64,
    pub slot: u64,
    pub updater: Pubkey,
}

pub fn handle_update_price(ctx: Context<UpdatePrice>, new_price: u64) -> Result<()> {
    require!(new_price > 0, ErrorCode::InvalidPrice);
    let oracle = &mut ctx.accounts.oracle;

    if oracle.price > 0 {
        let min = oracle.price.saturating_mul(8) / 10;
        let max = oracle.price.saturating_mul(12) / 10;
        require!(
            new_price >= min && new_price <= max,
            ErrorCode::PriceOutOfRange
        );
    }

    oracle.price = new_price;
    oracle.last_updated_slot = Clock::get()?.slot;

    emit!(PriceUpdated {
        price: oracle.price,
        slot: oracle.last_updated_slot,
        updater: oracle.admin,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct GetPrice<'info> {
    #[account(seeds = [ORACLE_SEED], bump = oracle.bump)]
    pub oracle: Account<'info, OracleState>,
}

pub fn handle_get_price(ctx: Context<GetPrice>) -> Result<()> {
    require_fresh(&ctx.accounts.oracle)
}

pub fn require_fresh(oracle: &OracleState) -> Result<()> {
    require!(
        Clock::get()?.slot.saturating_sub(oracle.last_updated_slot) <= MAX_STALENESS_SLOTS,
        ErrorCode::StaleOracle
    );
    Ok(())
}
