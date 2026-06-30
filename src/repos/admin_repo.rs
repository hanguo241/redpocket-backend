use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiResult;

/// Admin 用户认证数据访问
#[derive(Clone)]
pub struct AdminRepo {
    db: PgPool,
}

impl AdminRepo {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// 按 ID 和角色检查管理员是否活跃（用于 middleware 鉴权）
    pub async fn find_active_by_id(&self, id: Uuid, role: &str) -> ApiResult<bool> {
        let active: Option<bool> = sqlx::query_scalar(
            "SELECT is_active FROM admin_users WHERE id=$1 AND role=$2",
        )
        .bind(id)
        .bind(role)
        .fetch_optional(&self.db)
        .await?;

        Ok(active == Some(true))
    }

    /// 按邮箱查找活跃管理员（用于登录认证）
    pub async fn find_active_by_email(
        &self,
        email: &str,
    ) -> ApiResult<Option<AdminAuthRow>> {
        let row = sqlx::query_as::<_, (Uuid, String, String, String)>(
            "SELECT id, password_hash, name, role FROM admin_users WHERE email=$1 AND is_active=true",
        )
        .bind(email)
        .fetch_optional(&self.db)
        .await?;

        Ok(row.map(|r| AdminAuthRow {
            id: r.0,
            password_hash: r.1,
            name: r.2,
            role: r.3,
        }))
    }
}

/// Admin 用户认证查询结果
pub struct AdminAuthRow {
    pub id: Uuid,
    pub password_hash: String,
    pub name: String,
    pub role: String,
}
