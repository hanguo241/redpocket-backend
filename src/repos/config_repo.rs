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

    // ==================== Admin: ChainConfigs ====================

    /// 获取所有活跃链配置（对外 API 用）
    pub async fn list_active_chains(&self) -> ApiResult<Vec<ChainConfigRow>> {
        let rows = sqlx::query_as::<_, (String, i64, String, String, Option<String>, bool)>(
            "SELECT chain, chain_id, rpc_url, contract_address, explorer_url, is_active
             FROM chain_configs WHERE is_active = true ORDER BY chain_id",
        )
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ChainConfigRow {
                chain: r.0,
                chain_id: r.1,
                rpc_url: r.2,
                contract_address: r.3,
                explorer_url: r.4,
                is_active: r.5,
            })
            .collect())
    }

    /// 获取所有链配置（admin 管理用）
    pub async fn list_chain_configs(&self) -> ApiResult<Vec<ChainConfigRow>> {
        let rows = sqlx::query_as::<_, (String, i64, String, String, Option<String>, bool)>(
            "SELECT chain, chain_id, rpc_url, contract_address, explorer_url, is_active FROM chain_configs ORDER BY chain_id",
        )
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ChainConfigRow {
                chain: r.0,
                chain_id: r.1,
                rpc_url: r.2,
                contract_address: r.3,
                explorer_url: r.4,
                is_active: r.5,
            })
            .collect())
    }

    /// 更新链配置（admin 管理用）
    pub async fn update_chain_config(
        &self,
        chain: &str,
        rpc_url: &str,
        contract_address: &str,
        is_active: bool,
    ) -> ApiResult<()> {
        sqlx::query(
            "UPDATE chain_configs SET rpc_url=$1, contract_address=$2, is_active=$3 WHERE chain=$4",
        )
        .bind(rpc_url)
        .bind(contract_address)
        .bind(is_active)
        .bind(chain)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    // ==================== Admin: SystemConfig ====================

    /// 获取所有 system_config 键值对
    pub async fn get_all_config(&self) -> ApiResult<Vec<(String, String)>> {
        let rows = sqlx::query_as::<_, (String, String)>(
            "SELECT key, value FROM system_config ORDER BY key",
        )
        .fetch_all(&self.db)
        .await?;
        Ok(rows)
    }

    /// 写入/覆盖 system_config 键值对
    pub async fn upsert_config(&self, key: &str, value: &str) -> ApiResult<()> {
        sqlx::query(
            r#"INSERT INTO system_config (key, value, updated_at)
               VALUES ($1, $2, NOW())
               ON CONFLICT (key) DO UPDATE SET value = $2, updated_at = NOW()"#,
        )
        .bind(key)
        .bind(value)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    // ==================== Token Configs ====================

    /// 写入一条代币配置
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_token(
        &self, chain: &str, addr: &str, symbol: &str, name: &str,
        decimals: i32, is_native: bool, token_type: &str, sort_order: i32,
        logo_url: Option<&str>,
    ) -> ApiResult<()> {
        sqlx::query(
            r#"INSERT INTO token_configs (chain, token_address, symbol, name, decimals, is_native, token_type, sort_order, logo_url)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
               ON CONFLICT (chain, token_address) DO NOTHING"#
        )
        .bind(chain)
        .bind(addr)
        .bind(symbol)
        .bind(name)
        .bind(decimals)
        .bind(is_native)
        .bind(token_type)
        .bind(sort_order)
        .bind(logo_url)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    /// 为指定链播种默认代币（空链时自动调用）
    pub async fn seed_default_tokens(&self, chain: &str) -> ApiResult<()> {
        let defaults: Vec<(&str, &str, &str, i32, bool, &str, i32)> = match chain {
            "LOCAL" => vec![
                ("native", "ETH", "Local ETH", 18, true, "native", 0),
                ("0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0", "RPT", "RedPacket Test Token", 18, false, "erc20", 1),
            ],
            "ETH" => vec![
                ("native", "ETH", "Ether", 18, true, "native", 0),
                ("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", "USDC", "USD Coin", 6, false, "erc20", 1),
                ("0xdAC17F958D2ee523a2206206994597C13D831ec7", "USDT", "Tether USD", 6, false, "erc20", 2),
                ("0x6B175474E89094C44Da98b954EedeAC495271d0F", "DAI", "Dai Stablecoin", 18, false, "erc20", 3),
                ("0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599", "WBTC", "Wrapped Bitcoin", 8, false, "erc20", 4),
            ],
            "BSC" => vec![
                ("native", "BNB", "Binance Coin", 18, true, "native", 0),
                ("0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d", "USDC", "USD Coin", 18, false, "erc20", 1),
                ("0x55d398326f99059fF775485246999027B3197955", "USDT", "Tether USD", 18, false, "erc20", 2),
            ],
            "AB-Core" => vec![
                ("native", "ABC", "AB-Core Coin", 18, true, "native", 0),
            ],
            "AB-iOT" => vec![
                ("native", "ABT", "AB-iOT Token", 18, true, "native", 0),
            ],
            "AVAX-FUJI" => vec![
                ("native", "AVAX", "Avalanche", 18, true, "native", 0),
            ],
            "SOLANA" => vec![
                ("native", "SOL", "Solana", 9, true, "native", 0),
                ("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", "USDC", "USD Coin", 6, false, "spl-token", 1),
                ("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB", "USDT", "Tether USD", 6, false, "spl-token", 2),
            ],
            _ => vec![
                ("native", "ETH", "Ether", 18, true, "native", 0),
            ],
        };

        for (addr, symbol, name, decimals, is_native, token_type, sort_order) in defaults {
            self.insert_token(chain, addr, symbol, name, decimals, is_native, token_type, sort_order, None).await?;
        }
        Ok(())
    }

    /// 获取指定链的代币列表（公开 API，仅返回 is_active=true）
    /// 如果该链尚未配置任何代币，自动播种默认代币
    pub async fn list_tokens(&self, chain: &str) -> ApiResult<Vec<TokenConfigRow>> {
        let rows = sqlx::query_as::<_, (String, String, String, i32, bool, String, Option<String>, i32, bool)>(
            "SELECT token_address, symbol, name, decimals, is_native, token_type, logo_url, sort_order, is_active
             FROM token_configs
             WHERE chain = $1 AND is_active = true
             ORDER BY sort_order ASC, symbol ASC",
        )
        .bind(chain)
        .fetch_all(&self.db)
        .await?;

        if rows.is_empty() {
            // 可能是新链，自动播种默认代币后重新查询
            self.seed_default_tokens(chain).await?;
            let rows2 = sqlx::query_as::<_, (String, String, String, i32, bool, String, Option<String>, i32, bool)>(
                "SELECT token_address, symbol, name, decimals, is_native, token_type, logo_url, sort_order, is_active
                 FROM token_configs
                 WHERE chain = $1 AND is_active = true
                 ORDER BY sort_order ASC, symbol ASC",
            )
            .bind(chain)
            .fetch_all(&self.db)
            .await?;
            return Ok(rows2
                .into_iter()
                .map(|r| TokenConfigRow {
                    token_address: r.0,
                    symbol: r.1,
                    name: r.2,
                    decimals: r.3,
                    is_native: r.4,
                    token_type: r.5,
                    logo_url: r.6,
                    sort_order: r.7,
                    is_active: r.8,
                })
                .collect());
        }

        Ok(rows
            .into_iter()
            .map(|r| TokenConfigRow {
                token_address: r.0,
                symbol: r.1,
                name: r.2,
                decimals: r.3,
                is_native: r.4,
                token_type: r.5,
                logo_url: r.6,
                sort_order: r.7,
                is_active: r.8,
            })
            .collect())
    }

    // ==================== Admin Token CRUD ====================

    /// 获取所有代币配置（admin），可选按链过滤
    pub async fn list_all_tokens(&self, chain: Option<&str>) -> ApiResult<Vec<TokenConfigAdminRow>> {
        if let Some(chain) = chain {
            let rows = sqlx::query_as::<_, (i32, String, String, String, String, i32, bool, String, Option<String>, i32, bool)>(
                r#"SELECT id, chain, token_address, symbol, name, decimals, is_native, token_type, logo_url, sort_order, is_active
                   FROM token_configs WHERE chain = $1
                   ORDER BY chain, sort_order ASC, symbol ASC"#,
            )
            .bind(chain)
            .fetch_all(&self.db)
            .await?;

            return Ok(rows
                .into_iter()
                .map(|r| TokenConfigAdminRow {
                    id: r.0, chain: r.1, token_address: r.2, symbol: r.3,
                    name: r.4, decimals: r.5, is_native: r.6, token_type: r.7,
                    logo_url: r.8, sort_order: r.9, is_active: r.10,
                })
                .collect());
        }

        let rows = sqlx::query_as::<_, (i32, String, String, String, String, i32, bool, String, Option<String>, i32, bool)>(
            r#"SELECT id, chain, token_address, symbol, name, decimals, is_native, token_type, logo_url, sort_order, is_active
               FROM token_configs
               ORDER BY chain, sort_order ASC, symbol ASC"#,
        )
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| TokenConfigAdminRow {
                id: r.0, chain: r.1, token_address: r.2, symbol: r.3,
                name: r.4, decimals: r.5, is_native: r.6, token_type: r.7,
                logo_url: r.8, sort_order: r.9, is_active: r.10,
            })
            .collect())
    }

    /// 按 ID 获取单条代币配置
    pub async fn get_token_by_id(&self, id: i32) -> ApiResult<Option<TokenConfigAdminRow>> {
        let row = sqlx::query_as::<_, (i32, String, String, String, String, i32, bool, String, Option<String>, i32, bool)>(
            r#"SELECT id, chain, token_address, symbol, name, decimals, is_native, token_type, logo_url, sort_order, is_active
               FROM token_configs WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await?;

        Ok(row.map(|r| TokenConfigAdminRow {
            id: r.0, chain: r.1, token_address: r.2, symbol: r.3,
            name: r.4, decimals: r.5, is_native: r.6, token_type: r.7,
            logo_url: r.8, sort_order: r.9, is_active: r.10,
        }))
    }

    /// 更新代币配置
    pub async fn update_token(
        &self, id: i32, symbol: &str, name: &str, decimals: i32,
        token_type: &str, logo_url: Option<&str>, sort_order: i32, is_active: bool,
    ) -> ApiResult<()> {
        sqlx::query(
            r#"UPDATE token_configs
               SET symbol=$1, name=$2, decimals=$3, token_type=$4, logo_url=$5, sort_order=$6, is_active=$7
               WHERE id=$8"#,
        )
        .bind(symbol)
        .bind(name)
        .bind(decimals)
        .bind(token_type)
        .bind(logo_url)
        .bind(sort_order)
        .bind(is_active)
        .bind(id)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    /// 删除代币配置
    pub async fn delete_token(&self, id: i32) -> ApiResult<()> {
        sqlx::query("DELETE FROM token_configs WHERE id=$1")
            .bind(id)
            .execute(&self.db)
            .await?;
        Ok(())
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

/// 代币配置
/// 代币配置（admin 完整版，含 id）
pub struct TokenConfigAdminRow {
    pub id: i32,
    pub chain: String,
    pub token_address: String,
    pub symbol: String,
    pub name: String,
    pub decimals: i32,
    pub is_native: bool,
    pub token_type: String,
    pub logo_url: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
}

/// 代币配置（公开 API 用，不含敏感字段）
pub struct TokenConfigRow {
    pub token_address: String,
    pub symbol: String,
    pub name: String,
    pub decimals: i32,
    pub is_native: bool,
    pub token_type: String,
    pub logo_url: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
}

/// 完整链配置（admin 展示用）
pub struct ChainConfigRow {
    pub chain: String,
    pub chain_id: i64,
    pub rpc_url: String,
    pub contract_address: String,
    pub explorer_url: Option<String>,
    pub is_active: bool,
}
