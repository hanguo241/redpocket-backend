mod admin;
mod app;
mod handlers;
mod middleware;

use std::sync::Arc;

use axum::{
    middleware as axum_middleware,
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use redpacket_backend::config::Config;
use redpacket_backend::db;
use crate::app::AppState;

#[tokio::main]
async fn main() {
    // 加载 .env
    dotenvy::from_filename("backend/.env").ok();
    dotenvy::dotenv().ok();

    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // 加载配置
    let config = Arc::new(Config::from_env());

    // 连接数据库
    let pool = db::create_pool(&config.database_url)
        .await
        .expect("Failed to connect to database");

    // 运行迁移
    db::run_migrations(&pool)
        .await
        .expect("Failed to run database migrations");

    // 初始化应用状态
    let state = AppState::new(config.clone(), pool).await;

    // 构建路由
    let app = Router::new()
        .route("/health", get(health_check))
        .nest("/api/v1", api_routes(state.clone()))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = config.server_addr();
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind address");

    axum::serve(listener, app)
        .await
        .expect("Server failed");
}

/// API v1 路由组
fn api_routes(state: AppState) -> Router<AppState> {
    let public_packet_routes = Router::new()
        // 官网自用演示路径：用户在官方 website 发红包，不需要商户 AppSecret
        .route("/packet/prepare", post(handlers::packet::prepare))
        .route("/packet/create", post(handlers::packet::create))
        .route("/packet/{id}/status", get(handlers::packet::get_status));

    let merchant_routes = Router::new()
        .route("/packet/prepare", post(handlers::packet::prepare))
        .route("/packet/create", post(handlers::packet::create))
        .route("/packet/{id}/status", get(handlers::packet::get_status))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::auth::require_merchant,
        ));

    let admin_public_routes = Router::new()
        .route("/login", post(admin::auth::login));

    let admin_protected_routes = Router::new()
        .route("/stats", get(admin::dashboard::dashboard))
        .route("/dashboard", get(admin::dashboard::dashboard))
        .route("/projects", get(admin::projects::list).post(admin::projects::create))
        .route("/projects/{id}", axum::routing::put(admin::projects::update))
        .route("/packets", get(admin::packets::list))
        .route("/packets/{id}", get(admin::packets::get))
        .route("/claims", get(admin::claims::list))
        .route("/chains", get(admin::chains::list))
        .route("/chains/{chain}", axum::routing::put(admin::chains::update))
        .route("/settings", get(admin::settings::get))
        .route("/gas-config", get(admin::gas_config::get).put(admin::gas_config::update))
        .route("/fees/withdraw-transaction", post(admin::fees::prepare_withdraw_transaction))
        .route_layer(axum_middleware::from_fn_with_state(
            state,
            middleware::auth::require_admin,
        ));

    Router::new()
        .route("/project/register", post(handlers::project::register))
        .route("/claim/prepare", post(handlers::claim::prepare))
        .route("/claim/confirm", post(handlers::claim::confirm))
        .route("/claim/proxy", post(handlers::claim::proxy_claim))
        .route("/config/chains", get(handlers::config::get_chains))
        .route("/config/tokens", get(handlers::config::get_tokens))
        .route("/config/gas", get(handlers::config::get_gas_config))
        .merge(public_packet_routes)
        .nest("/merchant", merchant_routes)
        .nest("/admin", admin_public_routes.merge(admin_protected_routes))
}

/// Health check
async fn health_check() -> &'static str {
    "OK"
}
