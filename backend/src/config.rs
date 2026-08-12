use std::env;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use solana_sdk::pubkey::Pubkey;

pub struct PriceFeedConfig {
    pub admin_keypair_path: PathBuf,
    pub interval: Duration,
}

pub struct Config {
    pub rpc_url: String,
    pub program_id: Pubkey,
    pub oracle_state_pubkey: Pubkey,
    pub port: u16,
    pub price_feed: Option<PriceFeedConfig>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let rpc_url = env::var("RPC_URL").context("RPC_URL is not set")?;

        let program_id = parse_pubkey_env("PROGRAM_ID")?;
        let oracle_state_pubkey = parse_pubkey_env("ORACLE_STATE_PUBKEY")?;

        let port = match env::var("PORT") {
            Ok(raw) => raw
                .parse()
                .with_context(|| format!("invalid PORT: {raw}"))?,
            Err(_) => 8080,
        };

        let price_feed = match env::var("ADMIN_KEYPAIR_PATH") {
            Ok(path) => {
                let interval_secs: u64 = match env::var("PRICE_FEED_INTERVAL_SECS") {
                    Ok(raw) => raw
                        .parse()
                        .with_context(|| format!("invalid PRICE_FEED_INTERVAL_SECS: {raw}"))?,
                    Err(_) => 30,
                };
                Some(PriceFeedConfig {
                    admin_keypair_path: PathBuf::from(path),
                    interval: Duration::from_secs(interval_secs),
                })
            }
            Err(_) => None,
        };

        Ok(Self {
            rpc_url,
            program_id,
            oracle_state_pubkey,
            port,
            price_feed,
        })
    }
}

fn parse_pubkey_env(key: &str) -> Result<Pubkey> {
    let raw = env::var(key).with_context(|| format!("{key} is not set"))?;
    raw.parse().with_context(|| format!("invalid {key}: {raw}"))
}
