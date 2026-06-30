use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiResult;

/// Dashboard 统计数据
pub struct DashboardStats {
    pub total_packets: i64,
    pub total_claimed_amount: String,
    pub total_projects: i64,
    pub active_packets: i64,
    pub total_claims: i64,
    pub total_platform_fees_wei: String,
}

/// 包查询结果：用于 get_status
pub struct PacketStatusRow {
    pub status: String,
    pub gross_amount: String,
    pub total_amount: String,
    pub platform_fee_wei: String,
    pub claimed_amount: String,
    pub claimed_count: i32,
    pub head_count: i32,
    pub claim_mode: String,
    pub remaining_amount: String,
    pub gas_reserve_wei: String,
    pub gas_used_wei: String,
    pub refund_available_at: i64,
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
        project_id: Option<Uuid>,
        chain: &str,
        contract_addr: &str,
        creator: &str,
        token: &str,
        gross_amount: &str,
        claim_pool_amount: &str,
        platform_fee_wei: &str,
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
        tx_hash: &str,
        gas_reserve_wei: &str,
        gas_estimate_multiplier: f64,
    ) -> ApiResult<()> {
        sqlx::query(
            r#"
            INSERT INTO packets (
                id, project_id, chain, contract_address, creator_address, token_address,
                gross_amount, total_amount, remaining_amount, platform_fee_wei, head_count,
                packet_type, sub_type, claim_mode, password_hash,
                start_time, end_time, signer_address, fee_bps, fee_collector, status,
                tx_hash, gas_reserve_wei, gas_estimate_multiplier
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,'active',$21,$22,$23)
            "#,
        )
        .bind(id)
        .bind(project_id)
        .bind(chain)
        .bind(contract_addr)
        .bind(creator)
        .bind(token)
        .bind(gross_amount)
        .bind(claim_pool_amount)
        .bind(claim_pool_amount)
        .bind(platform_fee_wei)
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
        .bind(tx_hash)
        .bind(gas_reserve_wei)
        .bind(gas_estimate_multiplier)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    /// 查询红包状态
    pub async fn get_status(&self, id: Uuid) -> ApiResult<Option<PacketStatusRow>> {
        let row = sqlx::query_as::<_, (
            String, String, String, String, String, i32, i32, String, String, String, String, i64,
        )>(
            r#"
            SELECT status,
                   COALESCE(NULLIF(gross_amount, ''), total_amount) as gross_amount,
                   total_amount,
                   platform_fee_wei,
                   (SELECT COALESCE(SUM(CAST(amount AS numeric)),0)::text FROM claims WHERE packet_id=$1 AND status='confirmed') as claimed_amount,
                   (SELECT COUNT(*) FROM claims WHERE packet_id=$1 AND status='confirmed')::int as claimed_count,
                   head_count, claim_mode, remaining_amount::text,
                   gas_reserve_wei, gas_used_wei,
                   end_time as refund_available_at
            FROM packets WHERE id=$1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await?;

        Ok(row.map(|r| PacketStatusRow {
            status: r.0,
            gross_amount: r.1,
            total_amount: r.2,
            platform_fee_wei: r.3,
            claimed_amount: r.4,
            claimed_count: r.5,
            head_count: r.6,
            claim_mode: r.7,
            remaining_amount: r.8,
            gas_reserve_wei: r.9,
            gas_used_wei: r.10,
            refund_available_at: r.11,
        }))
    }

    /// 获取 onchain_packet_id
    pub async fn find_onchain_id(&self, id: Uuid) -> ApiResult<Option<i64>> {
        let val: Option<i64> =
            sqlx::query_scalar("SELECT onchain_packet_id FROM packets WHERE id=$1")
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

    /// 查询所有未同步 onchain_packet_id 的活跃红包（用于后台补充索引）
    pub async fn find_unsynced_packets(&self, limit: i64) -> ApiResult<Vec<UnsyncedPacket>> {
        let rows = sqlx::query_as::<_, (Uuid, String, String)>(
            r#"
            SELECT id, chain, tx_hash
            FROM packets
            WHERE onchain_packet_id IS NULL
              AND tx_hash IS NOT NULL
              AND tx_hash != ''
              AND status = 'active'
            ORDER BY created_at ASC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, chain, tx_hash)| UnsyncedPacket {
                id,
                chain,
                tx_hash,
            })
            .collect())
    }

    // ==================== Admin: 列表/详情 ====================

    // ==================== Dashboard ====================

    /// Dashboard 统计数据（跨多表查询，只读）
    pub async fn get_dashboard_stats(&self) -> ApiResult<DashboardStats> {
        let total_packets: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM packets")
            .fetch_one(&self.db).await?;
        let total_claimed: (String,) = sqlx::query_as(
            "SELECT COALESCE(SUM(CAST(amount AS numeric)),0)::text FROM claims WHERE status='confirmed'",
        ).fetch_one(&self.db).await?;
        let total_platform_fees: (String,) = sqlx::query_as(
            "SELECT COALESCE(SUM(CAST(platform_fee_wei AS numeric)),0)::text FROM packets",
        ).fetch_one(&self.db).await?;
        let total_projects: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM projects")
            .fetch_one(&self.db).await?;
        let active_packets: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM packets WHERE status='active' AND end_time > EXTRACT(EPOCH FROM NOW())",
        ).fetch_one(&self.db).await?;
        let total_claims: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM claims WHERE status='confirmed'")
            .fetch_one(&self.db).await?;

        Ok(DashboardStats {
            total_packets: total_packets.0,
            total_claimed_amount: total_claimed.0,
            total_projects: total_projects.0,
            active_packets: active_packets.0,
            total_claims: total_claims.0,
            total_platform_fees_wei: total_platform_fees.0,
        })
    }

    /// Admin 条件查询红包列表
    pub async fn admin_list(
        &self,
        status: Option<&str>,
        chain: Option<&str>,
    ) -> ApiResult<Vec<AdminPacketRow>> {
        use sqlx::QueryBuilder;

        let mut builder = QueryBuilder::new(
            "SELECT id, chain, creator_address, gross_amount, total_amount, platform_fee_wei, status, created_at, end_time FROM packets WHERE 1=1"
        );

        if let Some(status) = status {
            builder.push(" AND status = ");
            builder.push_bind(status);
        }
        if let Some(chain) = chain {
            builder.push(" AND chain = ");
            builder.push_bind(chain);
        }
        builder.push(" ORDER BY created_at DESC LIMIT 100");

        let rows = builder
            .build_query_as::<(
                Uuid, String, String, String, String, String, String,
                chrono::DateTime<chrono::Utc>, i64,
            )>()
            .fetch_all(&self.db)
            .await?;

        Ok(rows
            .into_iter()
            .map(|r| AdminPacketRow {
                id: r.0,
                chain: r.1,
                creator_address: r.2,
                gross_amount: r.3,
                total_amount: r.4,
                platform_fee_wei: r.5,
                status: r.6,
                created_at: r.7,
                end_time: r.8,
            })
            .collect())
    }

    /// Admin 获取红包详情（含领取记录）
    pub async fn admin_get_detail(&self, id: Uuid) -> ApiResult<Option<PacketDetail>> {
        let pkt = sqlx::query_as::<_, (
            String, String, String, String, String, String, String, String,
            i32, String, i64, Option<i64>,
        )>(
            r#"SELECT chain,contract_address,creator_address,token_address,total_amount,status,
                      gross_amount,platform_fee_wei,head_count,claim_mode,end_time,onchain_packet_id
               FROM packets WHERE id=$1"#,
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await?;

        Ok(pkt.map(|r| PacketDetail {
            chain: r.0,
            contract_address: r.1,
            creator_address: r.2,
            token_address: r.3,
            total_amount: r.4,
            status: r.5,
            gross_amount: r.6,
            platform_fee_wei: r.7,
            head_count: r.8,
            claim_mode: r.9,
            end_time: r.10,
            onchain_packet_id: r.11,
        }))
    }

    /// Admin 获取红包的领取记录
    pub async fn admin_get_claims(&self, packet_id: Uuid) -> ApiResult<Vec<PacketClaimRow>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, chrono::DateTime<chrono::Utc>)>(
            "SELECT recipient_address,amount,status,tx_hash,created_at FROM claims WHERE packet_id=$1 ORDER BY created_at DESC",
        )
        .bind(packet_id)
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| PacketClaimRow {
                recipient_address: r.0,
                amount: r.1,
                status: r.2,
                tx_hash: r.3,
                created_at: r.4,
            })
            .collect())
    }

    /// 查询红包信息（用于 claim）
    pub async fn find_claim_info(&self, id: Uuid) -> ApiResult<Option<ClaimPacketInfo>> {
        let row = sqlx::query_as::<_, (
            String, String, String, i32, String, i64, i32, i32, Option<String>,
        )>(
            r#"
            SELECT total_amount, remaining_amount, packet_type, head_count,
                   sub_type, end_time,
                   (SELECT COUNT(*) FROM claims WHERE packet_id=packets.id AND status='confirmed')::int as claimed_count,
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

/// Admin 列表行
pub struct AdminPacketRow {
    pub id: Uuid,
    pub chain: String,
    pub creator_address: String,
    pub gross_amount: String,
    pub total_amount: String,
    pub platform_fee_wei: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub end_time: i64,
}

/// Admin 详情
pub struct PacketDetail {
    pub chain: String,
    pub contract_address: String,
    pub creator_address: String,
    pub token_address: String,
    pub total_amount: String,
    pub status: String,
    pub gross_amount: String,
    pub platform_fee_wei: String,
    pub head_count: i32,
    pub claim_mode: String,
    pub end_time: i64,
    pub onchain_packet_id: Option<i64>,
}

/// Admin 红包的领取记录行
pub struct PacketClaimRow {
    pub recipient_address: String,
    pub amount: String,
    pub status: String,
    pub tx_hash: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 未同步 onchain_packet_id 的红包记录
pub struct UnsyncedPacket {
    pub id: Uuid,
    pub chain: String,
    pub tx_hash: String,
}
