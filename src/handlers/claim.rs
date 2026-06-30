use axum::{extract::State, Json};
use uuid::Uuid;

use redpacket_backend::error::{ApiError, ApiResult};
use crate::AppState;

/// POST /api/v1/claim/prepare
pub async fn prepare(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let packet_id = body.get("packet_id")
        .and_then(|v| v.as_str()).and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| ApiError::BadRequest("packet_id required".into()))?;
    let user_addr = body.get("user_address")
        .and_then(|v| v.as_str()).ok_or_else(|| ApiError::BadRequest("user_address required".into()))?;
    let proof_pw = body.get("proof")
        .and_then(|p| p.get("password")).and_then(|v| v.as_str());

    let resp = state.claim_service.prepare(packet_id, user_addr, proof_pw).await?;
    Ok(Json(resp))
}

/// POST /api/v1/claim/confirm
pub async fn confirm(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let packet_id = body.get("packet_id")
        .and_then(|v| v.as_str()).and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| ApiError::BadRequest("packet_id required".into()))?;
    let recipient = body.get("recipient")
        .and_then(|v| v.as_str()).ok_or_else(|| ApiError::BadRequest("recipient required".into()))?;
    let tx_hash = body.get("tx_hash")
        .and_then(|v| v.as_str()).ok_or_else(|| ApiError::BadRequest("tx_hash required".into()))?;

    let resp = state.claim_service.confirm(packet_id, recipient, tx_hash).await?;
    Ok(Json(resp))
}

/// POST /api/v1/claim/proxy
pub async fn proxy_claim(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let packet_id = body.get("packet_id")
        .and_then(|v| v.as_str()).and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| ApiError::BadRequest("packet_id required".into()))?;
    let user_addr = body.get("user_address")
        .and_then(|v| v.as_str()).ok_or_else(|| ApiError::BadRequest("user_address required".into()))?;
    let user_sig = body.get("user_signature")
        .and_then(|v| v.as_str()).ok_or_else(|| ApiError::BadRequest("user_signature required".into()))?;
    let proof_pw = body.get("proof")
        .and_then(|p| p.get("password")).and_then(|v| v.as_str());

    let resp = state.claim_service.proxy_claim(packet_id, user_addr, user_sig, proof_pw).await?;
    Ok(Json(resp))
}
