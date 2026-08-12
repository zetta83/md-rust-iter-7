mod common;

use {
    anchor_lang::{
        prelude::Pubkey,
        solana_program::{instruction::Instruction, rent, system_program},
        AccountDeserialize, AccountSerialize, InstructionData, ToAccountMetas,
    },
    anchor_spl::{associated_token, token as anchor_token},
    common::assert_anchor_error,
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

struct TokenAccounts {
    mint: Keypair,
    mint_authority: Pubkey,
    payer_ata: Pubkey,
    treasury: Pubkey,
    oracle: Pubkey,
}

fn setup(oracle_price: u64) -> (LiteSVM, Pubkey, Keypair, TokenAccounts) {
    let program_id = token_factory::id();
    let payer = Keypair::new();
    let mint = Keypair::new();
    let mint_authority = Pubkey::find_program_address(
        &[token_factory::constants::TOKEN_SEED, mint.pubkey().as_ref()],
        &program_id,
    )
    .0;
    let payer_ata = associated_token::get_associated_token_address(&payer.pubkey(), &mint.pubkey());
    let treasury =
        Pubkey::find_program_address(&[token_factory::constants::TREASURY_SEED], &program_id).0;
    let oracle =
        Pubkey::find_program_address(&[token_factory::constants::ORACLE_SEED], &program_id).0;

    let mut svm = LiteSVM::new();
    let bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/token_factory.so"
    ));
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let accounts = TokenAccounts {
        mint,
        mint_authority,
        payer_ata,
        treasury,
        oracle,
    };

    let init_ix = Instruction::new_with_bytes(
        program_id,
        &token_factory::instruction::Initialize {
            initialize_price: oracle_price,
        }
        .data(),
        token_factory::accounts::InitializeOracle {
            admin: payer.pubkey(),
            oracle,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );
    send(&mut svm, &payer, &[], init_ix).unwrap();

    (svm, program_id, payer, accounts)
}

fn create_token_with_fee_instruction(
    program_id: Pubkey,
    payer: Pubkey,
    accounts: &TokenAccounts,
    decimals: u8,
    initial_supply: u64,
    fee_usd: u64,
) -> Instruction {
    Instruction::new_with_bytes(
        program_id,
        &token_factory::instruction::CreateTokenWithFee {
            decimals,
            initial_supply,
            fee_usd,
        }
        .data(),
        token_factory::accounts::CreateTokenWithFee {
            mint: accounts.mint.pubkey(),
            payer_ata: accounts.payer_ata,
            mint_authority: accounts.mint_authority,
            payer,
            treasury: accounts.treasury,
            oracle: accounts.oracle,
            system_program: system_program::ID,
            token_program: anchor_token::ID,
            associated_token_program: associated_token::ID,
            rent: rent::ID,
        }
        .to_account_metas(None),
    )
}

fn send(
    svm: &mut LiteSVM,
    payer: &Keypair,
    extra_signers: &[&Keypair],
    instruction: Instruction,
) -> Result<(), litesvm::types::FailedTransactionMetadata> {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);
    let mut signers: Vec<&Keypair> = vec![payer];
    signers.extend_from_slice(extra_signers);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &signers).unwrap();
    svm.send_transaction(tx).map(|_| ())
}

fn read_mint(svm: &LiteSVM, mint: Pubkey) -> anchor_token::Mint {
    let account = svm.get_account(&mint).unwrap();
    let mut data: &[u8] = &account.data;
    anchor_token::Mint::try_deserialize(&mut data).unwrap()
}

fn read_token_account(svm: &LiteSVM, token_account: Pubkey) -> anchor_token::TokenAccount {
    let account = svm.get_account(&token_account).unwrap();
    let mut data: &[u8] = &account.data;
    anchor_token::TokenAccount::try_deserialize(&mut data).unwrap()
}

fn read_oracle(svm: &LiteSVM, oracle: Pubkey) -> token_factory::state::OracleState {
    let account = svm.get_account(&oracle).unwrap();
    let mut data: &[u8] = &account.data;
    token_factory::state::OracleState::try_deserialize(&mut data).unwrap()
}

fn corrupt_oracle_decimals(svm: &mut LiteSVM, oracle: Pubkey, bad_decimals: u8) {
    let mut account = svm.get_account(&oracle).unwrap();
    let mut state = token_factory::state::OracleState::try_deserialize(&mut account.data.as_slice())
        .unwrap();
    state.decimals = bad_decimals;

    let mut data = Vec::new();
    state.try_serialize(&mut data).unwrap();
    account.data = data;
    svm.set_account(oracle, account).unwrap();
}

fn expected_fee_lamports(fee_usd: u64, price: u64) -> u64 {
    let numerator = fee_usd as u128 * token_factory::constants::LAMPORTS_PER_SOL_U64 as u128;
    (numerator / price as u128) as u64
}

#[test]
fn test_create_token_with_fee() {
    let oracle_price = 1_000_000;
    let (mut svm, program_id, payer, accounts) = setup(oracle_price);
    let decimals = token_factory::constants::EXPECTED_DECIMALS;
    let initial_supply: u64 = 1_000;
    let fee_usd: u64 = 3_000_000;

    let instruction = create_token_with_fee_instruction(
        program_id,
        payer.pubkey(),
        &accounts,
        decimals,
        initial_supply,
        fee_usd,
    );
    let res = send(&mut svm, &payer, &[&accounts.mint], instruction);
    assert!(res.is_ok(), "tx failed: {:?}", res.err());

    let mint_state = read_mint(&svm, accounts.mint.pubkey());
    assert_eq!(mint_state.decimals, decimals);
    assert_eq!(mint_state.mint_authority.unwrap(), accounts.mint_authority);

    let expected_amount_raw = initial_supply * 10u64.pow(decimals as u32);
    assert_eq!(mint_state.supply, expected_amount_raw);

    let payer_ata_state = read_token_account(&svm, accounts.payer_ata);
    assert_eq!(payer_ata_state.mint, accounts.mint.pubkey());
    assert_eq!(payer_ata_state.owner, payer.pubkey());
    assert_eq!(payer_ata_state.amount, expected_amount_raw);

    let expected_fee = expected_fee_lamports(fee_usd, oracle_price);
    let treasury_account = svm.get_account(&accounts.treasury).unwrap();
    assert_eq!(treasury_account.lamports, expected_fee);
}

#[test]
fn test_create_token_with_fee_bad_decimals_fails() {
    let (mut svm, program_id, payer, accounts) = setup(1_000_000);
    let bad_decimals = token_factory::constants::EXPECTED_DECIMALS + 1;

    let instruction = create_token_with_fee_instruction(
        program_id,
        payer.pubkey(),
        &accounts,
        bad_decimals,
        1_000,
        3_000_000,
    );
    let res = send(&mut svm, &payer, &[&accounts.mint], instruction);

    assert_anchor_error(
        &res.unwrap_err(),
        token_factory::error::ErrorCode::BadTokenDecimals,
    );
    assert!(svm.get_account(&accounts.mint.pubkey()).is_none());
    assert!(svm.get_account(&accounts.treasury).is_none());
}

#[test]
fn test_create_token_with_fee_stale_oracle_fails() {
    let (mut svm, program_id, payer, accounts) = setup(1_000_000);
    let decimals = token_factory::constants::EXPECTED_DECIMALS;

    let last_updated_slot = read_oracle(&svm, accounts.oracle).last_updated_slot;
    svm.warp_to_slot(last_updated_slot + token_factory::constants::MAX_STALENESS_SLOTS + 1);

    let instruction = create_token_with_fee_instruction(
        program_id,
        payer.pubkey(),
        &accounts,
        decimals,
        1_000,
        3_000_000,
    );
    let res = send(&mut svm, &payer, &[&accounts.mint], instruction);

    assert_anchor_error(&res.unwrap_err(), token_factory::error::ErrorCode::StaleOracle);
    assert!(svm.get_account(&accounts.mint.pubkey()).is_none());
    assert!(svm.get_account(&accounts.treasury).is_none());
}

#[test]
fn test_create_token_with_fee_bad_oracle_decimals_fails() {
    let (mut svm, program_id, payer, accounts) = setup(1_000_000);
    let decimals = token_factory::constants::EXPECTED_DECIMALS;
    corrupt_oracle_decimals(&mut svm, accounts.oracle, decimals + 1);

    let instruction = create_token_with_fee_instruction(
        program_id,
        payer.pubkey(),
        &accounts,
        decimals,
        1_000,
        3_000_000,
    );
    let res = send(&mut svm, &payer, &[&accounts.mint], instruction);

    assert_anchor_error(
        &res.unwrap_err(),
        token_factory::error::ErrorCode::BadOracleDecimals,
    );
    assert!(svm.get_account(&accounts.mint.pubkey()).is_none());
    assert!(svm.get_account(&accounts.treasury).is_none());
}

#[test]
fn test_create_token_with_fee_zero_fee_usd_fails() {
    let (mut svm, program_id, payer, accounts) = setup(1_000_000);
    let decimals = token_factory::constants::EXPECTED_DECIMALS;

    let instruction = create_token_with_fee_instruction(
        program_id,
        payer.pubkey(),
        &accounts,
        decimals,
        1_000,
        0,
    );
    let res = send(&mut svm, &payer, &[&accounts.mint], instruction);

    assert_anchor_error(
        &res.unwrap_err(),
        token_factory::error::ErrorCode::BadTokenFeeUsd,
    );
    assert!(svm.get_account(&accounts.mint.pubkey()).is_none());
    assert!(svm.get_account(&accounts.treasury).is_none());
}

#[test]
fn test_create_token_with_fee_supply_overflow_fails() {
    let (mut svm, program_id, payer, accounts) = setup(1_000_000);
    let decimals = token_factory::constants::EXPECTED_DECIMALS;

    let instruction = create_token_with_fee_instruction(
        program_id,
        payer.pubkey(),
        &accounts,
        decimals,
        u64::MAX,
        3_000_000,
    );
    let res = send(&mut svm, &payer, &[&accounts.mint], instruction);

    assert_anchor_error(
        &res.unwrap_err(),
        token_factory::error::ErrorCode::MathOverflow,
    );
    assert!(svm.get_account(&accounts.mint.pubkey()).is_none());
    assert!(svm.get_account(&accounts.treasury).is_none());
}
