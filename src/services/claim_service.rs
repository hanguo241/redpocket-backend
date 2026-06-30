use std::sync::Arc;

use ethers::core::types::{H160, U256};
use rand::Rng;
use serde_json::json;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::repos::claim_repo::ClaimRepo;
use crate::repos::config_repo::ConfigRepo;
use crate::repos::packet_repo::PacketRepo;
use crate::services::abi;
use crate::services::relayer::RelayerService;
use crate::services::signer::SignerService;

/// 计算单个领取人应得的金额
///
/// - average: 均分 remaining / unclaimed_count
/// - random: 在 [avg×70%, avg×100%] 之间随机，最后一人领剩余
pub fn calculate_claim_amount(remaining: u128, unclaimed_count: u32, sub_type: &str) -> u128 {
    if unclaimed_count <= 1 {
        return remaining;
    }

    match sub_type {
        "random" => {
            let avg = remaining / unclaimed_count as u128;
            let min_ratio: u128 = 7000;
            let max_ratio: u128 = 10000;
            let min_amount = avg * min_ratio / 10000;
            let max_amount = avg * max_ratio / 10000;

            if min_amount >= max_amount || max_amount == 0 {
                return avg;
            }

            let mut rng = rand::rng();
            let range = max_amount - min_amount;
            let random_add = rng.random_range(0..=range);
            min_amount + random_add
        }
        _ => remaining / unclaimed_count as u128,
    }
}

/// 领取业务逻辑
pub struct ClaimService {
    pub claim_repo: ClaimRepo,
    pub packet_repo: PacketRepo,
    pub config_repo: ConfigRepo,
    pub signer: Arc<SignerService>,
    pub relayer: Arc<RelayerService>,
}

impl ClaimService {
    pub fn new(
        claim_repo: ClaimRepo,
        packet_repo: PacketRepo,
        config_repo: ConfigRepo,
        signer: Arc<SignerService>,
        relayer: Arc<RelayerService>,
    ) -> Self {
        Self {
            claim_repo,
            packet_repo,
            config_repo,
            signer,
            relayer,
        }
    }

    /// 准备领取：校验 + 金额计算 + 签名 + 构造 calldata
    pub async fn prepare(
        &self,
        packet_id: Uuid,
        user_addr: &str,
        proof_pw: Option<&str>,
    ) -> ApiResult<serde_json::Value> {
        let pkt = self
            .packet_repo
            .find_claim_info(packet_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Packet not found or not active".into()))?;

        if pkt.end_time <= chrono::Utc::now().timestamp() {
            return Err(ApiError::BadRequest("Packet has expired".into()));
        }

        // 校验口令
        if let Some(ref hash) = pkt.password_hash {
            if !hash.is_empty() {
                let input_hash = hex::encode(ethers::core::utils::keccak256(
                    proof_pw.unwrap_or("").as_bytes(),
                ));
                if input_hash != *hash {
                    return Err(ApiError::BadRequest("Invalid password".into()));
                }
            }
        }

        // 查链配置
        let (_chain, contract_addr_str) = self
            .config_repo
            .find_by_packet_id(packet_id)
            .await?
            .ok_or_else(|| ApiError::BadRequest("Chain not configured".into()))?;

        let contract_addr: H160 = contract_addr_str
            .parse()
            .map_err(|_| ApiError::BadRequest("Invalid contract address".into()))?;
        let chain_id = self
            .config_repo
            .find_chain_id_by_contract(&contract_addr_str)
            .await?
            .ok_or_else(|| ApiError::BadRequest("Chain config not found".into()))?
            as u64;

        // 计算金额
        fn parse_wei(s: &str) -> ApiResult<u128> {
            s.parse().map_err(|_| ApiError::Internal(
                format!("Invalid wei amount in database: {}", s)
            ))
        }
        let remaining_wei = parse_wei(&pkt.remaining_amount)?;
        let unclaimed = pkt.head_count - pkt.claimed_count;

        if unclaimed <= 0 {
            return Err(ApiError::BadRequest("Packet fully claimed".into()));
        }

        let amount = calculate_claim_amount(remaining_wei, unclaimed as u32, &pkt.sub_type);
        let amount_str = amount.to_string();

        let nonce_num = chrono::Utc::now().timestamp_millis() as u64;
        let deadline_num = (chrono::Utc::now().timestamp() + 1800) as u64;
        let recipient: H160 = user_addr
            .parse()
            .map_err(|_| ApiError::BadRequest("Invalid user address".into()))?;

        // 获取 onchain_packet_id
        let onchain_id = self.packet_repo.find_onchain_id(packet_id).await?;
        let chain_pid = onchain_id
            .filter(|&v| v > 0)
            .map(|v| v as u64)
            .ok_or_else(|| ApiError::BadRequest("Packet transaction is not indexed yet".into()))?;

        // 签发签名
        let signature = self
            .signer
            .sign_claim(
                U256::from(chain_pid),
                recipient,
                U256::from(amount),
                U256::from(nonce_num),
                U256::from(deadline_num),
                chain_id,
                contract_addr,
            )
            .await?;
        let sig_hex = hex::encode(&signature[..]);

        // 构造 calldata
        let calldata = abi::encode_claim(
            &format!("0x{:x}", chain_pid),
            user_addr,
            &amount_str,
            &nonce_num.to_string(),
            &deadline_num.to_string(),
            &format!("0x{}", sig_hex),
        );

        // 记录 pending
        self.claim_repo
            .create_pending(
                packet_id,
                user_addr,
                &amount_str,
                &nonce_num.to_string(),
                &sig_hex,
                "self",
            )
            .await?;

        Ok(json!({
            "packet_id": packet_id,
            "amount": amount_str,
            "signature": format!("0x{}", sig_hex),
            "nonce": nonce_num,
            "deadline": deadline_num,
            "transaction": {
                "to": format!("0x{}", contract_addr.to_fixed_bytes().iter().map(|b| format!("{:02x}", b)).collect::<String>()),
                "data": calldata,
                "value": "0"
            }
        }))
    }

    /// 确认自领
    pub async fn confirm(
        &self,
        packet_id: Uuid,
        recipient: &str,
        tx_hash: &str,
    ) -> ApiResult<serde_json::Value> {
        self.claim_repo
            .confirm(packet_id, recipient, tx_hash)
            .await?;
        Ok(json!({"status": "confirmed", "tx_hash": tx_hash}))
    }

    /// 代领：验证用户授权 → 签发签名 → claimFor 广播
    pub async fn proxy_claim(
        &self,
        packet_id: Uuid,
        user_addr: &str,
        user_sig: &str,
        proof_pw: Option<&str>,
    ) -> ApiResult<serde_json::Value> {
        // 1. 验证用户授权签名
        let auth_msg = format!(
            "\x19Ethereum Signed Message:\n{}RedPacket: authorize claim {}",
            (27 + packet_id.to_string().len()), // ← 修复 blocker: 27 而非 32
            packet_id
        );
        let auth_hash = ethers::core::utils::keccak256(auth_msg.as_bytes());
        let sig_bytes = hex::decode(user_sig.trim_start_matches("0x"))
            .map_err(|_| ApiError::BadRequest("Invalid signature format".into()))?;

        let recovered = ethers::core::types::Signature::try_from(sig_bytes.as_slice())
            .and_then(|s| s.recover(auth_hash))
            .map_err(|_| ApiError::BadRequest("Signature recovery failed".into()))?;

        let recovered_str = format!("{:?}", recovered).to_lowercase();
        let user_addr_normalized = user_addr.trim_start_matches("0x").to_lowercase();
        let recovered_normalized = recovered_str.trim_start_matches("0x").to_lowercase();

        if recovered_normalized != user_addr_normalized {
            return Err(ApiError::Unauthorized("Invalid user signature".into()));
        }

        // 2. 查红包
        let pkt = self
            .packet_repo
            .find_claim_info(packet_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Packet not found or not active".into()))?;

        if pkt.end_time <= chrono::Utc::now().timestamp() {
            return Err(ApiError::BadRequest("Packet has expired".into()));
        }

        // 口令校验
        if let Some(ref hash) = pkt.password_hash {
            if !hash.is_empty() {
                let input_hash = hex::encode(ethers::core::utils::keccak256(
                    proof_pw.unwrap_or("").as_bytes(),
                ));
                if input_hash != *hash {
                    return Err(ApiError::BadRequest("Invalid password".into()));
                }
            }
        }

        // 3. 查链配置
        let (_chain, contract_addr_str) = self
            .config_repo
            .find_by_packet_id(packet_id)
            .await?
            .ok_or_else(|| ApiError::BadRequest("Chain not configured".into()))?;

        let contract_addr: H160 = contract_addr_str
            .parse()
            .map_err(|_| ApiError::BadRequest("Invalid contract address".into()))?;
        let chain_id = self
            .config_repo
            .find_chain_id_by_contract(&contract_addr_str)
            .await?
            .ok_or_else(|| ApiError::BadRequest("Chain config not found".into()))?
            as u64;

        // 4. 计算金额
        fn parse_wei(s: &str) -> ApiResult<u128> {
            s.parse().map_err(|_| ApiError::Internal(
                format!("Invalid wei amount in database: {}", s)
            ))
        }
        let remaining_wei = parse_wei(&pkt.remaining_amount)?;
        let unclaimed = pkt.head_count - pkt.claimed_count;

        if unclaimed <= 0 {
            return Err(ApiError::BadRequest("Packet fully claimed".into()));
        }

        let amount = calculate_claim_amount(remaining_wei, unclaimed as u32, &pkt.sub_type);
        let amount_str = amount.to_string();

        let nonce_num = chrono::Utc::now().timestamp_millis() as u64;
        let deadline_num = (chrono::Utc::now().timestamp() + 1800) as u64;
        let recipient: H160 = user_addr
            .parse()
            .map_err(|_| ApiError::BadRequest("Invalid user address".into()))?;

        let onchain_id = self.packet_repo.find_onchain_id(packet_id).await?;
        let chain_pid = onchain_id
            .filter(|&v| v > 0)
            .map(|v| v as u64)
            .ok_or_else(|| ApiError::BadRequest("Packet transaction is not indexed yet".into()))?;

        // 5. 签发签名
        let signature = self
            .signer
            .sign_claim(
                U256::from(chain_pid),
                recipient,
                U256::from(amount),
                U256::from(nonce_num),
                U256::from(deadline_num),
                chain_id,
                contract_addr,
            )
            .await?;
        let sig_hex = hex::encode(&signature[..]);

        // 6. 构造 claimFor calldata
        let calldata = abi::encode_claim_for(
            &format!("0x{:x}", chain_pid),
            user_addr,
            &amount_str,
            &nonce_num.to_string(),
            &deadline_num.to_string(),
            &format!("0x{}", sig_hex),
        );

        // 7. 获取 RPC URL
        let rpc_url = self
            .config_repo
            .find_rpc_by_contract(&contract_addr_str)
            .await?
            .ok_or_else(|| ApiError::Internal("RPC URL not configured".into()))?;

        // 8. 用 Relayer 钱包广播
        let contract_addr_hex = format!(
            "0x{}",
            contract_addr
                .to_fixed_bytes()
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>()
        );
        let tx_hash = self.relayer.submit_claim_for(
            &rpc_url,
            &contract_addr_hex,
            &calldata,
            0x50000, // gas limit
        ).await?;

        // 9. 记录
        self.claim_repo
            .confirm_proxy(
                packet_id,
                user_addr,
                &amount_str,
                &nonce_num.to_string(),
                &sig_hex,
                &tx_hash,
            )
            .await?;

        tracing::info!(
            "Proxy claim: packet={} user={} tx={}",
            packet_id,
            user_addr,
            tx_hash
        );

        Ok(json!({
            "status": "confirmed",
            "tx_hash": tx_hash,
            "packet_id": packet_id,
            "amount": amount_str,
        }))
    }
}
