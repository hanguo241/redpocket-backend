use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ApiError;

/// JWT 声明
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub app_key: String,
    pub exp: usize,
}

/// 认证后的项目上下文
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub project_id: Uuid,
    pub app_key: String,
}

/// HMAC 签名验证 (用于 API 请求鉴权)
pub fn verify_hmac_signature(
    app_secret: &str,
    timestamp: &str,
    body: &str,
    signature: &str,
) -> Result<(), crate::error::ApiError> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(app_secret.as_bytes())
        .map_err(|_| crate::error::ApiError::Internal("HMAC initialization failed".into()))?;

    mac.update(timestamp.as_bytes());
    mac.update(body.as_bytes());

    let expected = hex::encode(mac.finalize().into_bytes());
    if expected != signature {
        return Err(crate::error::ApiError::Unauthorized("Invalid HMAC signature".into()));
    }

    Ok(())
}
