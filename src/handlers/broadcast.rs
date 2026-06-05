use axum::{extract::Path, Json};
use serde_json::json;
use uuid::Uuid;

use crate::{error::{ApiError, ApiResult}, AppState};

/// POST /api/v1/packet/{id}/broadcast
///
/// 前端签名后，将 signed RLP 交给后端广播上链
pub async fn broadcast(
    state: axum::extract::State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let signed_rlp = body
        .get("signed_rlp")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::BadRequest("signed_rlp is required".into()))?;

    let creator = body
        .get("creator_address")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::BadRequest("creator_address is required".into()))?;

    // 1. 查红包记录获取链信息
    let row = sqlx::query_as::<_, (String,)>(
        "SELECT chain FROM packets WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Packet not found".into()))?;

    let chain_name = row.0;

    // 2. 从 chain_configs 获取 RPC URL
    let rpc = sqlx::query_as::<_, (String,)>(
        "SELECT rpc_url FROM chain_configs WHERE chain = $1 AND is_active = true",
    )
    .bind(&chain_name)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::BadRequest(format!("Chain '{}' not configured", chain_name)))?;

    let rpc_url = rpc.0;
    if rpc_url.is_empty() {
        return Err(ApiError::Internal("RPC URL not configured for chain".into()));
    }

    // 3. 通过 RPC 广播交易
    let client = reqwest::Client::new();
    let rpc_resp = client
        .post(&rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_sendRawTransaction",
            "params": [signed_rlp],
            "id": 1
        }))
        .send()
        .await
        .map_err(|e| ApiError::Internal(format!("RPC call failed: {}", e)))?;

    let rpc_body: serde_json::Value = rpc_resp
        .json()
        .await
        .map_err(|e| ApiError::Internal(format!("RPC response parse failed: {}", e)))?;

    // 4. 检查 RPC 错误
    if let Some(err) = rpc_body.get("error") {
        return Err(ApiError::Internal(format!(
            "RPC error: {}",
            err.get("message").and_then(|m| m.as_str()).unwrap_or("unknown")
        )));
    }

    let tx_hash = rpc_body
        .get("result")
        .and_then(|r| r.as_str())
        .ok_or_else(|| ApiError::Internal("RPC did not return tx_hash".into()))?;

    // 5. 更新红包记录
    sqlx::query(
        r#"
        UPDATE packets
        SET status = 'active',
            creator_address = $2,
            updated_at = NOW()
        WHERE id = $1 AND status = 'pending'
        "#,
    )
    .bind(id)
    .bind(creator)
    .execute(&state.db)
    .await?;

    tracing::info!(
        "Packet broadcast: {} creator={} tx={}",
        id, creator, tx_hash
    );

    Ok(Json(json!({
        "packet_id": id,
        "status": "active",
        "tx_hash": tx_hash,
    })))
}
