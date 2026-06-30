use axum::{extract::State, Json};
use serde_json::json;
use uuid::Uuid;

use redpacket_backend::error::{ApiError, ApiResult};
use crate::AppState;

/// GET /api/v1/admin/projects
pub async fn list(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    let rows = state.project_repo.list_all().await?;

    Ok(Json(json!({
        "projects": rows.iter().map(|r| json!({
            "id": r.id, "name": r.name, "app_key": r.app_key, "created_at": r.created_at
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

    state.project_repo.admin_create(pid, name, &key, &secret).await?;

    Ok(Json(json!({"id": pid, "name": name, "app_key": key, "app_secret": secret})))
}

/// PUT /api/v1/admin/projects/{id}
pub async fn update(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    if let Some(name) = body.get("name").and_then(|v| v.as_str()) {
        state.project_repo.update_name(id, name).await?;
    }
    Ok(Json(json!({"status": "updated"})))
}
