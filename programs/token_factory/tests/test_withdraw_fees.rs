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

const FEES_COLLECTED: u64 = 5_000_000_000;

fn setup(fees_collected: u64) -> (LiteSVM, Pubkey, Keypair, Pubkey, Pubkey) {
    let program_id = token_factory::id();
    let admin = Keypair::new();
    let oracle =
        Pubkey::find_program_address(&[token_factory::constants::ORACLE_SEED], &program_id).0;
    let treasury =
        Pubkey::find_program_address(&[token_factory::constants::TREASURY_SEED], &program_id).0;

    let mut svm = LiteSVM::new();
    let bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/token_factory.so"
    ));
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&admin.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&treasury, fees_collected).unwrap();

    let init_ix = Instruction::new_with_bytes(
        program_id,
        &token_factory::instruction::Initialize {
            initialize_price: 1_000_000,
        }
        .data(),
        token_factory::accounts::InitializeOracle {
            admin: admin.pubkey(),
            oracle,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );
    send(&mut svm, &admin, init_ix).unwrap();

    (svm, program_id, admin, oracle, treasury)
}

fn withdraw_fees_instruction(
    program_id: Pubkey,
    admin: Pubkey,
    oracle: Pubkey,
    treasury: Pubkey,
    amount: u64,
) -> Instruction {
    Instruction::new_with_bytes(
        program_id,
        &token_factory::instruction::WithdrawFees { amount }.data(),
        token_factory::accounts::WithdrawFees {
            oracle,
            admin,
            treasury,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

fn send(
    svm: &mut LiteSVM,
    payer: &Keypair,
    instruction: Instruction,
) -> Result<(), litesvm::types::FailedTransactionMetadata> {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[payer]).unwrap();
    svm.send_transaction(tx).map(|_| ())
}

fn read_oracle(svm: &LiteSVM, oracle: Pubkey) -> token_factory::state::OracleState {
    let account = svm.get_account(&oracle).unwrap();
    let mut data: &[u8] = &account.data;
    token_factory::state::OracleState::try_deserialize(&mut data).unwrap()
}

#[test]
fn test_withdraw_fees_partial() {
    let (mut svm, program_id, admin, oracle, treasury) = setup(FEES_COLLECTED);
    let admin_balance_before = svm.get_balance(&admin.pubkey()).unwrap();
    let amount = FEES_COLLECTED / 2;

    let instruction =
        withdraw_fees_instruction(program_id, admin.pubkey(), oracle, treasury, amount);
    let res = send(&mut svm, &admin, instruction);
    assert!(res.is_ok(), "tx failed: {:?}", res.err());

    assert_eq!(svm.get_balance(&treasury).unwrap(), FEES_COLLECTED - amount);
    let admin_balance_after = svm.get_balance(&admin.pubkey()).unwrap();
    assert!(admin_balance_after > admin_balance_before);
    assert!(admin_balance_after <= admin_balance_before + amount);
}

#[test]
fn test_withdraw_fees_full_drains_treasury() {
    let (mut svm, program_id, admin, oracle, treasury) = setup(FEES_COLLECTED);

    let instruction =
        withdraw_fees_instruction(program_id, admin.pubkey(), oracle, treasury, FEES_COLLECTED);
    let res = send(&mut svm, &admin, instruction);
    assert!(res.is_ok(), "tx failed: {:?}", res.err());

    assert_eq!(svm.get_balance(&treasury).unwrap_or(0), 0);
}

#[test]
fn test_withdraw_fees_zero_amount_fails() {
    let (mut svm, program_id, admin, oracle, treasury) = setup(FEES_COLLECTED);

    let instruction = withdraw_fees_instruction(program_id, admin.pubkey(), oracle, treasury, 0);
    let res = send(&mut svm, &admin, instruction);

    assert_anchor_error(
        &res.unwrap_err(),
        token_factory::error::ErrorCode::InvalidWithdrawAmount,
    );
    assert_eq!(svm.get_balance(&treasury).unwrap(), FEES_COLLECTED);
}

#[test]
fn test_withdraw_fees_amount_exceeds_balance_fails() {
    let (mut svm, program_id, admin, oracle, treasury) = setup(FEES_COLLECTED);

    let instruction = withdraw_fees_instruction(
        program_id,
        admin.pubkey(),
        oracle,
        treasury,
        FEES_COLLECTED + 1,
    );
    let res = send(&mut svm, &admin, instruction);

    assert_anchor_error(
        &res.unwrap_err(),
        token_factory::error::ErrorCode::InsufficientTreasuryBalance,
    );
    assert_eq!(svm.get_balance(&treasury).unwrap(), FEES_COLLECTED);
}

#[test]
fn test_withdraw_fees_wrong_admin_fails() {
    let (mut svm, program_id, admin, oracle, treasury) = setup(FEES_COLLECTED);

    let attacker = Keypair::new();
    svm.airdrop(&attacker.pubkey(), 1_000_000_000).unwrap();

    let instruction = withdraw_fees_instruction(
        program_id,
        attacker.pubkey(),
        oracle,
        treasury,
        FEES_COLLECTED / 2,
    );
    let res = send(&mut svm, &attacker, instruction);

    assert_anchor_error(
        &res.unwrap_err(),
        anchor_lang::error::ErrorCode::ConstraintHasOne,
    );

    assert_eq!(svm.get_balance(&treasury).unwrap(), FEES_COLLECTED);
    let oracle_state = read_oracle(&svm, oracle);
    assert_eq!(oracle_state.admin, admin.pubkey());
}
