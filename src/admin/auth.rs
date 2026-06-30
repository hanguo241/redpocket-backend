use axum::{extract::State, Json};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use redpacket_backend::error::{ApiError, ApiResult};
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub admin_id: Uuid,
    pub name: String,
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdminClaims {
    pub sub: Uuid,
    pub role: String,
    pub exp: usize,
}

/// POST /api/v1/admin/login
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let admin = state
        .admin_repo
        .find_active_by_email(&req.email)
        .await?
        .ok_or_else(|| ApiError::Unauthorized("Invalid email or password".into()))?;

    let valid = bcrypt::verify(&req.password, &admin.password_hash).unwrap_or(false);
    if !valid {
        return Err(ApiError::Unauthorized("Invalid email or password".into()));
    }

    let exp = (chrono::Utc::now().timestamp() + 86400) as usize; // 24h
    let claims = AdminClaims {
        sub: admin.id,
        role: admin.role.clone(),
        exp,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.config.jwt_secret.as_bytes()),
    )
    .map_err(|e| ApiError::Internal(format!("JWT error: {}", e)))?;

    Ok(Json(json!(LoginResponse {
        token,
        admin_id: admin.id,
        name: admin.name,
        role: admin.role,
    })))
}
