mod admin;
mod app;
mod config;
mod db;
mod error;
mod handlers;
mod middleware;
mod models;
mod repos;
mod services;

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::app::AppState;
use crate::config::Config;

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
        .nest("/api/v1", api_routes())
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
fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/project/register", post(handlers::project::register))
        .route("/packet/prepare", post(handlers::packet::prepare))
        .route("/packet/create", post(handlers::packet::create))
        .route("/packet/{id}/status", get(handlers::packet::get_status))
        .route("/claim/prepare", post(handlers::claim::prepare))
        .route("/claim/confirm", post(handlers::claim::confirm))
        .route("/claim/proxy", post(handlers::claim::proxy_claim))
        .route("/config/chains", get(handlers::config::get_chains))
        .route("/config/gas", get(handlers::config::get_gas_config))
        .route("/admin/stats", get(handlers::admin::get_stats))
        .route("/admin/login", post(admin::auth::login))
        .route("/admin/dashboard", get(admin::dashboard::dashboard))
        .route("/admin/projects", get(admin::projects::list).post(admin::projects::create))
        .route("/admin/projects/{id}", axum::routing::put(admin::projects::update))
        .route("/admin/packets", get(admin::packets::list))
        .route("/admin/packets/{id}", get(admin::packets::get))
        .route("/admin/claims", get(admin::claims::list))
        .route("/admin/chains", get(admin::chains::list))
        .route("/admin/chains/{chain}", axum::routing::put(admin::chains::update))
        .route("/admin/settings", get(admin::settings::get))
        .route("/admin/gas-config", get(admin::gas_config::get).put(admin::gas_config::update))
}

/// Health check
async fn health_check() -> &'static str {
    "OK"
}
