use std::sync::Arc;

use sqlx::PgPool;

use crate::config::Config;
use crate::repos::claim_repo::ClaimRepo;
use crate::repos::config_repo::ConfigRepo;
use crate::repos::packet_repo::PacketRepo;
use crate::repos::project_repo::ProjectRepo;
use crate::services::claim_service::ClaimService;
use crate::services::packet_service::PacketService;
use crate::services::relayer::RelayerService;
use crate::services::signer::SignerService;

/// 共享应用状态 — 包含所有服务和数据访问层
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: PgPool,
    pub signer: Arc<SignerService>,
    pub relayer: Arc<RelayerService>,
    pub packet_service: Arc<PacketService>,
    pub claim_service: Arc<ClaimService>,
    pub packet_repo: PacketRepo,
    pub claim_repo: ClaimRepo,
    pub config_repo: ConfigRepo,
    pub project_repo: ProjectRepo,
}

impl AppState {
    pub async fn new(config: Arc<Config>, db: PgPool) -> Self {
        let signer = Arc::new(
            SignerService::new(&config.signer_private_key)
                .expect("Failed to initialize signer service"),
        );

        let relayer = Arc::new(
            RelayerService::new(&config.relayer_private_key, None)
                .expect("Failed to initialize relayer service"),
        );

        let packet_repo = PacketRepo::new(db.clone());
        let claim_repo = ClaimRepo::new(db.clone());
        let config_repo = ConfigRepo::new(db.clone());
        let project_repo = ProjectRepo::new(db.clone());

        let packet_service = Arc::new(PacketService::new(
            packet_repo.clone(),
            config_repo.clone(),
            signer.clone(),
            config.clone(),
        ));

        let claim_service = Arc::new(ClaimService::new(
            claim_repo.clone(),
            packet_repo.clone(),
            config_repo.clone(),
            signer.clone(),
            relayer.clone(),
        ));

        Self {
            config,
            db,
            signer,
            relayer,
            packet_service,
            claim_service,
            packet_repo,
            claim_repo,
            config_repo,
            project_repo,
        }
    }
}
