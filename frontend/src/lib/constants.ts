export const RPC_URL = import.meta.env.VITE_RPC_URL ?? 'http://127.0.0.1:8899'
export const BACKEND_URL = import.meta.env.VITE_BACKEND_URL ?? 'http://127.0.0.1:8080'

// Mirrors EXPECTED_DECIMALS in programs/token_factory/src/constants.rs
export const EXPECTED_DECIMALS = 6
