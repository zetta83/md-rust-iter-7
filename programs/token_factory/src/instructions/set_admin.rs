use crate::{OracleState, ORACLE_SEED};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(new_admin: Pubkey)]
pub struct SetAdmin<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [ORACLE_SEED],
        bump = oracle.bump,
        has_one = admin
    )]
    pub oracle: Account<'info, OracleState>,
}

pub fn handle_set_admin(ctx: Context<SetAdmin>, new_admin: Pubkey) -> Result<()> {
    let oracle = &mut ctx.accounts.oracle;
    oracle.admin = new_admin;
    Ok(())
}
