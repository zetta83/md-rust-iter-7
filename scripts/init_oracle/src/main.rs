use std::env;
use std::path::PathBuf;
use std::time::Duration;

use anchor_lang::solana_program::{instruction::Instruction, system_program};
use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use anyhow::{ensure, Context, Result};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::Transaction;

const DEFAULT_RPC_URL: &str = "http://127.0.0.1:8899";
const DEFAULT_INITIALIZE_PRICE: u64 = 1_000_000;

// Right after `anchor program deploy`, the validator needs a while before the program
// is actually invokable — RPC calls fail with "Attempt to load a program that does not
// exist" or "Program is not deployed" in the meantime (observed up to ~15s on a local
// solana-test-validator). `make init` typically runs right after `make deploy`, so
// retry through this warm-up window instead of failing outright.
const DEPLOY_WARMUP_RETRIES: u32 = 30;
const DEPLOY_WARMUP_RETRY_DELAY: Duration = Duration::from_millis(1000);

#[tokio::main]
async fn main() -> Result<()> {
    let rpc_url = env::var("RPC_URL").unwrap_or_else(|_| DEFAULT_RPC_URL.to_string());
    let program_id = match env::var("PROGRAM_ID") {
        Ok(raw) => raw.parse().context("invalid PROGRAM_ID")?,
        Err(_) => token_factory::id(),
    };
    let initialize_price: u64 = match env::var("INITIALIZE_PRICE") {
        Ok(raw) => raw.parse().context("invalid INITIALIZE_PRICE")?,
        Err(_) => DEFAULT_INITIALIZE_PRICE,
    };
    let keypair_path = admin_keypair_path()?;

    let admin = solana_keypair::read_keypair_file(&keypair_path).map_err(|e| {
        anyhow::anyhow!("failed to read keypair at {}: {e}", keypair_path.display())
    })?;

    let rpc_client = RpcClient::new(rpc_url);
    let (oracle, _bump) =
        Pubkey::find_program_address(&[token_factory::constants::ORACLE_SEED], &program_id);

    if let Some(account) = rpc_client
        .get_account_with_commitment(&oracle, CommitmentConfig::confirmed())
        .await
        .context("failed to fetch oracle account")?
        .value
    {
        ensure!(
            account.owner == program_id,
            "oracle PDA {oracle} is already in use by another program ({}), expected {program_id}",
            account.owner
        );
        let state =
            token_factory::state::OracleState::try_deserialize(&mut account.data.as_slice())
                .context("oracle PDA exists but failed to decode as OracleState")?;
        eprintln!(
            "oracle already initialized: admin={} price={} decimals={}",
            state.admin, state.price, state.decimals
        );
        println!("ORACLE_STATE_PUBKEY={oracle}");
        return Ok(());
    }

    let instruction = Instruction::new_with_bytes(
        program_id,
        &token_factory::instruction::Initialize { initialize_price }.data(),
        token_factory::accounts::InitializeOracle {
            admin: admin.pubkey(),
            oracle,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .context("failed to fetch latest blockhash")?;
    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&admin.pubkey()),
        &[&admin],
        blockhash,
    );

    let signature = send_with_warmup_retry(&rpc_client, &tx)
        .await
        .context("failed to send initialize transaction")?;

    eprintln!(
        "initialized oracle: admin={} price={initialize_price} tx={signature}",
        admin.pubkey()
    );
    println!("ORACLE_STATE_PUBKEY={oracle}");

    Ok(())
}

async fn send_with_warmup_retry(
    rpc_client: &RpcClient,
    tx: &Transaction,
) -> Result<solana_sdk::signature::Signature> {
    for attempt in 1..=DEPLOY_WARMUP_RETRIES {
        match rpc_client.send_and_confirm_transaction(tx).await {
            Ok(signature) => return Ok(signature),
            Err(err) if attempt < DEPLOY_WARMUP_RETRIES && is_program_warmup_error(&err) => {
                eprintln!(
                    "program not yet invokable (attempt {attempt}/{DEPLOY_WARMUP_RETRIES}), retrying: {err}"
                );
                tokio::time::sleep(DEPLOY_WARMUP_RETRY_DELAY).await;
            }
            Err(err) => return Err(err.into()),
        }
    }
    unreachable!("loop always returns on the last attempt")
}

fn is_program_warmup_error(err: &solana_client::client_error::ClientError) -> bool {
    let msg = err.to_string();
    msg.contains("Attempt to load a program that does not exist")
        || msg.contains("Program is not deployed")
}

fn admin_keypair_path() -> Result<PathBuf> {
    if let Ok(raw) = env::var("ADMIN_KEYPAIR_PATH") {
        return Ok(PathBuf::from(raw));
    }
    let home = env::var("HOME").context("HOME is not set; pass ADMIN_KEYPAIR_PATH explicitly")?;
    Ok(PathBuf::from(home).join(".config/solana/id.json"))
}
