use axum::{extract::State, Json};
use serde_json::json;
use uuid::Uuid;

use crate::error::ApiResult;
use crate::AppState;

/// GET /api/v1/admin/claims
pub async fn list(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> ApiResult<Json<serde_json::Value>> {
    let packet_filter = params.get("packet_id").and_then(|s| Uuid::parse_str(s).ok());
    let status_filter = params.get("status").map(|s| s.as_str()).unwrap_or("");

    let mut sql = String::from(
        "SELECT c.id, c.packet_id, c.recipient_address, c.amount, c.status, c.tx_hash, c.created_at, p.chain
         FROM claims c JOIN packets p ON p.id=c.packet_id WHERE 1=1"
    );
    if let Some(pid) = packet_filter {
        sql.push_str(&format!(" AND c.packet_id='{}'", pid));
    }
    if !status_filter.is_empty() {
        sql.push_str(" AND c.status='");
        sql.push_str(status_filter);
        sql.push('\'');
    }
    sql.push_str(" ORDER BY c.created_at DESC LIMIT 100");

    let rows = sqlx::query_as::<_, (Uuid, Uuid, String, String, String, Option<String>, chrono::DateTime<chrono::Utc>, String)>(&sql)
        .fetch_all(&state.db).await?;

    Ok(Json(json!({
        "claims": rows.iter().map(|(id, pid, addr, amt, st, tx, created, chain)| json!({
            "id": id, "packet_id": pid, "recipient": addr,
            "amount": amt, "status": st, "tx_hash": tx,
            "created_at": created, "chain": chain
        })).collect::<Vec<_>>()
    })))
}
