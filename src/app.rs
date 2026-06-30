use std::sync::Arc;

use sqlx::PgPool;

use redpacket_backend::config::Config;
use redpacket_backend::repos::claim_repo::ClaimRepo;
use redpacket_backend::repos::config_repo::ConfigRepo;
use redpacket_backend::repos::packet_repo::PacketRepo;
use redpacket_backend::repos::admin_repo::AdminRepo;
use redpacket_backend::repos::project_repo::ProjectRepo;
use redpacket_backend::services::claim_service::ClaimService;
use redpacket_backend::services::packet_service::PacketService;
use redpacket_backend::services::relayer::RelayerService;
use redpacket_backend::services::project_service::ProjectService;
use redpacket_backend::services::signer::SignerService;

/// 共享应用状态 — 集中管理所有服务、数据访问层和共享资源
///
/// # 设计原则
/// - 服务之间通过 AppState 解耦，不直接互相引用构造
/// - 所有 handler 仅通过 AppState 中的 service 访问业务逻辑
/// - `http_client` 是全局共享的 reqwest::Client（复用连接池）
/// - `config_repo` / `project_repo` 因 middleware 直接调用而暴露，长期目标是将 middleware 也改为 service 层调用
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    /// 全局共享 HTTP 客户端（复用连接池、DNS 缓存）
    /// 服务通过构造器注入获得各自引用，此处保留以便持有连接池生命周期
    #[allow(dead_code)]
    pub http_client: reqwest::Client,
    pub signer: Arc<SignerService>,
    pub relayer: Arc<RelayerService>,
    pub packet_service: Arc<PacketService>,
    pub claim_service: Arc<ClaimService>,
    pub project_service: Arc<ProjectService>,
    pub packet_repo: PacketRepo,
    pub claim_repo: ClaimRepo,
    pub config_repo: ConfigRepo,
    pub admin_repo: AdminRepo,
    pub project_repo: ProjectRepo,
}

impl AppState {
    /// 初始化完整应用状态
    ///
    /// 按依赖顺序依次初始化：Config → 密码学服务 → Repos → Services
    pub async fn new(config: Arc<Config>, db: PgPool) -> Self {
        let http_client = reqwest::Client::builder()
            .pool_max_idle_per_host(10)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        let signer = Arc::new(
            SignerService::new(&config.signer_private_key)
                .expect("Failed to initialize signer service"),
        );

        let relayer = Arc::new(
            RelayerService::new(&config.relayer_private_key, None, http_client.clone())
                .expect("Failed to initialize relayer service"),
        );

        let packet_repo = PacketRepo::new(db.clone());
        let claim_repo = ClaimRepo::new(db.clone());
        let config_repo = ConfigRepo::new(db.clone());
        let project_repo = ProjectRepo::new(db.clone());
        let admin_repo = AdminRepo::new(db.clone());

        let packet_service = Arc::new(PacketService::new(
            packet_repo.clone(),
            config_repo.clone(),
            signer.clone(),
            config.clone(),
            http_client.clone(),
        ));

        let claim_service = Arc::new(ClaimService::new(
            claim_repo.clone(),
            packet_repo.clone(),
            config_repo.clone(),
            signer.clone(),
            relayer.clone(),
        ));

        let project_service = Arc::new(ProjectService::new(project_repo.clone()));

        Self {
            config,
            http_client,
            signer,
            relayer,
            packet_service,
            claim_service,
            project_service,
            packet_repo,
            claim_repo,
            config_repo,
            admin_repo,
            project_repo,
        }
    }
}
