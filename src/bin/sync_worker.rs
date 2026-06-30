//! 后台 onchain_packet_id 补充索引服务
//!
//! 周期性扫描 packets 表中 onchain_packet_id IS NULL 的记录，
//! 通过 RPC 获取交易回执，解析 PacketCreated 事件中的 onchain_id 并回填。
//!
//! 作为后台异步同步的兜底机制 — 即使主流程中的 tokio::spawn 同步失败，
//! 此服务也会自动补充。
//!
//! 用法:
//!   cargo run --bin redpacket-sync-worker
//!   # 或
//!   ./target/debug/redpacket-sync-worker

use std::time::Duration;

use sqlx::PgPool;

use redpacket_backend::config::Config;
use redpacket_backend::repos::config_repo::ConfigRepo;
use redpacket_backend::repos::packet_repo::{PacketRepo, UnsyncedPacket};
use redpacket_backend::services::receipt::fetch_onchain_packet_id;

/// 轮询间隔
const POLL_INTERVAL_SECS: u64 = 15;
/// 每批处理上限
const BATCH_LIMIT: i64 = 20;

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
    let config = Config::from_env();
    tracing::info!("Connecting to database...");

    let pool = match PgPool::connect(&config.database_url).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Failed to connect to database: {}", e);
            return;
        }
    };

    // 运行迁移（确保 tx_hash 列存在）
    if let Err(e) = sqlx::migrate!("./migrations").run(&pool).await {
        tracing::error!("Migration failed: {}", e);
        return;
    }

    let http_client = reqwest::Client::new();
    let packet_repo = PacketRepo::new(pool.clone());
    let config_repo = ConfigRepo::new(pool.clone());

    tracing_info_worker_started();

    let mut interval = tokio::time::interval(Duration::from_secs(POLL_INTERVAL_SECS));

    loop {
        interval.tick().await;

        let unsynced = match packet_repo.find_unsynced_packets(BATCH_LIMIT).await {
            Ok(list) => list,
            Err(e) => {
                tracing::warn!("Failed to query unsynced packets: {}", e);
                continue;
            }
        };

        if unsynced.is_empty() {
            continue;
        }

        tracing::info!("Found {} packet(s) without onchain_id", unsynced.len());

        for pkt in unsynced {
            if let Err(()) = process_one(&http_client, &pkt, &config_repo, &packet_repo).await {
                // 单个失败不中断批次
                continue;
            }
        }
    }
}

/// 处理单个未同步红包
async fn process_one(
    client: &reqwest::Client,
    pkt: &UnsyncedPacket,
    config_repo: &ConfigRepo,
    packet_repo: &PacketRepo,
) -> Result<(), ()> {
    let rpc_url = match config_repo.find_rpc_url(&pkt.chain).await {
        Ok(Some(url)) if !url.is_empty() => url,
        Ok(_) => {
            tracing::debug!(
                "[sync_worker] No RPC URL for chain={} packet={}; skip",
                pkt.chain,
                pkt.id
            );
            return Err(());
        }
        Err(e) => {
            tracing::warn!(
                "[sync_worker] DB error for chain={} packet={}: {}",
                pkt.chain,
                pkt.id,
                e
            );
            return Err(());
        }
    };

    match fetch_onchain_packet_id(client, &rpc_url, &pkt.tx_hash).await {
        Ok(Some(id)) => {
            if let Err(e) = packet_repo.update_onchain_id(pkt.id, id as i64).await {
                tracing::error!(
                    "[sync_worker] Failed to update onchain_id for packet={}: {}",
                    pkt.id,
                    e
                );
                return Err(());
            }
            tracing::info!(
                "[sync_worker] Backfilled onchain_packet_id={} for packet={} chain={}",
                id,
                pkt.id,
                pkt.chain,
            );
            Ok(())
        }
        Ok(None) => {
            tracing::debug!(
                "[sync_worker] Receipt not ready: packet={} tx={}",
                pkt.id,
                pkt.tx_hash
            );
            Err(())
        }
        Err(e) => {
            tracing::warn!(
                "[sync_worker] RPC error for packet={} tx={}: {}",
                pkt.id,
                pkt.tx_hash,
                e
            );
            Err(())
        }
    }
}

fn tracing_info_worker_started() {
    tracing::info!(
        "sync-worker started (interval={}s, batch={})",
        POLL_INTERVAL_SECS,
        BATCH_LIMIT,
    );
}
