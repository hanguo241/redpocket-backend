use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiResult;

#[derive(Clone)]
pub struct ClaimRepo {
    db: PgPool,
}

impl ClaimRepo {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// 创建 pending claim
    pub async fn create_pending(
        &self,
        packet_id: Uuid,
        recipient: &str,
        amount: &str,
        nonce: &str,
        signature: &str,
        claim_type: &str,
    ) -> ApiResult<()> {
        sqlx::query(
            r#"
            INSERT INTO claims (packet_id,recipient_address,amount,fee,nonce,signature,claim_type,status)
            VALUES($1,$2,$3,'0',$4,$5,$6,'pending')
            ON CONFLICT (packet_id, recipient_address) DO UPDATE SET status='pending'
            "#,
        )
        .bind(packet_id)
        .bind(recipient)
        .bind(amount)
        .bind(nonce)
        .bind(signature)
        .bind(claim_type)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    /// 确认 claim
    pub async fn confirm(&self, packet_id: Uuid, recipient: &str, tx_hash: &str) -> ApiResult<()> {
        sqlx::query(
            r#"
            WITH updated AS (
                UPDATE claims
                SET status='confirmed', tx_hash=$1
                WHERE packet_id=$2 AND recipient_address=$3 AND status <> 'confirmed'
                RETURNING packet_id, amount
            )
            UPDATE packets p
            SET remaining_amount = GREATEST(CAST(p.remaining_amount AS numeric) - CAST(updated.amount AS numeric), 0)::text,
                status = CASE
                    WHEN GREATEST(CAST(p.remaining_amount AS numeric) - CAST(updated.amount AS numeric), 0) = 0 THEN 'completed'
                    ELSE p.status
                END,
                updated_at = NOW()
            FROM updated
            WHERE p.id = updated.packet_id
            "#,
        )
        .bind(tx_hash)
        .bind(packet_id)
        .bind(recipient)
        .execute(&self.db)
        .await?;
        Ok(())
    }

    // ==================== Admin: 列表 ====================

    /// Admin 条件查询领取记录
    pub async fn admin_list(
        &self,
        packet_id: Option<Uuid>,
        status: Option<&str>,
    ) -> ApiResult<Vec<AdminClaimRow>> {
        use sqlx::QueryBuilder;

        let mut builder = QueryBuilder::new(
            "SELECT c.id, c.packet_id, c.recipient_address, c.amount, c.status, c.tx_hash, c.created_at, p.chain
             FROM claims c JOIN packets p ON p.id=c.packet_id WHERE 1=1"
        );

        if let Some(pid) = packet_id {
            builder.push(" AND c.packet_id = ");
            builder.push_bind(pid);
        }
        if let Some(status) = status {
            builder.push(" AND c.status = ");
            builder.push_bind(status);
        }
        builder.push(" ORDER BY c.created_at DESC LIMIT 100");

        let rows = builder
            .build_query_as::<(
                Uuid, Uuid, String, String, String, Option<String>,
                DateTime<Utc>, String,
            )>()
            .fetch_all(&self.db)
            .await?;

        Ok(rows
            .into_iter()
            .map(|r| AdminClaimRow {
                id: r.0,
                packet_id: r.1,
                recipient_address: r.2,
                amount: r.3,
                status: r.4,
                tx_hash: r.5,
                created_at: r.6,
                chain: r.7,
            })
            .collect())
    }

    /// 代领确记录
    pub async fn confirm_proxy(
        &self,
        packet_id: Uuid,
        recipient: &str,
        amount: &str,
        nonce: &str,
        signature: &str,
        tx_hash: &str,
    ) -> ApiResult<()> {
        sqlx::query(
            r#"
            WITH updated AS (
                INSERT INTO claims (packet_id,recipient_address,amount,fee,nonce,signature,claim_type,status,tx_hash)
                VALUES($1,$2,$3,'0',$4,$5,'proxy','confirmed',$6)
                ON CONFLICT (packet_id, recipient_address) DO UPDATE
                SET status='confirmed', tx_hash=$6, amount=$3, nonce=$4, signature=$5, claim_type='proxy'
                WHERE claims.status <> 'confirmed'
                RETURNING packet_id, amount
            )
            UPDATE packets p
            SET remaining_amount = GREATEST(CAST(p.remaining_amount AS numeric) - CAST(updated.amount AS numeric), 0)::text,
                status = CASE
                    WHEN GREATEST(CAST(p.remaining_amount AS numeric) - CAST(updated.amount AS numeric), 0) = 0 THEN 'completed'
                    ELSE p.status
                END,
                updated_at = NOW()
            FROM updated
            WHERE p.id = updated.packet_id
            "#,
        )
        .bind(packet_id)
        .bind(recipient)
        .bind(amount)
        .bind(nonce)
        .bind(signature)
        .bind(tx_hash)
        .execute(&self.db)
        .await?;
        Ok(())
    }
}

/// Admin 领取记录列表行
pub struct AdminClaimRow {
    pub id: Uuid,
    pub packet_id: Uuid,
    pub recipient_address: String,
    pub amount: String,
    pub status: String,
    pub tx_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub chain: String,
}
