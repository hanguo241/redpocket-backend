use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 红包记录（数据库）
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Packet {
    pub id: Uuid,
    pub project_id: Uuid,
    pub chain: String,
    pub contract_address: String,
    pub creator_address: String,
    pub token_address: String,
    pub total_amount: String,
    pub remaining_amount: String,
    pub head_count: i32,
    pub packet_type: String,       // normal | password | condition
    pub sub_type: String,          // average | random
    pub claim_mode: String,        // self | proxy | both
    pub password_hash: Option<String>,
    pub start_time: i64,
    pub end_time: i64,
    pub signer_address: String,
    pub fee_bps: i32,
    pub fee_collector: String,
    pub status: String,            // active | completed | expired | refunded
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建红包请求
#[derive(Debug, Deserialize)]
pub struct CreatePacketRequest {
    pub chain: String,
    pub token: String,
    pub total_amount: String,
    pub head_count: i32,
    pub packet_type: String,
    pub sub_type: String,
    pub password: Option<String>,
    pub claim_mode: Option<String>,  // 默认 both
    pub start_time: Option<i64>,
    pub end_time: i64,
}

/// 创建红包响应
#[derive(Debug, Serialize)]
pub struct CreatePacketResponse {
    pub packet_id: Uuid,
    pub transaction: TransactionData,
    pub share_url: String,
    pub expire_at: i64,
}

/// 待签名的交易数据
#[derive(Debug, Serialize)]
pub struct TransactionData {
    pub to: String,
    pub data: String,
    pub value: String,
}

/// 红包状态查询响应
#[derive(Debug, Serialize)]
pub struct PacketStatusResponse {
    pub packet_id: Uuid,
    pub status: String,
    pub total_amount: String,
    pub claimed_amount: String,
    pub remaining_amount: String,
    pub claimed_count: i32,
    pub head_count: i32,
    pub claim_mode: String,
}
