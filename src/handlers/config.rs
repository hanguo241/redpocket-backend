use axum::{extract::State, Json};
use serde::Serialize;
use serde_json::json;

use crate::{error::ApiResult, AppState};

/// GET /api/v1/config/gas
pub async fn get_gas_config(
    State(state): State<AppState>,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT key, value FROM system_config ORDER BY key",
    )
    .fetch_all(&state.db)
    .await?;

    let mut config = serde_json::Map::new();
    for (key, value) in &rows {
        config.insert(key.clone(), json!(value));
    }

    Ok(Json(json!({
        "config": config
    })))
}

/// 链配置返回
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ChainConfig {
    pub chain: String,
    pub chain_id: i64,
    pub rpc_url: String,
    pub contract_address: String,
    pub explorer_url: Option<String>,
    pub is_active: bool,
}

/// GET /api/v1/config/chains
///
/// 返回平台支持的链列表
pub async fn get_chains(
    State(state): State<AppState>,
) -> ApiResult<Json<serde_json::Value>> {
    let chains = sqlx::query_as::<_, ChainConfig>(
        r#"
        SELECT chain, chain_id, rpc_url, contract_address, explorer_url, is_active
        FROM chain_configs
        WHERE is_active = true
        ORDER BY chain_id
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({
        "chains": chains
    })))
}
