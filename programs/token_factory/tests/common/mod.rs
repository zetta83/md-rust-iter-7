pub fn assert_anchor_error<T: Into<u32> + std::fmt::Debug + Copy>(
    err: &litesvm::types::FailedTransactionMetadata,
    expected: T,
) {
    let expected_code: u32 = expected.into();
    match err.err {
        solana_transaction_error::TransactionError::InstructionError(
            _,
            solana_instruction::error::InstructionError::Custom(code),
        ) => {
            assert_eq!(
                code, expected_code,
                "expected {:?} ({}), got custom error {}; logs: {:?}",
                expected, expected_code, code, err.meta.logs
            );
        }
        ref other => panic!(
            "expected custom program error {:?} ({}), got {:?}; logs: {:?}",
            expected, expected_code, other, err.meta.logs
        ),
    }
}
