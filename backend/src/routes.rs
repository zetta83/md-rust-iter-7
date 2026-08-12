use std::sync::Arc;

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};
use tower_http::cors::{Any, CorsLayer};

use crate::error::AppError;
use crate::oracle::{fetch_oracle_state, AppState, OracleResponse};

pub fn build_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any);

    Router::new()
        .route("/health", get(health))
        .route("/oracle", get(oracle))
        .with_state(state)
        .layer(cors)
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

async fn oracle(State(state): State<Arc<AppState>>) -> Result<Json<OracleResponse>, AppError> {
    Ok(Json(fetch_oracle_state(&state).await?))
}
