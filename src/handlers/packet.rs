use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    config::Config,
    error::{ApiError, ApiResult},
    models::packet::{
        CreatePacketRequest, CreatePacketResponse, PacketStatusResponse, TransactionData,
    },
    AppState,
};

// ===================== ABI 编码 =====================

/// 地址/hex 左补零到 32 字节
fn pad32(hex: &str) -> String {
    let s = hex.trim_start_matches("0x");
    format!("{:0>64}", s)
}

/// 十进制数字字符串 → 32 字节 hex (ABI uint256)
fn dec_to_u256(val: &str) -> String {
    let trimmed = val.trim_start_matches("0x");
    // 用 u128 解析（基础情况），实际应使用大数库，此处用简单方式
    let num: u128 = trimmed.parse().unwrap_or(0);
    format!("{:0>64}", format!("{:x}", num))
}

/// u64 → 32 字节 hex (ABI uint256)
fn u64_to_u256(val: u64) -> String {
    format!("{:0>64}", format!("{:x}", val))
}

fn encode_native_calldata(
    head_count: u32, packet_type: u8, sub_type: u8,
    signer: &str, start_time: u64, end_time: u64,
    min_ratio_bps: u32, max_ratio_bps: u32,
    fee_bps: u32, fee_collector: &str,
) -> String {
    // createPacketNative(uint256,uint8,uint8,address,uint256,uint256,uint256,uint256,uint256,address)
    let mut d = String::from("0x81c6cc50");
    d.push_str(&u64_to_u256(head_count as u64));
    d.push_str(&pad32(&format!("{:02x}", packet_type)));
    d.push_str(&pad32(&format!("{:02x}", sub_type)));
    d.push_str(&pad32(signer));
    d.push_str(&u64_to_u256(start_time));
    d.push_str(&u64_to_u256(end_time));
    d.push_str(&u64_to_u256(min_ratio_bps as u64));
    d.push_str(&u64_to_u256(max_ratio_bps as u64));
    d.push_str(&u64_to_u256(fee_bps as u64));
    d.push_str(&pad32(fee_collector));
    d
}

fn encode_erc20_calldata(
    token: &str, total_amount: &str,
    head_count: u32, packet_type: u8, sub_type: u8,
    signer: &str, start_time: u64, end_time: u64,
    min_ratio_bps: u32, max_ratio_bps: u32,
    fee_bps: u32, fee_collector: &str,
) -> String {
    // createPacketERC20(address,uint256,uint256,uint8,uint8,address,uint256,uint256,uint256,uint256,uint256,address)
    let mut d = String::from("0xa68b2711");
    d.push_str(&pad32(if token == "native" { "0x0000000000000000000000000000000000000000" } else { token }));
    d.push_str(&dec_to_u256(total_amount));
    d.push_str(&u64_to_u256(head_count as u64));
    d.push_str(&pad32(&format!("{:02x}", packet_type)));
    d.push_str(&pad32(&format!("{:02x}", sub_type)));
    d.push_str(&pad32(signer));
    d.push_str(&u64_to_u256(start_time));
    d.push_str(&u64_to_u256(end_time));
    d.push_str(&u64_to_u256(min_ratio_bps as u64));
    d.push_str(&u64_to_u256(max_ratio_bps as u64));
    d.push_str(&u64_to_u256(fee_bps as u64));
    d.push_str(&pad32(fee_collector));
    d
}

fn share_url_base(cfg: &Config) -> String {
    cfg.share_url_host.clone()
}

// ===================== Step 1: 获取 calldata (无副作用) =====================

/// POST /api/v1/packet/prepare
pub async fn prepare(
    State(state): State<AppState>,
    Json(req): Json<CreatePacketRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    if req.total_amount.is_empty() {
        return Err(ApiError::BadRequest("total_amount is required".into()));
    }
    if req.head_count <= 0 {
        return Err(ApiError::BadRequest("head_count must be > 0".into()));
    }
    if req.end_time <= chrono::Utc::now().timestamp() {
        return Err(ApiError::BadRequest("end_time must be in the future".into()));
    }

    let chain_cfg = sqlx::query_as::<_, (String, i64)>(
        "SELECT contract_address, chain_id FROM chain_configs WHERE chain = $1 AND is_active = true",
    )
    .bind(&req.chain)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::BadRequest(format!("Chain '{}' not configured", req.chain)))?;

    let contract_address = chain_cfg.0;
    if contract_address.is_empty() {
        return Err(ApiError::BadRequest(format!("Contract not deployed for chain '{}'", req.chain)));
    }

    let signer_address = format!("{:?}", state.signer.address());
    let fee_collector = signer_address.clone();

    let data = if req.token == "native" {
        encode_native_calldata(
            req.head_count as u32,
            match req.packet_type.as_str() { "password" => 1, "condition" => 2, _ => 0 },
            match req.sub_type.as_str() { "random" => 1, _ => 0 },
            &signer_address,
            req.start_time.unwrap_or(0) as u64, req.end_time as u64,
            7000, 10000, 100, &fee_collector,
        )
    } else {
        encode_erc20_calldata(
            &req.token, &req.total_amount,
            req.head_count as u32,
            match req.packet_type.as_str() { "password" => 1, "condition" => 2, _ => 0 },
            match req.sub_type.as_str() { "random" => 1, _ => 0 },
            &signer_address,
            req.start_time.unwrap_or(0) as u64, req.end_time as u64,
            7000, 10000, 100, &fee_collector,
        )
    };

    let tx_data = TransactionData {
        to: format!("0x{}", contract_address.trim_start_matches("0x")),
        data,
        value: if req.token == "native" { req.total_amount.clone() } else { "0".to_string() },
    };

    let pid = Uuid::new_v4();
    let host = share_url_base(&state.config);
    let share_url = format!("{}/claim/{}", host, pid);

    Ok(Json(json!({
        "packet_id": pid,
        "transaction": tx_data,
        "share_url": share_url,
        "expire_at": req.end_time,
    })))
}

// ===================== Step 2: 用户广播后提交 txHash (入库) =====================

/// POST /api/v1/packet/create
///
/// 前端已通过 eth_sendTransaction 广播, 将结果提交给后端记录
/// 写入 DB 后尝试从交易回执中解析 onchain_packet_id
pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let packet_id = body.get("packet_id")
        .and_then(|v| v.as_str()).and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| ApiError::BadRequest("packet_id required".into()))?;
    let tx_hash = body.get("tx_hash")
        .and_then(|v| v.as_str()).ok_or_else(|| ApiError::BadRequest("tx_hash required".into()))?;
    let creator = body.get("creator_address")
        .and_then(|v| v.as_str()).ok_or_else(|| ApiError::BadRequest("creator_address required".into()))?;
    let chain = body.get("chain")
        .and_then(|v| v.as_str()).unwrap_or("");
    let contract_addr = body.get("contract_address")
        .and_then(|v| v.as_str()).unwrap_or("");
    let token = body.get("token")
        .and_then(|v| v.as_str()).unwrap_or("native");
    let total_amount = body.get("total_amount")
        .and_then(|v| v.as_str()).unwrap_or("0");
    let head_count = body.get("head_count")
        .and_then(|v| v.as_i64()).unwrap_or(1) as i32;
    let packet_type = body.get("packet_type").and_then(|v| v.as_str()).unwrap_or("normal");
    let sub_type = body.get("sub_type").and_then(|v| v.as_str()).unwrap_or("average");
    let claim_mode = body.get("claim_mode").and_then(|v| v.as_str()).unwrap_or("both");
    let start_time = body.get("start_time").and_then(|v| v.as_i64()).unwrap_or(0);
    let end_time = body.get("end_time").and_then(|v| v.as_i64()).unwrap_or(0);
    let password_hash = body.get("password").and_then(|v| v.as_str())
        .map(|p| hex::encode(ethers::core::utils::keccak256(p.as_bytes())));
    let signer_address = body.get("signer_address").and_then(|v| v.as_str()).unwrap_or("");
    let fee_bps = body.get("fee_bps").and_then(|v| v.as_i64()).unwrap_or(100) as i32;
    let fee_collector = body.get("fee_collector").and_then(|v| v.as_str()).unwrap_or(signer_address);

    sqlx::query(
        r#"
        INSERT INTO packets (
            id, chain, contract_address, creator_address, token_address,
            total_amount, remaining_amount, head_count,
            packet_type, sub_type, claim_mode, password_hash,
            start_time, end_time, signer_address, fee_bps, fee_collector, status
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,'active')
        "#,
    )
    .bind(packet_id)
    .bind(chain)
    .bind(contract_addr)
    .bind(creator)
    .bind(token)
    .bind(total_amount)
    .bind(total_amount)
    .bind(head_count)
    .bind(packet_type)
    .bind(sub_type)
    .bind(claim_mode)
    .bind(&password_hash)
    .bind(start_time)
    .bind(end_time)
    .bind(signer_address)
    .bind(fee_bps)
    .bind(fee_collector)
    .execute(&state.db)
    .await?;

    tracing::info!(
        "Packet recorded: {} tx={} creator={} chain={}",
        packet_id, tx_hash, creator, chain,
    );

    // 异步获取 onchain_packet_id
    let db2 = state.db.clone();
    let ch = chain.to_string();
    let txh = tx_hash.to_string();
    let pid2 = packet_id;
    tokio::spawn(async move {
        let rpc = sqlx::query_scalar::<_, String>(
            "SELECT rpc_url FROM chain_configs WHERE chain=$1 AND is_active=true",
        )
        .bind(&ch)
        .fetch_optional(&db2)
        .await;
        if let Ok(Some(rpc_url)) = rpc {
            if !rpc_url.is_empty() {
                match crate::services::receipt::fetch_onchain_packet_id(&rpc_url, &txh).await {
                    Ok(Some(onchain_id)) => {
                        let _ = sqlx::query("UPDATE packets SET onchain_packet_id=$1 WHERE id=$2")
                            .bind(onchain_id as i64)
                            .bind(pid2)
                            .execute(&db2)
                            .await;
                        tracing::info!("onchain_packet_id={} for packet={}", onchain_id, pid2);
                    }
                    _ => tracing::warn!("Could not fetch onchain_packet_id for packet={}", pid2),
                }
            }
        }
    });

    let host = share_url_base(&state.config);
    let share_url = format!("{}/claim/{}", host, packet_id);

    Ok(Json(json!({
        "packet_id": packet_id,
        "status": "active",
        "tx_hash": tx_hash,
        "share_url": share_url,
    })))
}

// ===================== 查询状态 =====================

/// GET /api/v1/packet/{id}/status
pub async fn get_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let packet = sqlx::query_as::<_, (String, String, String, i32, i32, String, String, i64)>(
        r#"
        SELECT status, total_amount,
               (SELECT COALESCE(SUM(CAST(amount AS numeric)),0)::text FROM claims WHERE packet_id=$1) as claimed_amount,
               (SELECT COUNT(*) FROM claims WHERE packet_id=$1)::int as claimed_count,
               head_count, claim_mode, remaining_amount::text, end_time
        FROM packets WHERE id=$1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Packet not found".into()))?;

    Ok(Json(json!(PacketStatusResponse {
        packet_id: id,
        status: packet.0,
        total_amount: packet.1,
        claimed_amount: packet.2,
        claimed_count: packet.3,
        head_count: packet.4,
        claim_mode: packet.5,
        remaining_amount: packet.6,
    })))
}
