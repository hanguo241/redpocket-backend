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
    pub gross_amount: String,
    pub total_amount: String,
    pub remaining_amount: String,
    pub platform_fee_wei: String,
    pub head_count: i32,
    pub packet_type: String, // normal | password | condition
    pub sub_type: String,    // average | random
    pub claim_mode: String,  // self | proxy | both
    pub password_hash: Option<String>,
    pub start_time: i64,
    pub end_time: i64,
    pub signer_address: String,
    pub fee_bps: i32,
    pub fee_collector: String,
    pub status: String, // active | completed | expired | refunded
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
    pub claim_mode: Option<String>, // 默认 both
    pub start_time: Option<i64>,
    pub end_time: i64,
    /// 代领 gas 准备金 (wei) — 如不传则由后端按 multiplier 估算
    pub gas_reserve_wei: Option<String>,
}

/// 创建红包响应
#[derive(Debug, Serialize)]
pub struct CreatePacketResponse {
    pub packet_id: Uuid,
    pub transaction: TransactionData,
    pub share_url: String,
    pub expire_at: i64,
    /// 平台手续费基点，默认 20 = 0.2% / 千分之二
    pub fee_bps: i32,
    /// 创建红包时计提的平台手续费
    pub platform_fee_wei: String,
    /// 扣除平台手续费后的可领取红包池
    pub claim_pool_wei: String,
    /// 未领取本金可退款时间
    pub refund_available_at: i64,
    /// 预估总 gas 费 (wei) = gas_per_claim * head_count * gas_price * multiplier
    pub estimated_gas_fee_wei: String,
    /// 预估 gas 费 (ETH 单位，仅展示)
    pub estimated_gas_fee_eth: String,
    /// 当前 gas price (gwei)
    pub gas_price_gwei: String,
    /// gas 估算倍数
    pub gas_estimate_multiplier: f64,
    /// 建议 gas 准备金 (wei) = estimated_gas_fee_wei
    pub suggested_gas_reserve_wei: String,
}

/// 待签名的交易数据
#[derive(Debug, Serialize)]
pub struct TransactionData {
    pub to: String,
    pub data: String,
    pub value: String,
    /// Native: totalAmount + gasReserve 的总 value
    /// ERC20: gasReserve (wei)
    pub gas_reserve_wei: String,
}

/// 红包状态查询响应
#[derive(Debug, Serialize)]
pub struct PacketStatusResponse {
    pub packet_id: Uuid,
    pub status: String,
    pub gross_amount: String,
    pub total_amount: String,
    pub platform_fee_wei: String,
    pub claimed_amount: String,
    pub remaining_amount: String,
    pub claimed_count: i32,
    pub head_count: i32,
    pub claim_mode: String,
    pub refund_available_at: i64,
}
