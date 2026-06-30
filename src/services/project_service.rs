use ethers::core::types::H160;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::models::project::RegisterRequest;
use crate::repos::project_repo::ProjectRepo;

/// 项目方注册/认证相关业务逻辑
pub struct ProjectService {
    pub project_repo: ProjectRepo,
}

impl ProjectService {
    pub fn new(project_repo: ProjectRepo) -> Self {
        Self { project_repo }
    }

    /// 验证钱包签名并创建项目方账户
    ///
    /// 签名消息格式: `"RedPacket Register: {wallet_address}"`
    pub async fn register(&self, req: RegisterRequest) -> ApiResult<RegisterOutput> {
        if req.name.trim().is_empty() {
            return Err(ApiError::BadRequest("项目名称不能为空".into()));
        }
        if req.wallet_address.trim().is_empty() {
            return Err(ApiError::BadRequest("钱包地址不能为空".into()));
        }

        // 验证钱包签名
        let wallet: H160 = req
            .wallet_address
            .parse()
            .map_err(|_| ApiError::BadRequest("无效的钱包地址".into()))?;

        let msg = format!(
            "\x19Ethereum Signed Message:\n{}RedPacket Register: {}",
            (20 + req.wallet_address.len()),
            req.wallet_address
        );

        let msg_hash = ethers::core::utils::keccak256(msg.as_bytes());
        let sig_bytes = hex::decode(req.signature.trim_start_matches("0x"))
            .map_err(|_| ApiError::BadRequest("无效的签名格式".into()))?;

        let recovered = ethers::core::types::Signature::try_from(sig_bytes.as_slice())
            .and_then(|s| s.recover(msg_hash))
            .map_err(|_| ApiError::BadRequest("签名恢复失败".into()))?;

        if recovered != wallet {
            return Err(ApiError::Unauthorized("钱包签名不匹配".into()));
        }

        // 生成凭证
        let project_id = Uuid::new_v4();
        let app_key = Uuid::new_v4().to_string().replace("-", "");
        let app_secret = Uuid::new_v4().to_string().replace("-", "");
        let website = req.website.unwrap_or_default();
        let contact = req.contact.unwrap_or_default();

        // 落库
        self.project_repo
            .create(
                project_id,
                req.name.trim(),
                &app_key,
                &app_secret,
                req.wallet_address.trim(),
                &website,
                &contact,
            )
            .await?;

        tracing::info!(
            "Project registered: {} ({}) wallet={}",
            req.name, project_id, req.wallet_address
        );

        Ok(RegisterOutput {
            project_id,
            app_key,
            app_secret,
            message: "注册成功！请妥善保存 AppSecret，不会再次显示。".to_string(),
        })
    }
}

/// 注册结果
pub struct RegisterOutput {
    pub project_id: Uuid,
    pub app_key: String,
    pub app_secret: String,
    pub message: String,
}
