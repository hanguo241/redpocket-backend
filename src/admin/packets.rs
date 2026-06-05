use axum::{extract::{Path, State}, Json};
use serde_json::json;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::AppState;

/// GET /api/v1/admin/packets
pub async fn list(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> ApiResult<Json<serde_json::Value>> {
    let status_filter = params.get("status").map(|s| s.as_str()).unwrap_or("");
    let chain_filter = params.get("chain").map(|s| s.as_str()).unwrap_or("");

    let mut sql = String::from(
        "SELECT id, chain, creator_address, total_amount, status, created_at, end_time FROM packets WHERE 1=1"
    );
    if !status_filter.is_empty() { sql.push_str(" AND status='"); sql.push_str(status_filter); sql.push('\''); }
    if !chain_filter.is_empty() { sql.push_str(" AND chain='"); sql.push_str(chain_filter); sql.push('\''); }
    sql.push_str(" ORDER BY created_at DESC LIMIT 100");

    let rows = sqlx::query_as::<_, (Uuid, String, String, String, String, chrono::DateTime<chrono::Utc>, i64)>(&sql)
        .fetch_all(&state.db).await?;

    Ok(Json(json!({
        "packets": rows.iter().map(|(id, chain, creator, amt, status, created, end)| json!({
            "id": id, "chain": chain, "creator": creator,
            "total_amount": amt, "status": status,
            "created_at": created, "end_time": end
        })).collect::<Vec<_>>()
    })))
}

/// GET /api/v1/admin/packets/{id}
pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let pkt = sqlx::query_as::<_, (String, String, String, String, String, String, i32, String, i64, Option<i64>)>(
        r#"SELECT chain,contract_address,creator_address,token_address,total_amount,status,
                  head_count,claim_mode,end_time,onchain_packet_id FROM packets WHERE id=$1"#,
    )
    .bind(id)
    .fetch_optional(&state.db).await?
    .ok_or_else(|| ApiError::NotFound("Packet not found".into()))?;

    let claims = sqlx::query_as::<_, (String, String, String, String, chrono::DateTime<chrono::Utc>)>(
        "SELECT recipient_address,amount,status,tx_hash,created_at FROM claims WHERE packet_id=$1 ORDER BY created_at DESC",
    )
    .bind(id)
    .fetch_all(&state.db).await?;

    Ok(Json(json!({
        "packet": {
            "chain": pkt.0, "contract": pkt.1, "creator": pkt.2, "token": pkt.3,
            "total_amount": pkt.4, "status": pkt.5, "head_count": pkt.6,
            "claim_mode": pkt.7, "end_time": pkt.8, "onchain_packet_id": pkt.9,
        },
        "claims": claims.iter().map(|(addr, amt, st, tx, created)| json!({
            "recipient": addr, "amount": amt, "status": st, "tx_hash": tx, "created_at": created
        })).collect::<Vec<_>>()
    })))
}
