use ethers::{
    core::k256::ecdsa::SigningKey,
    providers::{Http, Provider},
    signers::{Signer, Wallet},
};
use std::sync::Arc;

use crate::error::{ApiError, ApiResult};

/// 代领中继服务 — 由平台支付 gas 提交 claim 交易
#[derive(Clone)]
pub struct RelayerService {
    wallet: Wallet<SigningKey>,
    provider: Option<Arc<Provider<Http>>>,
}

impl RelayerService {
    pub fn new(private_key: &str, rpc_url: Option<&str>) -> ApiResult<Self> {
        let wallet: Wallet<SigningKey> = private_key
            .parse()
            .map_err(|e| ApiError::Crypto(format!("Invalid relayer private key: {}", e)))?;

        let provider = if let Some(url) = rpc_url {
            Some(Arc::new(
                Provider::<Http>::try_from(url)
                    .map_err(|e| ApiError::Internal(format!("Invalid RPC URL: {}", e)))?,
            ))
        } else {
            None
        };

        Ok(Self { wallet, provider })
    }

    /// 获取 Relayer 地址（代领钱包）
    pub fn address(&self) -> ethers::core::types::H160 {
        self.wallet.address()
    }

    /// 提交代领交易（占位 — 需在各链落地时实现具体调用逻辑）
    pub async fn submit_claim(
        &self,
        _contract_address: &str,
        _packet_id: ethers::core::types::U256,
        _recipient: ethers::core::types::H160,
        _amount: ethers::core::types::U256,
        _signature: &[u8],
    ) -> ApiResult<String> {
        tracing::info!(
            "Relayer would submit claim: packet={:?}, recipient={:?}, amount={:?}",
            _packet_id,
            _recipient,
            _amount
        );
        Ok("0x_pending_tx_hash_placeholder".to_string())
    }

    /// 检查 Relayer gas 余额是否充足
    pub async fn has_sufficient_gas(&self) -> bool {
        true
    }
}
