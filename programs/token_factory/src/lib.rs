pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("9VptbChe2mPw3fkmcnKxCShNmagw4F8YvNX3BzSrPKQF");

#[program]
pub mod token_factory {
    use super::*;

    pub fn initialize(ctx: Context<InitializeOracle>, initialize_price: u64) -> Result<()> {
        crate::instructions::initialize::handle_initialize_oracle(ctx, initialize_price)
    }

    pub fn update_price(ctx: Context<UpdatePrice>, new_price: u64) -> Result<()> {
        crate::instructions::update_price::handle_update_price(ctx, new_price)
    }

    pub fn get_price(ctx: Context<GetPrice>) -> Result<()> {
        crate::instructions::update_price::handle_get_price(ctx)
    }

    pub fn set_admin(ctx: Context<SetAdmin>, new_admin: Pubkey) -> Result<()> {
        crate::instructions::set_admin::handle_set_admin(ctx, new_admin)
    }

    pub fn create_token(
        ctx: Context<CreateToken>,
        decimals: u8,
        initial_supply: u64,
    ) -> Result<()> {
        crate::instructions::create_token::handle_create_token(ctx, decimals, initial_supply)
    }

    pub fn create_token_with_fee(
        ctx: Context<CreateTokenWithFee>,
        decimals: u8,
        initial_supply: u64,
        fee_usd: u64,
    ) -> Result<()> {
        crate::instructions::create_token::handle_create_token_with_fee(
            ctx,
            decimals,
            initial_supply,
            fee_usd,
        )
    }

    pub fn withdraw_fees(ctx: Context<WithdrawFees>, amount: u64) -> Result<()> {
        crate::instructions::withdraw_fees::handle_withdraw_fees(ctx, amount)
    }
}
