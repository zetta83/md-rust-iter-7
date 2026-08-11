use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct OracleState {
    pub admin: Pubkey,
    pub price: u64,
    pub decimals: u8,
    pub last_updated_slot: u64,
    pub bump: u8,
}
