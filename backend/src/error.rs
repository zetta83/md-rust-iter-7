use anchor_lang::error::Error as AnchorError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use solana_client::client_error::ClientError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("oracle account not found at the configured address")]
    OracleNotFound,
    #[error("oracle account is not owned by the configured program")]
    OwnerMismatch,
    #[error("rpc error: {0}")]
    Rpc(#[from] ClientError),
    #[error("failed to decode oracle account: {0}")]
    Deserialize(#[from] AnchorError),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self {
            AppError::OracleNotFound => StatusCode::NOT_FOUND,
            AppError::OwnerMismatch | AppError::Rpc(_) | AppError::Deserialize(_) => {
                StatusCode::BAD_GATEWAY
            }
        };

        (status, Json(json!({ "error": self.to_string() }))).into_response()
    }
}
