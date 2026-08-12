mod config;
mod error;
mod oracle;
mod price_feed;
mod routes;

use std::sync::Arc;

use anyhow::Context;
use solana_client::nonblocking::rpc_client::RpcClient;
use tokio::net::TcpListener;

use config::Config;
use oracle::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = Config::from_env()?;
    let addr = format!("0.0.0.0:{}", config.port);

    let state = Arc::new(AppState {
        rpc_client: RpcClient::new(config.rpc_url),
        program_id: config.program_id,
        oracle_state_pubkey: config.oracle_state_pubkey,
    });

    if let Some(price_feed_config) = config.price_feed {
        let admin = solana_keypair::read_keypair_file(&price_feed_config.admin_keypair_path)
            .map_err(|e| anyhow::anyhow!("failed to read ADMIN_KEYPAIR_PATH: {e}"))?;
        tracing::info!(
            interval_secs = price_feed_config.interval.as_secs(),
            "price feed: enabled"
        );
        price_feed::spawn(state.clone(), admin, price_feed_config.interval);
    }

    let router = routes::build_router(state);
    let listener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("failed to bind {addr}"))?;
    tracing::info!("backend listening on {addr}");
    axum::serve(listener, router)
        .await
        .context("server error")?;

    Ok(())
}
