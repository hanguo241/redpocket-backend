use axum::{extract::State, Json};
use serde_json::json;

use redpacket_backend::error::ApiResult;
use crate::AppState;

/// GET /api/v1/config/gas
pub async fn get_gas_config(
    State(state): State<AppState>,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = state.config_repo.get_all_config().await?;

    let mut config = serde_json::Map::new();
    for (key, value) in rows {
        config.insert(key, json!(value));
    }

    Ok(Json(json!({
        "config": config
    })))
}

/// GET /api/v1/config/chains
///
/// 返回平台支持的链列表
pub async fn get_chains(
    State(state): State<AppState>,
) -> ApiResult<Json<serde_json::Value>> {
    let chains = state.config_repo.list_active_chains().await?;

    Ok(Json(json!({
        "chains": chains.iter().map(|r| json!({
            "chain": r.chain,
            "chain_id": r.chain_id,
            "rpc_url": r.rpc_url,
            "contract_address": r.contract_address,
            "explorer_url": r.explorer_url,
            "is_active": r.is_active,
        })).collect::<Vec<_>>()
    })))
}
