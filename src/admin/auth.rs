use axum::{extract::State, Json};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
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
    let admin = sqlx::query_as::<_, (Uuid, String, String, String)>(
        "SELECT id, password_hash, name, role FROM admin_users WHERE email=$1 AND is_active=true",
    )
    .bind(&req.email)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::Unauthorized("Invalid email or password".into()))?;

    let valid = bcrypt::verify(&req.password, &admin.1).unwrap_or(false);
    if !valid {
        return Err(ApiError::Unauthorized("Invalid email or password".into()));
    }

    let exp = (chrono::Utc::now().timestamp() + 86400) as usize; // 24h
    let claims = AdminClaims { sub: admin.0, role: admin.3.clone(), exp };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.config.jwt_secret.as_bytes()),
    )
    .map_err(|e| ApiError::Internal(format!("JWT error: {}", e)))?;

    Ok(Json(json!(LoginResponse {
        token,
        admin_id: admin.0,
        name: admin.2,
        role: admin.3,
    })))
}
