use axum::{extract::State, Json};
use serde_json::json;

use crate::error::{ApiError, ApiResult};
use crate::AppState;

/// GET /api/v1/admin/gas-config
pub async fn get(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT key, value FROM system_config ORDER BY key",
    )
    .fetch_all(&state.db)
    .await?;

    let mut config = serde_json::Map::new();
    for (key, value) in &rows {
        config.insert(key.clone(), json!(value));
    }

    Ok(Json(json!({
        "config": config
    })))
}

/// PUT /api/v1/admin/gas-config
pub async fn update(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let config = body.get("config").and_then(|c| c.as_object())
        .ok_or_else(|| ApiError::BadRequest("config object required".into()))?;

    for (key, value) in config {
        let val_str = if let Some(s) = value.as_str() {
            s.to_string()
        } else if let Some(f) = value.as_f64() {
            f.to_string()
        } else if let Some(i) = value.as_i64() {
            i.to_string()
        } else {
            return Err(ApiError::BadRequest(format!("Invalid value for key '{}'", key)));
        };

        sqlx::query(
            r#"INSERT INTO system_config (key, value, updated_at)
               VALUES ($1, $2, NOW())
               ON CONFLICT (key) DO UPDATE SET value = $2, updated_at = NOW()"#,
        )
        .bind(key)
        .bind(&val_str)
        .execute(&state.db)
        .await?;
    }

    Ok(Json(json!({"status": "updated"})))
}
