use axum::{extract::{Path, State}, Json};
use serde_json::json;

use crate::error::{ApiError, ApiResult};
use crate::AppState;

/// GET /api/v1/admin/chains
pub async fn list(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query_as::<_, (String, i64, String, String, Option<String>, bool)>(
        "SELECT chain, chain_id, rpc_url, contract_address, explorer_url, is_active FROM chain_configs ORDER BY chain_id",
    )
    .fetch_all(&state.db).await?;

    Ok(Json(json!({
        "chains": rows.iter().map(|(c, id, rpc, addr, exp, active)| json!({
            "chain": c, "chain_id": id, "rpc_url": rpc,
            "contract_address": addr, "explorer_url": exp, "is_active": active
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

    sqlx::query("UPDATE chain_configs SET rpc_url=$1, contract_address=$2, is_active=$3 WHERE chain=$4")
        .bind(rpc_url).bind(contract).bind(active).bind(&chain)
        .execute(&state.db).await?;

    Ok(Json(json!({"status": "updated"})))
}
