use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid oracle price")]
    InvalidPrice,
    #[msg("Oracle price overflow")]
    StaleOracle,
    #[msg("Oracle price overflow")]
    PriceOutOfRange,
    #[msg("Bad token decimals")]
    BadTokenDecimals,
    #[msg("Bad token fee")]
    BadTokenFeeUsd,
    #[msg("Bad oracle decimals")]
    BadOracleDecimals,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Withdraw amount must be greater than zero")]
    InvalidWithdrawAmount,
    #[msg("Withdraw amount exceeds treasury balance")]
    InsufficientTreasuryBalance,
}
