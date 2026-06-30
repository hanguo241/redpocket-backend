use axum::{extract::{Path, State}, Json};
use serde_json::json;

use redpacket_backend::error::ApiResult;
use crate::AppState;

/// GET /api/v1/admin/chains
pub async fn list(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    let rows = state.config_repo.list_chain_configs().await?;

    Ok(Json(json!({
        "chains": rows.iter().map(|r| json!({
            "chain": r.chain, "chain_id": r.chain_id, "rpc_url": r.rpc_url,
            "contract_address": r.contract_address, "explorer_url": r.explorer_url, "is_active": r.is_active
        })).collect::<Vec<_>>()
    })))
}

/// PUT /api/v1/admin/chains/{chain}
pub async fn update(
    State(state): State<AppState>,
    Path(chain): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let rpc_url = body.get("rpc_url").and_then(|v| v.as_str()).unwrap_or("");
    let contract = body.get("contract_address").and_then(|v| v.as_str()).unwrap_or("");
    let active = body.get("is_active").and_then(|v| v.as_bool()).unwrap_or(true);

    state.config_repo.update_chain_config(&chain, rpc_url, contract, active).await?;

    Ok(Json(json!({"status": "updated"})))
}
