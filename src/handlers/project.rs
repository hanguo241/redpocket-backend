use axum::{extract::State, Json};

use redpacket_backend::error::ApiResult;
use redpacket_backend::models::project::{RegisterRequest, RegisterResponse};
use crate::AppState;

/// POST /api/v1/project/register
///
/// 项目方注册 — 连接钱包 + 签名消息验证
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> ApiResult<Json<RegisterResponse>> {
    let output = state.project_service.register(req).await?;

    Ok(Json(RegisterResponse {
        project_id: output.project_id,
        app_key: output.app_key,
        app_secret: output.app_secret,
        message: output.message,
    }))
}
