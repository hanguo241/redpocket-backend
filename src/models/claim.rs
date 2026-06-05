use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 领取记录（数据库）
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Claim {
    pub id: Uuid,
    pub packet_id: Uuid,
    pub recipient_address: String,
    pub amount: String,
    pub fee: String,
    pub nonce: i64,
    pub signature: String,
    pub claim_type: String,        // self | proxy
    pub tx_hash: Option<String>,
    pub status: String,            // pending | confirmed | failed
    pub created_at: DateTime<Utc>,
}

/// 请求 claim 签名
#[derive(Debug, Deserialize)]
pub struct RequestClaimSignRequest {
    pub packet_id: Uuid,
    pub user_address: String,
    pub proof: Option<ClaimProof>,
}

#[derive(Debug, Deserialize)]
pub struct ClaimProof {
    pub password: Option<String>,
    pub captcha_token: Option<String>,
}

/// Claim 签名响应
#[derive(Debug, Serialize)]
pub struct ClaimSignResponse {
    pub packet_id: Uuid,
    pub recipient: String,
    pub amount: String,
    pub signature: String,
    pub nonce: i64,
    pub deadline: i64,
}

/// 代领请求
#[derive(Debug, Deserialize)]
pub struct ProxyClaimRequest {
    pub packet_id: Uuid,
    pub recipient: String,
    pub amount: String,
    pub signature: String,
    pub nonce: i64,
    pub deadline: i64,
    pub user_authorization: String,  // 用户授权平台代领的签名
}

/// 代领响应
#[derive(Debug, Serialize)]
pub struct ProxyClaimResponse {
    pub claim_id: Uuid,
    pub tx_hash: Option<String>,
    pub status: String,
}
