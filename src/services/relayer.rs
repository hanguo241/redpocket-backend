use ethers::{
    core::k256::ecdsa::SigningKey,
    signers::{Signer, Wallet},
};
use serde_json::json;

use crate::error::{ApiError, ApiResult};

/// 代领中继服务 — 由平台支付 gas 提交 claim 交易
#[derive(Clone)]
pub struct RelayerService {
    wallet: Wallet<SigningKey>,
    http_client: reqwest::Client,
}

impl RelayerService {
    pub fn new(private_key: &str, _rpc_url: Option<&str>, http_client: reqwest::Client) -> ApiResult<Self> {
        let wallet: Wallet<SigningKey> = private_key
            .parse()
            .map_err(|e| ApiError::Crypto(format!("Invalid relayer private key: {}", e)))?;

        Ok(Self {
            wallet,
            http_client,
        })
    }

    /// 获取 Relayer 地址（代领钱包）
    pub fn address(&self) -> ethers::core::types::H160 {
        self.wallet.address()
    }

    /// 提交 claimFor 交易 — 由 relayer 支付 gas
    pub async fn submit_claim_for(
        &self,
        rpc_url: &str,
        contract_address: &str,
        calldata: &str,
        gas_limit: u64,
    ) -> ApiResult<String> {
        let from = format!("{:?}", self.wallet.address());
        let to = format!("0x{}", contract_address.trim_start_matches("0x"));

        let rpc_resp = self
            .http_client
            .post(rpc_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "eth_sendTransaction",
                "params": [{
                    "from": from,
                    "to": to,
                    "data": calldata,
                    "gas": format!("0x{:x}", gas_limit),
                }],
                "id": 1
            }))
            .send()
            .await
            .map_err(|e| ApiError::Internal(format!("RPC call failed: {}", e)))?;

        let rpc_body: serde_json::Value = rpc_resp
            .json()
            .await
            .map_err(|e| ApiError::Internal(format!("RPC parse error: {}", e)))?;

        if let Some(err) = rpc_body.get("error") {
            return Err(ApiError::Internal(format!(
                "RPC error: {}",
                err.get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown")
            )));
        }

        rpc_body
            .get("result")
            .and_then(|r| r.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| ApiError::Internal("RPC did not return tx_hash".into()))
    }

    /// 获取 relayer 在目标链上的 ETH 余额，用于判断 gas 是否充足
    pub async fn get_balance(&self, rpc_url: &str) -> ApiResult<u128> {
        let addr = format!("{:?}", self.wallet.address());

        let resp: serde_json::Value = self
            .http_client
            .post(rpc_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "eth_getBalance",
                "params": [format!("0x{}", addr.trim_start_matches("0x")), "latest"],
                "id": 1
            }))
            .send()
            .await
            .map_err(|e| ApiError::Internal(format!("RPC balance call failed: {}", e)))?
            .json()
            .await
            .map_err(|e| ApiError::Internal(format!("RPC balance parse failed: {}", e)))?;

        let hex_balance = resp
            .get("result")
            .and_then(|r| r.as_str())
            .ok_or_else(|| ApiError::Internal("RPC did not return balance".into()))?;

        u128::from_str_radix(hex_balance.trim_start_matches("0x"), 16)
            .map_err(|e| ApiError::Internal(format!("Invalid balance hex: {}", e)))
    }

    /// 检查 Relayer gas 余额是否充足（余额 > 0.01 ETH 视为充足）
    pub async fn has_sufficient_gas(&self, rpc_url: &str) -> bool {
        match self.get_balance(rpc_url).await {
            Ok(balance) => balance > 10_000_000_000_000_000, // 0.01 ETH
            Err(_) => false,
        }
    }
}
