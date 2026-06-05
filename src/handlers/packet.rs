use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::models::packet::CreatePacketRequest;
use crate::AppState;

/// POST /api/v1/packet/prepare
pub async fn prepare(
    State(state): State<AppState>,
    Json(req): Json<CreatePacketRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let resp = state.packet_service.prepare(
        &req.chain, &req.token, &req.total_amount,
        req.head_count, &req.packet_type, &req.sub_type,
        req.start_time, req.end_time, req.gas_reserve_wei,
    ).await?;
    Ok(Json(resp))
}

/// POST /api/v1/packet/create
pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let packet_id = body.get("packet_id")
        .and_then(|v| v.as_str()).and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| ApiError::BadRequest("packet_id required".into()))?;
    let tx_hash = body.get("tx_hash")
        .and_then(|v| v.as_str()).ok_or_else(|| ApiError::BadRequest("tx_hash required".into()))?;
    let creator = body.get("creator_address")
        .and_then(|v| v.as_str()).ok_or_else(|| ApiError::BadRequest("creator_address required".into()))?;
    let chain = body.get("chain").and_then(|v| v.as_str()).unwrap_or("");
    let contract_addr = body.get("contract_address").and_then(|v| v.as_str()).unwrap_or("");
    let token = body.get("token").and_then(|v| v.as_str()).unwrap_or("native");
    let total_amount = body.get("total_amount").and_then(|v| v.as_str()).unwrap_or("0");
    let head_count = body.get("head_count").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
    let packet_type = body.get("packet_type").and_then(|v| v.as_str()).unwrap_or("normal");
    let sub_type = body.get("sub_type").and_then(|v| v.as_str()).unwrap_or("average");
    let claim_mode = body.get("claim_mode").and_then(|v| v.as_str()).unwrap_or("both");
    let start_time = body.get("start_time").and_then(|v| v.as_i64()).unwrap_or(0);
    let end_time = body.get("end_time").and_then(|v| v.as_i64()).unwrap_or(0);
    let password = body.get("password").and_then(|v| v.as_str());
    let signer_address = body.get("signer_address").and_then(|v| v.as_str()).unwrap_or("");
    let fee_bps = body.get("fee_bps").and_then(|v| v.as_i64()).unwrap_or(20) as i32;
    let fee_collector = body.get("fee_collector").and_then(|v| v.as_str()).unwrap_or(signer_address);
    let gas_reserve_wei = body.get("gas_reserve_wei").and_then(|v| v.as_str()).unwrap_or("0");
    let gas_estimate_multiplier = body.get("gas_estimate_multiplier").and_then(|v| v.as_f64()).unwrap_or(1.2);

    let resp = state.packet_service.create(
        packet_id, tx_hash, creator, chain, contract_addr,
        token, total_amount, head_count, packet_type, sub_type,
        claim_mode, password, start_time, end_time,
        signer_address, fee_bps, fee_collector,
        gas_reserve_wei, gas_estimate_multiplier,
        state.db.clone(),
    ).await?;
    Ok(Json(resp))
}

/// GET /api/v1/packet/{id}/status
pub async fn get_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let resp = state.packet_service.get_status(id).await?;
    Ok(Json(resp))
}
