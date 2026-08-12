use anchor_lang::AccountDeserialize;
use serde::Serialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use token_factory::{OracleState, MAX_STALENESS_SLOTS};

use crate::error::AppError;

pub struct AppState {
    pub rpc_client: RpcClient,
    pub program_id: Pubkey,
    pub oracle_state_pubkey: Pubkey,
}

#[derive(Serialize)]
pub struct OracleResponse {
    pub admin: String,
    pub price: u64,
    pub decimals: u8,
    pub last_updated_slot: u64,
    pub is_stale: bool,
}

pub async fn fetch_oracle_state(state: &AppState) -> Result<OracleResponse, AppError> {
    let account = state
        .rpc_client
        .get_account_with_commitment(&state.oracle_state_pubkey, CommitmentConfig::confirmed())
        .await?
        .value
        .ok_or(AppError::OracleNotFound)?;

    if account.owner != state.program_id {
        return Err(AppError::OwnerMismatch);
    }

    let oracle = OracleState::try_deserialize(&mut account.data.as_slice())?;
    let current_slot = state.rpc_client.get_slot().await?;
    let is_stale = current_slot.saturating_sub(oracle.last_updated_slot) > MAX_STALENESS_SLOTS;

    Ok(OracleResponse {
        admin: oracle.admin.to_string(),
        price: oracle.price,
        decimals: oracle.decimals,
        last_updated_slot: oracle.last_updated_slot,
        is_stale,
    })
}
