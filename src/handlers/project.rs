use axum::{extract::State, Json};
use ethers::core::types::H160;
use serde_json::json;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    models::project::{RegisterRequest, RegisterResponse},
    AppState,
};

/// POST /api/v1/project/register
///
/// 项目方注册 — 连接钱包 + 签名消息验证
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    if req.name.trim().is_empty() {
        return Err(ApiError::BadRequest("项目名称不能为空".into()));
    }
    if req.wallet_address.trim().is_empty() {
        return Err(ApiError::BadRequest("钱包地址不能为空".into()));
    }

    // 验证钱包签名: 用户签名 "RedPacket Register: {wallet_address}"
    let wallet: H160 = req
        .wallet_address
        .parse()
        .map_err(|_| ApiError::BadRequest("无效的钱包地址".into()))?;

    let msg = format!("\x19Ethereum Signed Message:\n{}RedPacket Register: {}", 
        (20 + req.wallet_address.len()), req.wallet_address);

    let msg_hash = ethers::core::utils::keccak256(msg.as_bytes());
    let sig_bytes = hex::decode(req.signature.trim_start_matches("0x"))
        .map_err(|_| ApiError::BadRequest("无效的签名格式".into()))?;

    let recovered = ethers::core::types::Signature::try_from(sig_bytes.as_slice())
        .and_then(|s| s.recover(msg_hash))
        .map_err(|_| ApiError::BadRequest("签名恢复失败".into()))?;

    if recovered != wallet {
        return Err(ApiError::Unauthorized("钱包签名不匹配".into()));
    }

    let project_id = Uuid::new_v4();
    let app_key = Uuid::new_v4().to_string().replace("-", "");
    let app_secret = Uuid::new_v4().to_string().replace("-", "");
    let website = req.website.unwrap_or_default();
    let contact = req.contact.unwrap_or_default();

    sqlx::query(
        r#"
        INSERT INTO projects (id, name, app_key, app_secret, wallet_address, website, contact)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(project_id)
    .bind(req.name.trim())
    .bind(&app_key)
    .bind(&app_secret)
    .bind(req.wallet_address.trim())
    .bind(&website)
    .bind(&contact)
    .execute(&state.db)
    .await?;

    tracing::info!("Project registered: {} ({}) wallet={}", req.name, project_id, req.wallet_address);

    Ok(Json(json!(RegisterResponse {
        project_id,
        app_key,
        app_secret,
        message: "注册成功！请妥善保存 AppSecret，不会再次显示。".to_string(),
    })))
}
