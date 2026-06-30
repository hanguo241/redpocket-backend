use axum::{
    body::{to_bytes, Body},
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    admin::auth::AdminClaims,
    app::AppState,
    error::{ApiError, ApiResult},
};

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

/// 认证后的管理员上下文
#[derive(Debug, Clone)]
pub struct AdminContext {
    pub admin_id: Uuid,
    pub role: String,
}

/// HMAC 签名验证 (用于 API 请求鉴权)
pub fn verify_hmac_signature(
    app_secret: &str,
    timestamp: &str,
    body: &str,
    signature: &str,
) -> Result<(), ApiError> {
    verify_hmac_signature_bytes(app_secret, timestamp, body.as_bytes(), signature)
}

/// HMAC 签名验证，使用原始 body bytes 避免 JSON 格式化差异
pub fn verify_hmac_signature_bytes(
    app_secret: &str,
    timestamp: &str,
    body: &[u8],
    signature: &str,
) -> Result<(), ApiError> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(app_secret.as_bytes())
        .map_err(|_| ApiError::Internal("HMAC initialization failed".into()))?;

    mac.update(timestamp.as_bytes());
    mac.update(body);

    let expected = hex::encode(mac.finalize().into_bytes());
    if expected != signature {
        return Err(ApiError::Unauthorized("Invalid HMAC signature".into()));
    }

    Ok(())
}

/// Admin API JWT middleware.
pub async fn require_admin(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> ApiResult<Response> {
    let token = bearer_token(&req)?;
    let claims = decode::<AdminClaims>(
        token,
        &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| ApiError::Unauthorized("Invalid admin token".into()))?
    .claims;

    let active: Option<bool> = sqlx::query_scalar(
        "SELECT is_active FROM admin_users WHERE id=$1 AND role=$2",
    )
    .bind(claims.sub)
    .bind(&claims.role)
    .fetch_optional(&state.db)
    .await?;

    if active != Some(true) {
        return Err(ApiError::Unauthorized("Admin account is inactive".into()));
    }

    req.extensions_mut().insert(AdminContext {
        admin_id: claims.sub,
        role: claims.role,
    });

    Ok(next.run(req).await)
}

/// Merchant API HMAC middleware.
pub async fn require_merchant(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> ApiResult<Response> {
    let app_key = header_value(&req, "x-app-key")?.to_string();
    let signature = header_value(&req, "x-signature")?.to_string();
    let timestamp = header_value(&req, "x-timestamp")?.to_string();
    validate_timestamp(&timestamp)?;

    let (mut parts, body) = req.into_parts();
    let body_bytes = to_bytes(body, 1024 * 1024)
        .await
        .map_err(|_| ApiError::BadRequest("Failed to read request body".into()))?;

    let project = state
        .project_repo
        .find_auth_by_app_key(&app_key)
        .await?
        .ok_or_else(|| ApiError::Unauthorized("Invalid AppKey".into()))?;

    verify_hmac_signature_bytes(
        &project.app_secret,
        &timestamp,
        &body_bytes,
        &signature,
    )?;

    parts.extensions.insert(AuthContext {
        project_id: project.id,
        app_key,
    });

    let req = Request::from_parts(parts, Body::from(body_bytes));
    Ok(next.run(req).await)
}

fn bearer_token(req: &Request) -> ApiResult<&str> {
    let value = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::Unauthorized("Authorization header required".into()))?;

    value
        .strip_prefix("Bearer ")
        .ok_or_else(|| ApiError::Unauthorized("Bearer token required".into()))
}

fn header_value<'a>(req: &'a Request, name: &str) -> ApiResult<&'a str> {
    req.headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::Unauthorized(format!("{} header required", name)))
}

fn validate_timestamp(timestamp: &str) -> ApiResult<()> {
    let ts_ms: i64 = timestamp
        .parse()
        .map_err(|_| ApiError::Unauthorized("Invalid timestamp".into()))?;
    let now_ms = chrono::Utc::now().timestamp_millis();
    let skew_ms = (now_ms - ts_ms).abs();

    if skew_ms > 5 * 60 * 1000 {
        return Err(ApiError::Unauthorized("Timestamp expired".into()));
    }

    Ok(())
}
