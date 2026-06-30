use axum::{extract::State, Json};
use serde_json::json;
use uuid::Uuid;

use redpacket_backend::error::ApiResult;
use crate::AppState;

/// GET /api/v1/admin/claims
pub async fn list(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> ApiResult<Json<serde_json::Value>> {
    let packet_id = params.get("packet_id").and_then(|s| Uuid::parse_str(s).ok());
    let status = params.get("status").map(|s| s.as_str());

    let rows = state.claim_repo.admin_list(packet_id, status).await?;

    Ok(Json(json!({
        "claims": rows.iter().map(|r| json!({
            "id": r.id, "packet_id": r.packet_id, "recipient": r.recipient_address,
            "amount": r.amount, "status": r.status, "tx_hash": r.tx_hash,
            "created_at": r.created_at, "chain": r.chain
        })).collect::<Vec<_>>()
    })))
}
