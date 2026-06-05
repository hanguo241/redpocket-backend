use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiResult;

#[derive(Clone)]
pub struct ProjectRepo {
    db: PgPool,
}

impl ProjectRepo {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// 注册项目方
    pub async fn create(
        &self,
        id: Uuid,
        name: &str,
        app_key: &str,
        app_secret: &str,
        wallet_address: &str,
        website: &str,
        contact: &str,
    ) -> ApiResult<()> {
        sqlx::query(
            r#"
            INSERT INTO projects (id, name, app_key, app_secret, wallet_address, website, contact)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(app_key)
        .bind(app_secret)
        .bind(wallet_address)
        .bind(website)
        .bind(contact)
        .execute(&self.db)
        .await?;
        Ok(())
    }
}
