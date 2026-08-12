use crate::{constants::*, error::ErrorCode, state::OracleState};
use anchor_lang::prelude::*;
use anchor_lang::system_program;

#[derive(Accounts)]
pub struct WithdrawFees<'info> {
    #[account(seeds = [ORACLE_SEED], bump = oracle.bump, has_one = admin)]
    pub oracle: Account<'info, OracleState>,

    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(mut, seeds = [TREASURY_SEED], bump)]
    pub treasury: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_withdraw_fees(ctx: Context<WithdrawFees>, amount: u64) -> Result<()> {
    require!(amount > 0, ErrorCode::InvalidWithdrawAmount);
    require!(
        amount <= ctx.accounts.treasury.lamports(),
        ErrorCode::InsufficientTreasuryBalance
    );

    let signer_seeds: &[&[u8]] = &[TREASURY_SEED, &[ctx.bumps.treasury]];

    system_program::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.key(),
            system_program::Transfer {
                from: ctx.accounts.treasury.to_account_info(),
                to: ctx.accounts.admin.to_account_info(),
            },
            &[signer_seeds],
        ),
        amount,
    )?;

    Ok(())
}
