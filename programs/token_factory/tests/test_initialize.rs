
mod common;

use {
    anchor_lang::{
        prelude::Pubkey,
        solana_program::{instruction::Instruction, system_program},
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    common::assert_anchor_error,
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

fn setup() -> (LiteSVM, Pubkey, Keypair, Pubkey) {
    let program_id = token_factory::id();
    let admin = Keypair::new();
    let oracle =
        Pubkey::find_program_address(&[token_factory::constants::ORACLE_SEED], &program_id).0;
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/token_factory.so"
    ));
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&admin.pubkey(), 1_000_000_000).unwrap();
    (svm, program_id, admin, oracle)
}

fn initialize_instruction(program_id: Pubkey, admin: Pubkey, oracle: Pubkey, price: u64) -> Instruction {
    Instruction::new_with_bytes(
        program_id,
        &token_factory::instruction::Initialize {
            initialize_price: price,
        }
        .data(),
        token_factory::accounts::InitializeOracle {
            admin,
            oracle,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

fn update_price_instruction(program_id: Pubkey, admin: Pubkey, oracle: Pubkey, new_price: u64) -> Instruction {
    Instruction::new_with_bytes(
        program_id,
        &token_factory::instruction::UpdatePrice { new_price }.data(),
        token_factory::accounts::UpdatePrice { oracle, admin }.to_account_metas(None),
    )
}

fn get_price_instruction(program_id: Pubkey, oracle: Pubkey) -> Instruction {
    Instruction::new_with_bytes(
        program_id,
        &token_factory::instruction::GetPrice {}.data(),
        token_factory::accounts::GetPrice { oracle }.to_account_metas(None),
    )
}

fn set_admin_instruction(
    program_id: Pubkey,
    admin: Pubkey,
    oracle: Pubkey,
    new_admin: Pubkey,
) -> Instruction {
    Instruction::new_with_bytes(
        program_id,
        &token_factory::instruction::SetAdmin { new_admin }.data(),
        token_factory::accounts::SetAdmin { admin, oracle }.to_account_metas(None),
    )
}

fn send(svm: &mut LiteSVM, payer: &Keypair, instruction: Instruction) -> Result<(), litesvm::types::FailedTransactionMetadata> {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[payer]).unwrap();
    svm.send_transaction(tx).map(|_| ())
}

fn read_oracle(svm: &LiteSVM, oracle: Pubkey) -> token_factory::state::OracleState {
    let oracle_account = svm.get_account(&oracle).unwrap();
    let mut data: &[u8] = &oracle_account.data;
    token_factory::state::OracleState::try_deserialize(&mut data).unwrap()
}

#[test]
fn test_initialize() {
    let (mut svm, program_id, admin, oracle) = setup();
    let initialize_price: u64 = 1_000_000;

    let instruction = initialize_instruction(program_id, admin.pubkey(), oracle, initialize_price);
    let res = send(&mut svm, &admin, instruction);
    assert!(res.is_ok());

    let oracle_state = read_oracle(&svm, oracle);
    assert_eq!(oracle_state.admin, admin.pubkey());
    assert_eq!(oracle_state.price, initialize_price);
    assert_eq!(oracle_state.decimals, token_factory::constants::EXPECTED_DECIMALS);
}

#[test]
fn test_initialize_zero_price_fails() {
    let (mut svm, program_id, admin, oracle) = setup();

    let instruction = initialize_instruction(program_id, admin.pubkey(), oracle, 0);
    let res = send(&mut svm, &admin, instruction);

    assert_anchor_error(&res.unwrap_err(), token_factory::error::ErrorCode::InvalidPrice);
    assert!(svm.get_account(&oracle).is_none());
}

#[test]
fn test_update_price() {
    let (mut svm, program_id, admin, oracle) = setup();
    let initialize_price: u64 = 1_000_000;
    let new_price: u64 = 1_100_000;

    send(
        &mut svm,
        &admin,
        initialize_instruction(program_id, admin.pubkey(), oracle, initialize_price),
    )
    .unwrap();

    let instruction = update_price_instruction(program_id, admin.pubkey(), oracle, new_price);
    let res = send(&mut svm, &admin, instruction);
    assert!(res.is_ok());

    let oracle_state = read_oracle(&svm, oracle);
    assert_eq!(oracle_state.price, new_price);
}

#[test]
fn test_update_price_out_of_range_fails() {
    let (mut svm, program_id, admin, oracle) = setup();
    let initialize_price: u64 = 1_000_000;
    let new_price: u64 = 2_000_000; // above the 120% ceiling

    send(
        &mut svm,
        &admin,
        initialize_instruction(program_id, admin.pubkey(), oracle, initialize_price),
    )
    .unwrap();

    let instruction = update_price_instruction(program_id, admin.pubkey(), oracle, new_price);
    let res = send(&mut svm, &admin, instruction);
    assert_anchor_error(
        &res.unwrap_err(),
        token_factory::error::ErrorCode::PriceOutOfRange,
    );

    let oracle_state = read_oracle(&svm, oracle);
    assert_eq!(oracle_state.price, initialize_price);
}

#[test]
fn test_get_price_fresh() {
    let (mut svm, program_id, admin, oracle) = setup();
    let initialize_price: u64 = 1_000_000;

    send(
        &mut svm,
        &admin,
        initialize_instruction(program_id, admin.pubkey(), oracle, initialize_price),
    )
    .unwrap();

    let reader = Keypair::new();
    svm.airdrop(&reader.pubkey(), 1_000_000_000).unwrap();

    let instruction = get_price_instruction(program_id, oracle);
    let res = send(&mut svm, &reader, instruction);
    assert!(res.is_ok());
}

#[test]
fn test_get_price_stale_fails() {
    let (mut svm, program_id, admin, oracle) = setup();
    let initialize_price: u64 = 1_000_000;

    send(
        &mut svm,
        &admin,
        initialize_instruction(program_id, admin.pubkey(), oracle, initialize_price),
    )
    .unwrap();

    let last_updated_slot = read_oracle(&svm, oracle).last_updated_slot;
    svm.warp_to_slot(last_updated_slot + token_factory::constants::MAX_STALENESS_SLOTS + 1);

    let reader = Keypair::new();
    svm.airdrop(&reader.pubkey(), 1_000_000_000).unwrap();

    let instruction = get_price_instruction(program_id, oracle);
    let res = send(&mut svm, &reader, instruction);
    assert_anchor_error(&res.unwrap_err(), token_factory::error::ErrorCode::StaleOracle);
}

#[test]
fn test_update_price_wrong_admin_fails() {
    let (mut svm, program_id, admin, oracle) = setup();
    let initialize_price: u64 = 1_000_000;
    let new_price: u64 = 1_100_000;

    send(
        &mut svm,
        &admin,
        initialize_instruction(program_id, admin.pubkey(), oracle, initialize_price),
    )
    .unwrap();

    let attacker = Keypair::new();
    svm.airdrop(&attacker.pubkey(), 1_000_000_000).unwrap();

    let instruction = update_price_instruction(program_id, attacker.pubkey(), oracle, new_price);
    let res = send(&mut svm, &attacker, instruction);
    assert_anchor_error(
        &res.unwrap_err(),
        anchor_lang::error::ErrorCode::ConstraintHasOne,
    );

    let oracle_state = read_oracle(&svm, oracle);
    assert_eq!(oracle_state.price, initialize_price);
}

#[test]
fn test_set_admin() {
    let (mut svm, program_id, admin, oracle) = setup();
    let initialize_price: u64 = 1_000_000;

    send(
        &mut svm,
        &admin,
        initialize_instruction(program_id, admin.pubkey(), oracle, initialize_price),
    )
    .unwrap();

    let new_admin = Keypair::new();
    let instruction =
        set_admin_instruction(program_id, admin.pubkey(), oracle, new_admin.pubkey());
    let res = send(&mut svm, &admin, instruction);
    assert!(res.is_ok());

    let oracle_state = read_oracle(&svm, oracle);
    assert_eq!(oracle_state.admin, new_admin.pubkey());
}

#[test]
fn test_set_admin_wrong_admin_fails() {
    let (mut svm, program_id, admin, oracle) = setup();
    let initialize_price: u64 = 1_000_000;

    send(
        &mut svm,
        &admin,
        initialize_instruction(program_id, admin.pubkey(), oracle, initialize_price),
    )
    .unwrap();

    let attacker = Keypair::new();
    svm.airdrop(&attacker.pubkey(), 1_000_000_000).unwrap();

    let new_admin = Keypair::new();
    let instruction =
        set_admin_instruction(program_id, attacker.pubkey(), oracle, new_admin.pubkey());
    let res = send(&mut svm, &attacker, instruction);
    assert_anchor_error(
        &res.unwrap_err(),
        anchor_lang::error::ErrorCode::ConstraintHasOne,
    );

    let oracle_state = read_oracle(&svm, oracle);
    assert_eq!(oracle_state.admin, admin.pubkey());
}
