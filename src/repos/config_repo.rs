use sqlx::PgPool;

use crate::error::ApiResult;

#[derive(Clone)]
pub struct ConfigRepo {
    db: PgPool,
}

impl ConfigRepo {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// 获取活跃链的合约地址和 chain_id
    pub async fn find_active_chain(&self, chain: &str) -> ApiResult<Option<(String, i64)>> {
        let row = sqlx::query_as::<_, (String, i64)>(
            "SELECT contract_address, chain_id FROM chain_configs WHERE chain = $1 AND is_active = true",
        )
        .bind(chain)
        .fetch_optional(&self.db)
        .await?;
        Ok(row)
    }

    /// 获取链的 RPC URL
    pub async fn find_rpc_url(&self, chain: &str) -> ApiResult<Option<String>> {
        let url: Option<String> = sqlx::query_scalar(
            "SELECT rpc_url FROM chain_configs WHERE chain = $1 AND is_active = true",
        )
        .bind(chain)
        .fetch_optional(&self.db)
        .await?;
        Ok(url)
    }

    /// 按合约地址查找链信息
    pub async fn find_by_contract(&self, contract_address: &str) -> ApiResult<Option<ChainContractRow>> {
        let row = sqlx::query_as::<_, (String, String, i64)>(
            "SELECT p.chain, cc.contract_address, cc.chain_id
             FROM chain_configs cc
             WHERE cc.contract_address = $1",
        )
        .bind(contract_address)
        .fetch_optional(&self.db)
        .await?;
        Ok(row.map(|r| ChainContractRow { chain: r.0, contract_address: r.1, chain_id: r.2 }))
    }

    /// 按红包 ID 查找链配置
    pub async fn find_by_packet_id(&self, packet_id: uuid::Uuid) -> ApiResult<Option<(String, String)>> {
        let row = sqlx::query_as::<_, (String, String)>(
            "SELECT p.chain, cc.contract_address
             FROM packets p JOIN chain_configs cc ON cc.chain=p.chain WHERE p.id=$1",
        )
        .bind(packet_id)
        .fetch_optional(&self.db)
        .await?;
        Ok(row)
    }

    /// 获取 chain_id (by contract)
    pub async fn find_chain_id_by_contract(&self, contract_address: &str) -> ApiResult<Option<i64>> {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT chain_id FROM chain_configs WHERE contract_address=$1",
        )
        .bind(contract_address)
        .fetch_optional(&self.db)
        .await?;
        Ok(row.map(|r| r.0))
    }

    /// 按合约地址找 RPC URL
    pub async fn find_rpc_by_contract(&self, contract_address: &str) -> ApiResult<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT rpc_url FROM chain_configs WHERE contract_address=$1",
        )
        .bind(contract_address)
        .fetch_optional(&self.db)
        .await?;
        Ok(row.map(|r| r.0))
    }

    /// 读 system_config
    pub async fn load_gas_config(&self) -> ApiResult<(u64, f64)> {
        let rows = sqlx::query_as::<_, (String, String)>(
            "SELECT key, value FROM system_config WHERE key IN ('gas_per_claim', 'gas_estimate_multiplier')",
        )
        .fetch_all(&self.db)
        .await
        .unwrap_or_default();

        let mut gas_per_claim: u64 = 100_000;
        let mut multiplier: f64 = 1.2;

        for (key, value) in &rows {
            match key.as_str() {
                "gas_per_claim" => gas_per_claim = value.parse().unwrap_or(100_000),
                "gas_estimate_multiplier" => multiplier = value.parse().unwrap_or(1.2),
                _ => {}
            }
        }

        Ok((gas_per_claim, multiplier))
    }
}

pub struct ChainContractRow {
    pub chain: String,
    pub contract_address: String,
    pub chain_id: i64,
}
