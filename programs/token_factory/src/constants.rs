use anchor_lang::prelude::*;

#[constant]
pub const ORACLE_SEED: &[u8] = b"oracle";

#[constant]
pub const EXPECTED_DECIMALS: u8 = 6;

#[constant]
pub const MAX_STALENESS_SLOTS: u64 = 100;

#[constant]
pub const LAMPORTS_PER_SOL_U64: u64 = 1_000_000_000;

#[constant]
pub const TOKEN_SEED: &[u8] = b"mint_authority";

#[constant]
pub const TREASURY_SEED: &[u8] = b"mint_treasury";
