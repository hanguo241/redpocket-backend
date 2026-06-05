use std::sync::Arc;

use ethers::core::types::H160;
use serde_json::json;
use uuid::Uuid;

use crate::config::Config;
use crate::error::{ApiError, ApiResult};
use crate::repos::config_repo::ConfigRepo;
use crate::repos::packet_repo::{ClaimPacketInfo, PacketRepo};
use crate::services::abi;
use crate::services::signer::SignerService;
use crate::AppState;

/// 计算 share_url 的基础地址
fn share_url_base(cfg: &Config) -> String {
    cfg.share_url_host.clone()
}

/// 从 RPC 获取当前 gas price (wei)，失败返回默认 10 gwei
async fn fetch_gas_price(rpc_url: &str) -> u128 {
    let client = reqwest::Client::new();
    let resp = client
        .post(rpc_url)
        .json(&json!({
            "jsonrpc": "2.0", "method": "eth_gasPrice", "params": [], "id": 1
        }))
        .send()
        .await; // TODO: 从 chain config 读取默认值而非硬编码 10 gwei

    match resp {
        Ok(resp) => {
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                if let Some(result) = body.get("result").and_then(|r| r.as_str()) {
                    let trimmed = result.trim_start_matches("0x");
                    if let Ok(val) = u128::from_str_radix(trimmed, 16) {
                        return val;
                    }
                }
            }
            10_000_000_000
        }
        Err(_) => 10_000_000_000,
    }
}

/// 红包业务逻辑
pub struct PacketService {
    pub packet_repo: PacketRepo,
    pub config_repo: ConfigRepo,
    pub signer: Arc<SignerService>,
    pub config: Arc<Config>,
}

impl PacketService {
    pub fn new(
        packet_repo: PacketRepo,
        config_repo: ConfigRepo,
        signer: Arc<SignerService>,
        config: Arc<Config>,
    ) -> Self {
        Self { packet_repo, config_repo, signer, config }
    }

    /// 准备创建红包：校验 + ABI 编码 + gas 估算
    pub async fn prepare(
        &self,
        chain: &str,
        token: &str,
        total_amount: &str,
        head_count: i32,
        packet_type: &str,
        sub_type: &str,
        start_time: Option<i64>,
        end_time: i64,
        gas_reserve_wei: Option<String>,
    ) -> ApiResult<serde_json::Value> {
        if total_amount.is_empty() {
            return Err(ApiError::BadRequest("total_amount is required".into()));
        }
        if head_count <= 0 {
            return Err(ApiError::BadRequest("head_count must be > 0".into()));
        }
        if end_time <= chrono::Utc::now().timestamp() {
            return Err(ApiError::BadRequest("end_time must be in the future".into()));
        }

        let (contract_address, _) = self.config_repo.find_active_chain(chain).await?
            .ok_or_else(|| ApiError::BadRequest(format!("Chain '{}' not configured", chain)))?;
        if contract_address.is_empty() {
            return Err(ApiError::BadRequest(format!("Contract not deployed for chain '{}'", chain)));
        }

        let signer_address = format!("{:?}", self.signer.address());
        let fee_collector = signer_address.clone();

        // gas 估算
        let (gas_per_claim, multiplier) = self.config_repo.load_gas_config().await?;
        let rpc_url = self.config_repo.find_rpc_url(chain).await?.unwrap_or_default();
        let gas_price_wei = if !rpc_url.is_empty() {
            fetch_gas_price(&rpc_url).await
        } else {
            10_000_000_000
        };

        let estimated_gas = (gas_per_claim as f64) * (head_count as f64) * multiplier;
        let estimated_gas_wei = (estimated_gas as u128) * gas_price_wei;
        let estimated_gas_eth = format!("{}", estimated_gas as f64 / 1e18);
        let gas_price_gwei = format!("{}", gas_price_wei as f64 / 1e9);
        let gas_reserve = gas_reserve_wei.unwrap_or_else(|| estimated_gas_wei.to_string());

        // ABI 编码
        let ptype = match packet_type { "password" => 1u8, "condition" => 2u8, _ => 0u8 };
        let stype = match sub_type { "random" => 1u8, _ => 0u8 };

        let data = if token == "native" {
            abi::encode_create_packet_native(
                head_count as u32, ptype, stype, &signer_address,
                start_time.unwrap_or(0) as u64, end_time as u64,
                7000, 10000, 20, &fee_collector, &gas_reserve,
            )
        } else {
            abi::encode_create_packet_erc20(
                token, total_amount, head_count as u32, ptype, stype, &signer_address,
                start_time.unwrap_or(0) as u64, end_time as u64,
                7000, 10000, 20, &fee_collector, &gas_reserve,
            )
        };

        // 计算交易 value
        let total_value = if token == "native" {
            let amount_num: u128 = total_amount.parse().unwrap_or(0);
            let gas_num: u128 = gas_reserve.parse().unwrap_or(0);
            (amount_num + gas_num).to_string()
        } else {
            gas_reserve.clone()
        };

        let pid = Uuid::new_v4();
        let host = share_url_base(&self.config);
        let share_url = format!("{}/claim/{}", host, pid);

        Ok(json!({
            "packet_id": pid,
            "transaction": {
                "to": format!("0x{}", contract_address.trim_start_matches("0x")),
                "data": data,
                "value": total_value,
                "gas_reserve_wei": gas_reserve,
            },
            "share_url": share_url,
            "expire_at": end_time,
            "estimated_gas_fee_wei": estimated_gas_wei.to_string(),
            "estimated_gas_fee_eth": estimated_gas_eth,
            "gas_price_gwei": gas_price_gwei,
            "gas_estimate_multiplier": multiplier,
            "suggested_gas_reserve_wei": gas_reserve,
        }))
    }

    /// 确认创建：落库 + 异步获取 onchain_id
    pub async fn create(
        &self,
        packet_id: Uuid,
        tx_hash: &str,
        creator: &str,
        chain: &str,
        contract_addr: &str,
        token: &str,
        total_amount: &str,
        head_count: i32,
        packet_type: &str,
        sub_type: &str,
        claim_mode: &str,
        password: Option<&str>,
        start_time: i64,
        end_time: i64,
        signer_address: &str,
        fee_bps: i32,
        fee_collector: &str,
        gas_reserve_wei: &str,
        gas_estimate_multiplier: f64,
        db: sqlx::PgPool,
    ) -> ApiResult<serde_json::Value> {
        let password_hash = password
            .map(|p| hex::encode(ethers::core::utils::keccak256(p.as_bytes())));

        self.packet_repo.create(
            packet_id, chain, contract_addr, creator, token,
            total_amount, head_count, packet_type, sub_type, claim_mode,
            &password_hash, start_time, end_time, signer_address,
            fee_bps, fee_collector, gas_reserve_wei, gas_estimate_multiplier,
        ).await?;

        tracing::info!(
            "Packet recorded: {} tx={} creator={} chain={} gas_reserve={}",
            packet_id, tx_hash, creator, chain, gas_reserve_wei,
        );

        // 异步获取 onchain_packet_id
        let pid2 = packet_id;
        let ch = chain.to_string();
        let txh = tx_hash.to_string();
        let db2 = db;
        tokio::spawn(async move {
            let rpc = sqlx::query_scalar::<_, String>(
                "SELECT rpc_url FROM chain_configs WHERE chain=$1 AND is_active=true",
            )
            .bind(&ch)
            .fetch_optional(&db2)
            .await;
            if let Ok(Some(rpc_url)) = rpc {
                if !rpc_url.is_empty() {
                    match crate::services::receipt::fetch_onchain_packet_id(&rpc_url, &txh).await {
                        Ok(Some(onchain_id)) => {
                            let _ = sqlx::query("UPDATE packets SET onchain_packet_id=$1 WHERE id=$2")
                                .bind(onchain_id as i64)
                                .bind(pid2)
                                .execute(&db2)
                                .await;
                            tracing::info!("onchain_packet_id={} for packet={}", onchain_id, pid2);
                        }
                        _ => tracing::warn!("Could not fetch onchain_packet_id for packet={}", pid2),
                    }
                }
            }
        });

        let host = share_url_base(&self.config);
        let share_url = format!("{}/claim/{}", host, packet_id);

        Ok(json!({
            "packet_id": packet_id,
            "status": "active",
            "tx_hash": tx_hash,
            "share_url": share_url,
        }))
    }

    /// 查询红包状态
    pub async fn get_status(&self, id: Uuid) -> ApiResult<serde_json::Value> {
        let row = self.packet_repo.get_status(id).await?
            .ok_or_else(|| ApiError::NotFound("Packet not found".into()))?;

        Ok(json!({
            "packet_id": id,
            "status": row.status,
            "total_amount": row.total_amount,
            "claimed_amount": row.claimed_amount,
            "claimed_count": row.claimed_count,
            "head_count": row.head_count,
            "claim_mode": row.claim_mode,
            "remaining_amount": row.remaining_amount,
            "gas_reserve_wei": row.gas_reserve_wei,
            "gas_used_wei": row.gas_used_wei,
        }))
    }
}
