mod admin;
mod config;
mod db;
mod error;
mod handlers;
mod middleware;
mod models;
mod services;

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::PgPool;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::config::Config;
use crate::services::{relayer::RelayerService, signer::SignerService};

/// 共享应用状态
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: PgPool,
    pub signer: Arc<SignerService>,
    pub relayer: Arc<RelayerService>,
}

#[tokio::main]
async fn main() {
    // 加载 .env (兼容从 repo root 和 backend/ 目录启动)
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

    // 初始化服务
    let signer = Arc::new(
        SignerService::new(&config.signer_private_key)
            .expect("Failed to initialize signer service"),
    );

    let relayer = Arc::new(
        RelayerService::new(&config.relayer_private_key, None)
            .expect("Failed to initialize relayer service"),
    );

    let state = AppState {
        config: config.clone(),
        db: pool,
        signer,
        relayer,
    };

    // 构建路由
    let app = Router::new()
        // Health check
        .route("/health", get(health_check))
        // API v1
        .nest("/api/v1", api_routes())
        // Middleware
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
        // 项目方管理
        .route("/project/register", post(handlers::project::register))
        // 红包管理
        .route("/packet/prepare", post(handlers::packet::prepare))
        .route("/packet/create", post(handlers::packet::create))
        .route("/packet/{id}/status", get(handlers::packet::get_status))
        // Claim
        .route("/claim/prepare", post(handlers::claim::prepare))
        .route("/claim/confirm", post(handlers::claim::confirm))
        .route("/claim/proxy", post(handlers::claim::proxy_claim))
        // 配置
        .route("/config/chains", get(handlers::config::get_chains))
        .route("/config/gas", get(handlers::config::get_gas_config))
        // 管理
        .route("/admin/stats", get(handlers::admin::get_stats))
        // 管理后台 API (admin/)
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
        // gas 配置
        .route("/admin/gas-config", get(admin::gas_config::get).put(admin::gas_config::update))
}

/// Health check
async fn health_check() -> &'static str {
    "OK"
}
