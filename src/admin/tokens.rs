use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::json;

use redpacket_backend::error::{ApiError, ApiResult};
use crate::AppState;

// ── Query params ──

#[derive(Deserialize)]
pub struct ListParams {
    chain: Option<String>,
}

// ── Body types ──

#[derive(Deserialize)]
pub struct CreateTokenRequest {
    pub chain: String,
    pub token_address: String,
    pub symbol: String,
    pub name: String,
    pub decimals: i32,
    pub is_native: Option<bool>,
    pub token_type: String,
    pub logo_url: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Deserialize)]
pub struct UpdateTokenRequest {
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub decimals: Option<i32>,
    pub token_type: Option<String>,
    pub logo_url: Option<String>,
    pub sort_order: Option<i32>,
    pub is_active: Option<bool>,
}

// ── Handlers ──

/// GET /api/v1/admin/tokens?chain=XXX
pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> ApiResult<Json<serde_json::Value>> {
    let tokens = state
        .config_repo
        .list_all_tokens(params.chain.as_deref())
        .await?;

    Ok(Json(json!({
        "tokens": tokens.iter().map(|t| json!({
            "id": t.id,
            "chain": t.chain,
            "token_address": t.token_address,
            "symbol": t.symbol,
            "name": t.name,
            "decimals": t.decimals,
            "is_native": t.is_native,
            "token_type": t.token_type,
            "logo_url": t.logo_url,
            "sort_order": t.sort_order,
            "is_active": t.is_active,
        })).collect::<Vec<_>>()
    })))
}

/// POST /api/v1/admin/tokens
pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<CreateTokenRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    if body.chain.is_empty() || body.token_address.is_empty() || body.symbol.is_empty() {
        return Err(ApiError::BadRequest(
            "chain, token_address, symbol are required".into(),
        ));
    }

    let valid_types = ["native", "erc20", "spl-token", "spl-token-2022", "trc20"];
    if !valid_types.contains(&body.token_type.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "invalid token_type '{}'. Must be one of: {}",
            body.token_type,
            valid_types.join(", "),
        )));
    }

    state
        .config_repo
        .insert_token(
            &body.chain,
            &body.token_address,
            &body.symbol,
            &body.name,
            body.decimals,
            body.is_native.unwrap_or(false),
            &body.token_type,
            body.sort_order.unwrap_or(0),
            body.logo_url.as_deref(),
        )
        .await?;

    Ok(Json(json!({"status": "created"})))
}

/// PUT /api/v1/admin/tokens/{id}
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(body): Json<UpdateTokenRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    // 先读取现有记录
    let existing = state
        .config_repo
        .get_token_by_id(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Token id={} not found", id)))?;

    if let Some(ref token_type) = body.token_type {
        let valid_types = ["native", "erc20", "spl-token", "spl-token-2022", "trc20"];
        if !valid_types.contains(&token_type.as_str()) {
            return Err(ApiError::BadRequest(format!(
                "invalid token_type '{}'. Must be one of: {}",
                token_type,
                valid_types.join(", "),
            )));
        }
    }

    state
        .config_repo
        .update_token(
            id,
            body.symbol.as_deref().unwrap_or(&existing.symbol),
            body.name.as_deref().unwrap_or(&existing.name),
            body.decimals.unwrap_or(existing.decimals),
            body.token_type.as_deref().unwrap_or(&existing.token_type),
            body.logo_url.as_deref().or(existing.logo_url.as_deref()),
            body.sort_order.unwrap_or(existing.sort_order),
            body.is_active.unwrap_or(existing.is_active),
        )
        .await?;

    Ok(Json(json!({"status": "updated"})))
}

/// DELETE /api/v1/admin/tokens/{id}
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> ApiResult<Json<serde_json::Value>> {
    // 检查存在性
    if state.config_repo.get_token_by_id(id).await?.is_none() {
        return Err(ApiError::NotFound(format!("Token id={} not found", id)));
    }

    state.config_repo.delete_token(id).await?;
    Ok(Json(json!({"status": "deleted"})))
}
