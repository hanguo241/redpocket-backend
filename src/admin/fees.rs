use axum::{extract::State, Extension, Json};
use ethers::core::types::H160;
use serde::Deserialize;
use serde_json::json;

use redpacket_backend::error::{ApiError, ApiResult};
use redpacket_backend::services::abi;
use crate::{
    middleware::auth::AdminContext,
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct WithdrawFeesRequest {
    pub chain: String,
    pub token: String,
    pub to: String,
    pub amount: String,
}

/// POST /api/v1/admin/fees/withdraw-transaction
///
/// 返回提取平台手续费的链上交易数据。后端不广播、不持有 owner 钱包；
/// 管理员需要用合约 owner 钱包签名发送这笔交易。
pub async fn prepare_withdraw_transaction(
    State(state): State<AppState>,
    Extension(admin): Extension<AdminContext>,
    Json(req): Json<WithdrawFeesRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    if admin.role == "readonly" {
        return Err(ApiError::Unauthorized("Readonly admin cannot withdraw fees".into()));
    }

    if req.amount.trim().is_empty() || req.amount == "0" {
        return Err(ApiError::BadRequest("amount must be > 0".into()));
    }

    let (contract_address, chain_id) = state
        .config_repo
        .find_active_chain(&req.chain)
        .await?
        .ok_or_else(|| ApiError::BadRequest(format!("Chain '{}' not configured", req.chain)))?;

    if contract_address.is_empty() {
        return Err(ApiError::BadRequest(format!(
            "Contract not deployed for chain '{}'",
            req.chain
        )));
    }

    let _to: H160 = req
        .to
        .parse()
        .map_err(|_| ApiError::BadRequest("Invalid recipient address".into()))?;

    if req.token != "native" {
        let _token: H160 = req
            .token
            .parse()
            .map_err(|_| ApiError::BadRequest("Invalid token address".into()))?;
    }

    let data = abi::encode_withdraw_platform_fees(&req.token, &req.to, &req.amount);

    tracing::info!(
        "Admin {} prepared fee withdrawal: chain={} token={} to={} amount={}",
        admin.admin_id,
        req.chain,
        req.token,
        req.to,
        req.amount
    );

    Ok(Json(json!({
        "transaction": {
            "to": format!("0x{}", contract_address.trim_start_matches("0x")),
            "data": data,
            "value": "0"
        },
        "chain": req.chain,
        "chain_id": chain_id,
        "token": req.token,
        "amount": req.amount,
        "recipient": req.to,
    })))
}
