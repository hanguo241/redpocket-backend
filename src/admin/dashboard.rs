use axum::{extract::State, Json};
use serde_json::json;

use redpacket_backend::error::ApiResult;
use crate::AppState;

/// GET /api/v1/admin/dashboard
///
/// 也挂载在 GET /api/v1/admin/stats 路径上（已合并）
pub async fn dashboard(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    let stats = state.packet_repo.get_dashboard_stats().await?;

    Ok(Json(json!({
        "total_packets": stats.total_packets,
        "total_claimed_amount": stats.total_claimed_amount,
        "total_projects": stats.total_projects,
        "active_packets": stats.active_packets,
        "total_claims": stats.total_claims,
        "total_platform_fees_wei": stats.total_platform_fees_wei,
    })))
}
