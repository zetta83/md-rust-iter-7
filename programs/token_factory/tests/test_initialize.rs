
use {
    anchor_lang::{
        prelude::Pubkey,
        solana_program::{instruction::Instruction, system_program},
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
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

    assert!(res.is_err());
    assert!(svm.get_account(&oracle).is_none());
}

#[test]
fn test_update_price() {
    let (mut svm, program_id, admin, oracle) = setup();
    let initialize_price: u64 = 1_000_000;
    let new_price: u64 = 1_100_000; // within [80%, 120%] of initialize_price

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
    assert!(res.is_err());

    let oracle_state = read_oracle(&svm, oracle);
    assert_eq!(oracle_state.price, initialize_price);
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
    assert!(res.is_err());

    let oracle_state = read_oracle(&svm, oracle);
    assert_eq!(oracle_state.price, initialize_price);
}
