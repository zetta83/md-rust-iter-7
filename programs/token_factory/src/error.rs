use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid oracle price")]
    InvalidPrice,
    #[msg("Oracle price overflow")]
    StaleOracle,
    #[msg("Oracle price overflow")]
    PriceOutOfRange,
}
