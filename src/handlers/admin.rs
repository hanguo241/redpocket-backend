use axum::{extract::State, Json};
use serde_json::json;

use crate::{error::ApiResult, AppState};

/// GET /api/v1/admin/stats
///
/// 平台统计概览
pub async fn get_stats(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    let total_packets: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM packets")
        .fetch_one(&state.db)
        .await?;

    let total_claimed: (String,) = sqlx::query_as(
        r#"SELECT COALESCE(SUM(CAST(amount AS numeric)), 0)::text FROM claims WHERE status = 'confirmed'"#,
    )
    .fetch_one(&state.db)
    .await?;

    let total_projects: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM projects")
        .fetch_one(&state.db)
        .await?;
    let total_platform_fees: (String,) = sqlx::query_as(
        "SELECT COALESCE(SUM(CAST(platform_fee_wei AS numeric)), 0)::text FROM packets",
    )
    .fetch_one(&state.db)
    .await?;

    let active_packets: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM packets WHERE status = 'active' AND end_time > EXTRACT(EPOCH FROM NOW())",
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({
        "total_packets": total_packets.0,
        "total_claimed_amount": total_claimed.0,
        "total_projects": total_projects.0,
        "active_packets": active_packets.0,
        "total_platform_fees_wei": total_platform_fees.0,
    })))
}
