
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

#[test]
fn test_initialize() {
    let (mut svm, program_id, admin, oracle) = setup();
    let initialize_price: u64 = 1_000_000;

    let instruction = initialize_instruction(program_id, admin.pubkey(), oracle, initialize_price);

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&admin.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&admin]).unwrap();

    let res = svm.send_transaction(tx);
    assert!(res.is_ok());

    let oracle_account = svm.get_account(&oracle).unwrap();
    let mut data: &[u8] = &oracle_account.data;
    let oracle_state = token_factory::state::OracleState::try_deserialize(&mut data).unwrap();
    assert_eq!(oracle_state.admin, admin.pubkey());
    assert_eq!(oracle_state.price, initialize_price);
    assert_eq!(oracle_state.decimals, token_factory::constants::EXPECTED_DECIMALS);
}

#[test]
fn test_initialize_zero_price_fails() {
    let (mut svm, program_id, admin, oracle) = setup();

    let instruction = initialize_instruction(program_id, admin.pubkey(), oracle, 0);

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&admin.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&admin]).unwrap();

    let res = svm.send_transaction(tx);
    assert!(res.is_err());
    assert!(svm.get_account(&oracle).is_none());
}
