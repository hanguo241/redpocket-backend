use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::json;
use uuid::Uuid;

use redpacket_backend::error::{ApiError, ApiResult};
use crate::AppState;

/// GET /api/v1/admin/packets
pub async fn list(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> ApiResult<Json<serde_json::Value>> {
    let status = params.get("status").map(|s| s.as_str());
    let chain = params.get("chain").map(|s| s.as_str());

    let rows = state.packet_repo.admin_list(status, chain).await?;

    Ok(Json(json!({
        "packets": rows.iter().map(|r| json!({
            "id": r.id, "chain": r.chain, "creator": r.creator_address,
            "gross_amount": r.gross_amount, "total_amount": r.total_amount,
            "platform_fee_wei": r.platform_fee_wei, "status": r.status,
            "created_at": r.created_at, "end_time": r.end_time
        })).collect::<Vec<_>>()
    })))
}

/// GET /api/v1/admin/packets/{id}
pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let pkt = state.packet_repo.admin_get_detail(id).await?
        .ok_or_else(|| ApiError::NotFound("Packet not found".into()))?;

    let claims = state.packet_repo.admin_get_claims(id).await?;

    Ok(Json(json!({
        "packet": {
            "chain": pkt.chain, "contract": pkt.contract_address, "creator": pkt.creator_address,
            "token": pkt.token_address, "total_amount": pkt.total_amount, "status": pkt.status,
            "gross_amount": pkt.gross_amount, "platform_fee_wei": pkt.platform_fee_wei,
            "head_count": pkt.head_count, "claim_mode": pkt.claim_mode, "end_time": pkt.end_time,
            "onchain_packet_id": pkt.onchain_packet_id,
        },
        "claims": claims.iter().map(|c| json!({
            "recipient": c.recipient_address, "amount": c.amount, "status": c.status,
            "tx_hash": c.tx_hash, "created_at": c.created_at
        })).collect::<Vec<_>>()
    })))
}
