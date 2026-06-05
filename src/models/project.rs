use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 项目方注册信息
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub app_key: String,
    pub app_secret: String,
    pub wallet_address: String,
    pub website: String,
    pub contact: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 注册请求
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub wallet_address: String,
    pub signature: String,
    pub website: Option<String>,
    pub contact: Option<String>,
}

/// 注册响应
#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub project_id: Uuid,
    pub app_key: String,
    pub app_secret: String,
    pub message: String,
}
