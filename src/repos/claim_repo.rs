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
