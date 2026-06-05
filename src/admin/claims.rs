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
    use sqlx::QueryBuilder;

    let mut builder = QueryBuilder::new(
        "SELECT c.id, c.packet_id, c.recipient_address, c.amount, c.status, c.tx_hash, c.created_at, p.chain
         FROM claims c JOIN packets p ON p.id=c.packet_id WHERE 1=1"
    );

    if let Some(pid) = params.get("packet_id").and_then(|s| Uuid::parse_str(s).ok()) {
        builder.push(" AND c.packet_id = ");
        builder.push_bind(pid);
    }
    if let Some(status) = params.get("status") {
        builder.push(" AND c.status = ");
        builder.push_bind(status);
    }
    builder.push(" ORDER BY c.created_at DESC LIMIT 100");

    let rows = builder
        .build_query_as::<(Uuid, Uuid, String, String, String, Option<String>, chrono::DateTime<chrono::Utc>, String)>()
        .fetch_all(&state.db).await?;

    Ok(Json(json!({
        "claims": rows.iter().map(|(id, pid, addr, amt, st, tx, created, chain)| json!({
            "id": id, "packet_id": pid, "recipient": addr,
            "amount": amt, "status": st, "tx_hash": tx,
            "created_at": created, "chain": chain
        })).collect::<Vec<_>>()
    })))
}
