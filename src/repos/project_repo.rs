use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiResult;

pub struct ProjectAuthRow {
    pub id: Uuid,
    pub app_secret: String,
}

/// Admin 项目列表行
pub struct ProjectListRow {
    pub id: Uuid,
    pub name: String,
    pub app_key: String,
    pub created_at: DateTime<Utc>,
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

    // ==================== Admin: 列表/创建/更新 ====================

    /// 获取所有项目列表（admin 管理用）
    pub async fn list_all(&self) -> ApiResult<Vec<ProjectListRow>> {
        let rows = sqlx::query_as::<_, (Uuid, String, String, DateTime<Utc>)>(
            "SELECT id, name, app_key, created_at FROM projects ORDER BY created_at DESC",
        )
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ProjectListRow {
                id: r.0,
                name: r.1,
                app_key: r.2,
                created_at: r.3,
            })
            .collect())
    }

    /// Admin 直接创建项目（跳过快钱签名验证）
    pub async fn admin_create(
        &self,
        id: Uuid,
        name: &str,
        app_key: &str,
        app_secret: &str,
    ) -> ApiResult<()> {
        sqlx::query("INSERT INTO projects (id, name, app_key, app_secret) VALUES ($1,$2,$3,$4)")
            .bind(id)
            .bind(name)
            .bind(app_key)
            .bind(app_secret)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    /// 更新项目名称
    pub async fn update_name(&self, id: Uuid, name: &str) -> ApiResult<()> {
        sqlx::query("UPDATE projects SET name=$1 WHERE id=$2")
            .bind(name)
            .bind(id)
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
