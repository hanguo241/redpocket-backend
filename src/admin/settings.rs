use axum::{extract::State, Json};
use serde_json::json;

use crate::error::ApiResult;
use crate::AppState;

/// GET /api/v1/admin/settings
pub async fn get(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    let signer = format!("{:?}", state.signer.address());
    let relayer = format!("{:?}", state.relayer.address());

    Ok(Json(json!({
        "signer_address": signer,
        "relayer_address": relayer,
        "default_fee_bps": 20,
        "share_url_host": state.config.share_url_host,
    })))
}
