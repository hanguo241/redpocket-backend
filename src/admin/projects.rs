use axum::{extract::State, Json};
use serde_json::json;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::AppState;

/// GET /api/v1/admin/projects
pub async fn list(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query_as::<_, (Uuid, String, String, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, name, app_key, created_at FROM projects ORDER BY created_at DESC",
    )
    .fetch_all(&state.db).await?;

    Ok(Json(json!({
        "projects": rows.iter().map(|(id, name, key, created)| json!({
            "id": id, "name": name, "app_key": key, "created_at": created
        })).collect::<Vec<_>>()
    })))
}

/// POST /api/v1/admin/projects
pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let name = body.get("name").and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::BadRequest("name required".into()))?;
    let pid = Uuid::new_v4();
    let key = Uuid::new_v4().to_string().replace("-", "");
    let secret = Uuid::new_v4().to_string().replace("-", "");

    sqlx::query("INSERT INTO projects (id, name, app_key, app_secret) VALUES ($1,$2,$3,$4)")
        .bind(pid).bind(name).bind(&key).bind(&secret)
        .execute(&state.db).await?;

    Ok(Json(json!({"id": pid, "name": name, "app_key": key, "app_secret": secret})))
}

/// PUT /api/v1/admin/projects/{id}
pub async fn update(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    if let Some(name) = body.get("name").and_then(|v| v.as_str()) {
        sqlx::query("UPDATE projects SET name=$1 WHERE id=$2")
            .bind(name).bind(id).execute(&state.db).await?;
    }
    if let Some(active) = body.get("is_active") {
        // projects 表目前没有 is_active 字段，扩展预留
    }
    Ok(Json(json!({"status": "updated"})))
}
