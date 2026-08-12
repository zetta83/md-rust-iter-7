use std::sync::Arc;
use std::time::Duration;

use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{InstructionData, ToAccountMetas};
use rand::Rng;
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_transaction::Transaction;
use tokio::time::interval;

use crate::oracle::{fetch_oracle_state, AppState};

const MAX_DRIFT_BPS: i64 = 100;

pub fn spawn(state: Arc<AppState>, admin: Keypair, period: Duration) {
    tokio::spawn(async move {
        let mut ticker = interval(period);
        loop {
            ticker.tick().await;
            if let Err(err) = tick(&state, &admin).await {
                tracing::warn!("price feed tick failed: {err:#}");
            }
        }
    });
}

async fn tick(state: &AppState, admin: &Keypair) -> anyhow::Result<()> {
    let current = fetch_oracle_state(state).await?;
    let new_price = drifted_price(current.price);

    let instruction = Instruction::new_with_bytes(
        state.program_id,
        &token_factory::instruction::UpdatePrice { new_price }.data(),
        token_factory::accounts::UpdatePrice {
            oracle: state.oracle_state_pubkey,
            admin: admin.pubkey(),
        }
        .to_account_metas(None),
    );

    let blockhash = state.rpc_client.get_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&admin.pubkey()),
        &[admin],
        blockhash,
    );

    let signature = state.rpc_client.send_and_confirm_transaction(&tx).await?;
    tracing::info!(price = new_price, %signature, "price feed: updated oracle price");

    Ok(())
}

fn drifted_price(price: u64) -> u64 {
    let drift_bps = rand::rng().random_range(-MAX_DRIFT_BPS..=MAX_DRIFT_BPS);
    let delta = (price as i128 * drift_bps as i128) / 10_000;
    (price as i128 + delta).clamp(1, u64::MAX as i128) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program_allowed_range(price: u64) -> (u64, u64) {
        (price.saturating_mul(8) / 10, price.saturating_mul(12) / 10)
    }

    #[test]
    fn drift_always_stays_within_program_bounds() {
        for price in [1u64, 2, 1_000, 1_000_000, u64::MAX / 20] {
            let (min, max) = program_allowed_range(price);
            for _ in 0..1000 {
                let drifted = drifted_price(price);
                assert!(
                    drifted >= min && drifted <= max,
                    "price={price} drifted={drifted} allowed=[{min}, {max}]"
                );
            }
        }
    }
}
