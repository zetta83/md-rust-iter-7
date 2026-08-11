pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("45uvftn89ifwzk47sg2Q1W2HFjtDyMNtxFbHnokL8sKn");

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
}
