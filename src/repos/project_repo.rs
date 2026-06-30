use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiResult;

pub struct ProjectAuthRow {
    pub id: Uuid,
    pub app_secret: String,
}

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

    /// 按 AppKey 查询 API 鉴权信息
    pub async fn find_auth_by_app_key(&self, app_key: &str) -> ApiResult<Option<ProjectAuthRow>> {
        let row = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id, app_secret FROM projects WHERE app_key=$1",
        )
        .bind(app_key)
        .fetch_optional(&self.db)
        .await?;

        Ok(row.map(|(id, app_secret)| ProjectAuthRow { id, app_secret }))
    }
}
