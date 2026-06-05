use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiResult;

/// 包查询结果：用于 get_status
pub struct PacketStatusRow {
    pub status: String,
    pub total_amount: String,
    pub claimed_amount: String,
    pub claimed_count: i32,
    pub head_count: i32,
    pub claim_mode: String,
    pub remaining_amount: String,
    pub gas_reserve_wei: String,
    pub gas_used_wei: String,
}

#[derive(Clone)]
pub struct PacketRepo {
    db: PgPool,
}

impl PacketRepo {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// 创建红包记录
    pub async fn create(
        &self,
        id: Uuid,
        chain: &str,
        contract_addr: &str,
        creator: &str,
        token: &str,
        total_amount: &str,
        head_count: i32,
        packet_type: &str,
        sub_type: &str,
        claim_mode: &str,
        password_hash: &Option<String>,
        start_time: i64,
        end_time: i64,
        signer_address: &str,
        fee_bps: i32,
        fee_collector: &str,
        gas_reserve_wei: &str,
        gas_estimate_multiplier: f64,
    ) -> ApiResult<()> {
        sqlx::query(
            r#"
            INSERT INTO packets (
                id, chain, contract_address, creator_address, token_address,
                total_amount, remaining_amount, head_count,
                packet_type, sub_type, claim_mode, password_hash,
                start_time, end_time, signer_address, fee_bps, fee_collector, status,
                gas_reserve_wei, gas_estimate_multiplier
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,'active',$18,$19)
            "#,
        )
        .bind(id)
        .bind(chain)
        .bind(contract_addr)
        .bind(creator)
        .bind(token)
        .bind(total_amount)
        .bind(total_amount)
        .bind(head_count)
        .bind(packet_type)
        .bind(sub_type)
        .bind(claim_mode)
        .bind(password_hash)
        .bind(start_time)
        .bind(end_time)
        .bind(signer_address)
        .bind(fee_bps)
        .bind(fee_collector)
        .bind(gas_reserve_wei)
        .bind(gas_estimate_multiplier)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    /// 查询红包状态
    pub async fn get_status(&self, id: Uuid) -> ApiResult<Option<PacketStatusRow>> {
        let row = sqlx::query_as::<_, (
            String, String, String, i32, i32, String, String, String, String,
        )>(
            r#"
            SELECT status, total_amount,
                   (SELECT COALESCE(SUM(CAST(amount AS numeric)),0)::text FROM claims WHERE packet_id=$1) as claimed_amount,
                   (SELECT COUNT(*) FROM claims WHERE packet_id=$1)::int as claimed_count,
                   head_count, claim_mode, remaining_amount::text,
                   gas_reserve_wei, gas_used_wei
            FROM packets WHERE id=$1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await?;

        Ok(row.map(|r| PacketStatusRow {
            status: r.0,
            total_amount: r.1,
            claimed_amount: r.2,
            claimed_count: r.3,
            head_count: r.4,
            claim_mode: r.5,
            remaining_amount: r.6,
            gas_reserve_wei: r.7,
            gas_used_wei: r.8,
        }))
    }

    /// 获取 onchain_packet_id
    pub async fn find_onchain_id(&self, id: Uuid) -> ApiResult<Option<i64>> {
        let val: Option<i64> = sqlx::query_scalar(
            "SELECT onchain_packet_id FROM packets WHERE id=$1",
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await?
        .unwrap_or(None);
        Ok(val)
    }

    /// 更新 onchain_packet_id
    pub async fn update_onchain_id(&self, id: Uuid, onchain_id: i64) -> ApiResult<()> {
        sqlx::query("UPDATE packets SET onchain_packet_id=$1 WHERE id=$2")
            .bind(onchain_id)
            .bind(id)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    /// 查询红包信息（用于 claim）
    pub async fn find_claim_info(&self, id: Uuid) -> ApiResult<Option<ClaimPacketInfo>> {
        let row = sqlx::query_as::<_, (
            String, String, String, i32, String, i64, i32, i32, Option<String>,
        )>(
            r#"
            SELECT total_amount, remaining_amount, packet_type, head_count,
                   sub_type, end_time,
                   (SELECT COUNT(*) FROM claims WHERE packet_id=packets.id)::int as claimed_count,
                   fee_bps, password_hash
            FROM packets WHERE id=$1 AND status='active'
            "#,
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await?;

        Ok(row.map(|r| ClaimPacketInfo {
            total_amount: r.0,
            remaining_amount: r.1,
            packet_type: r.2,
            head_count: r.3,
            sub_type: r.4,
            end_time: r.5,
            claimed_count: r.6,
            fee_bps: r.7,
            password_hash: r.8,
        }))
    }
}

pub struct ClaimPacketInfo {
    pub total_amount: String,
    pub remaining_amount: String,
    pub packet_type: String,
    pub head_count: i32,
    pub sub_type: String,
    pub end_time: i64,
    pub claimed_count: i32,
    pub fee_bps: i32,
    pub password_hash: Option<String>,
}
