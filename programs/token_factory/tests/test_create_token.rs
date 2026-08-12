mod common;

use {
    anchor_lang::{
        prelude::Pubkey,
        solana_program::{instruction::Instruction, rent, system_program},
        AccountDeserialize, InstructionData, ToAccountMetas,
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
    oracle: Pubkey,
}

fn setup() -> (LiteSVM, Pubkey, Keypair, TokenAccounts) {
    let program_id = token_factory::id();
    let admin = Keypair::new();
    let mint = Keypair::new();
    let mint_authority = Pubkey::find_program_address(
        &[token_factory::constants::TOKEN_SEED, mint.pubkey().as_ref()],
        &program_id,
    )
    .0;
    let oracle =
        Pubkey::find_program_address(&[token_factory::constants::ORACLE_SEED], &program_id).0;

    let mut svm = LiteSVM::new();
    let bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/token_factory.so"
    ));
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&admin.pubkey(), 1_000_000_000).unwrap();

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
    send(&mut svm, &admin, &[], init_ix).unwrap();

    (
        svm,
        program_id,
        admin,
        TokenAccounts {
            mint,
            mint_authority,
            oracle,
        },
    )
}

fn create_token_instruction(
    program_id: Pubkey,
    admin: Pubkey,
    accounts: &TokenAccounts,
    decimals: u8,
    initial_supply: u64,
) -> Instruction {
    let admin_ata = associated_token::get_associated_token_address(&admin, &accounts.mint.pubkey());
    Instruction::new_with_bytes(
        program_id,
        &token_factory::instruction::CreateToken {
            decimals,
            initial_supply,
        }
        .data(),
        token_factory::accounts::CreateToken {
            mint: accounts.mint.pubkey(),
            admin_ata,
            mint_authority: accounts.mint_authority,
            admin,
            oracle: accounts.oracle,
            token_program: anchor_token::ID,
            associated_token_program: associated_token::ID,
            system_program: system_program::ID,
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

#[test]
fn test_create_token() {
    let (mut svm, program_id, admin, accounts) = setup();
    let decimals = token_factory::constants::EXPECTED_DECIMALS;
    let initial_supply: u64 = 1_000;

    let instruction = create_token_instruction(
        program_id,
        admin.pubkey(),
        &accounts,
        decimals,
        initial_supply,
    );
    let res = send(&mut svm, &admin, &[&accounts.mint], instruction);
    assert!(res.is_ok(), "tx failed: {:?}", res.err());

    let mint_state = read_mint(&svm, accounts.mint.pubkey());
    assert_eq!(mint_state.decimals, decimals);
    assert_eq!(mint_state.mint_authority.unwrap(), accounts.mint_authority);

    let expected_amount_raw = initial_supply * 10u64.pow(decimals as u32);
    assert_eq!(mint_state.supply, expected_amount_raw);

    let admin_ata =
        associated_token::get_associated_token_address(&admin.pubkey(), &accounts.mint.pubkey());
    let admin_ata_state = read_token_account(&svm, admin_ata);
    assert_eq!(admin_ata_state.mint, accounts.mint.pubkey());
    assert_eq!(admin_ata_state.owner, admin.pubkey());
    assert_eq!(admin_ata_state.amount, expected_amount_raw);
}

#[test]
fn test_create_token_wrong_admin_fails() {
    let (mut svm, program_id, _admin, accounts) = setup();
    let decimals = token_factory::constants::EXPECTED_DECIMALS;

    let attacker = Keypair::new();
    svm.airdrop(&attacker.pubkey(), 1_000_000_000).unwrap();

    let instruction =
        create_token_instruction(program_id, attacker.pubkey(), &accounts, decimals, 1_000);
    let res = send(&mut svm, &attacker, &[&accounts.mint], instruction);

    assert_anchor_error(
        &res.unwrap_err(),
        anchor_lang::error::ErrorCode::ConstraintHasOne,
    );
    assert!(svm.get_account(&accounts.mint.pubkey()).is_none());
}

#[test]
fn test_create_token_bad_decimals_fails() {
    let (mut svm, program_id, admin, accounts) = setup();
    let bad_decimals = token_factory::constants::EXPECTED_DECIMALS + 1;

    let instruction =
        create_token_instruction(program_id, admin.pubkey(), &accounts, bad_decimals, 1_000);
    let res = send(&mut svm, &admin, &[&accounts.mint], instruction);

    assert_anchor_error(
        &res.unwrap_err(),
        token_factory::error::ErrorCode::BadTokenDecimals,
    );
    assert!(svm.get_account(&accounts.mint.pubkey()).is_none());
}

#[test]
fn test_create_token_overflow_fails() {
    let (mut svm, program_id, admin, accounts) = setup();
    let decimals = token_factory::constants::EXPECTED_DECIMALS;

    let instruction =
        create_token_instruction(program_id, admin.pubkey(), &accounts, decimals, u64::MAX);
    let res = send(&mut svm, &admin, &[&accounts.mint], instruction);

    assert_anchor_error(
        &res.unwrap_err(),
        token_factory::error::ErrorCode::MathOverflow,
    );
    assert!(svm.get_account(&accounts.mint.pubkey()).is_none());
}
