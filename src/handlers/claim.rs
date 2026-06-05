use axum::{extract::State, Json};
use ethers::core::types::{H160, U256};
use serde_json::json;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    AppState,
};

// ===================== ABI 编码工具 =====================

fn pad32(hex: &str) -> String {
    format!("{:0>64}", hex.trim_start_matches("0x"))
}
fn u64_to_hex256(val: u64) -> String {
    format!("{:0>64}", format!("{:x}", val))
}
fn dec_to_hex256(val: &str) -> String {
    let v: u128 = val.parse().unwrap_or(0);
    format!("{:0>64}", format!("{:x}", v))
}

/// 编码 claim(uint256,address,uint256,uint256,uint256,bytes) 调用数据
fn encode_claim_call(
    onchain_packet_id: &str, recipient: &str, amount: &str,
    nonce: &str, deadline: &str, signature_hex: &str,
) -> String {
    let sig = signature_hex.trim_start_matches("0x");
    let sig_len = sig.len() / 2;
    let padded_len = ((sig.len() + 63) / 64) * 64;

    let mut d = String::from("0xedc19c89"); // claim(...) selector
    d.push_str(&pad32(onchain_packet_id));     // packetId
    d.push_str(&pad32(recipient));             // recipient
    d.push_str(&dec_to_hex256(amount));        // amount
    d.push_str(&dec_to_hex256(nonce));         // nonce
    d.push_str(&dec_to_hex256(deadline));      // deadline
    d.push_str(&u64_to_hex256(192));           // offset to bytes (6*32=192)
    d.push_str(&u64_to_hex256(sig_len as u64));// length
    // bytes 数据左对齐，右补零到 32 字节对齐
    d.push_str(&format!("{:0<width$}", sig, width = padded_len));
    d
}

// ===================== 自领: 准备交易数据 =====================

/// POST /api/v1/claim/prepare
///
/// 验证条件 + 签发签名 + 构造交易数据
/// 前端拿到 to+data 后直接 eth_sendTransaction
pub async fn prepare(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let packet_id = body.get("packet_id")
        .and_then(|v| v.as_str()).and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| ApiError::BadRequest("packet_id required".into()))?;
    let user_addr = body.get("user_address")
        .and_then(|v| v.as_str()).ok_or_else(|| ApiError::BadRequest("user_address required".into()))?;
    let proof_pw = body.get("proof")
        .and_then(|p| p.get("password")).and_then(|v| v.as_str());

    // 1. 查红包
    let pkt = sqlx::query_as::<_, (
        String,String,String,i32,String,i64,Option<String>,
    )>(
        r#"SELECT total_amount,remaining_amount,packet_type,head_count,sub_type,end_time,password_hash
           FROM packets WHERE id=$1 AND status='active'"#,
    )
    .bind(packet_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Packet not found or not active".into()))?;

    if pkt.5 <= chrono::Utc::now().timestamp() {
        return Err(ApiError::BadRequest("Packet has expired".into()));
    }

    // 2. 校验口令
    if let Some(ref hash) = pkt.6 {
        if !hash.is_empty() {
            let input_hash = hex::encode(
                ethers::core::utils::keccak256(proof_pw.unwrap_or("").as_bytes())
            );
            if input_hash != *hash {
                return Err(ApiError::BadRequest("Invalid password".into()));
            }
        }
    }

    // 3. 查链配置
    let chain_row = sqlx::query_as::<_, (String, String)>(
        "SELECT p.chain, cc.contract_address
         FROM packets p JOIN chain_configs cc ON cc.chain=p.chain WHERE p.id=$1",
    )
    .bind(packet_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::BadRequest("Chain not configured".into()))?;

    let contract_addr: H160 = chain_row.1.parse().unwrap_or_default();

    let chain_cfg = sqlx::query_as::<_, (i64,)>(
        "SELECT chain_id FROM chain_configs WHERE contract_address=$1",
    )
    .bind(&chain_row.1)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::BadRequest("Chain config not found".into()))?;
    let chain_id = chain_cfg.0 as u64;

    // 4. 计算领取金额
    fn parse_wei(s: &str) -> u128 { s.parse().unwrap_or(0) }
    let remaining = parse_wei(&pkt.1);
    let head_count = pkt.3 as u128;
    let amount = remaining / head_count;

    let nonce_num = chrono::Utc::now().timestamp_millis() as u64;
    let deadline_num = (chrono::Utc::now().timestamp() + 1800) as u64;
    let recipient: H160 = user_addr.parse().unwrap_or_default();

    // 5. 从 DB 获取真实的 onchain_packet_id
    let onchain_id: Option<i64> = sqlx::query_scalar(
        "SELECT onchain_packet_id FROM packets WHERE id=$1",
    )
    .bind(packet_id)
    .fetch_optional(&state.db)
    .await?
    .unwrap_or(None);
    let chain_pid = onchain_id.filter(|&v| v > 0).map(|v| v as u64).unwrap_or(1);

    // 6. 签发 EIP-712 签名
    let signature = state
        .signer
        .sign_claim(
            U256::from(chain_pid),
            recipient,
            U256::from(amount),
            U256::from(nonce_num),
            U256::from(deadline_num),
            chain_id,
            contract_addr,
        )
        .await?;
    let sig_hex = hex::encode(&signature[..]);

    // 6. 构造交易数据
    let calldata = encode_claim_call(
        &format!("0x{:x}", chain_pid), user_addr, &amount.to_string(),
        &nonce_num.to_string(), &deadline_num.to_string(),
        &format!("0x{}", sig_hex),
    );

    // 7. 记录 pending claim
    sqlx::query(
        r#"INSERT INTO claims (packet_id,recipient_address,amount,fee,nonce,signature,claim_type,status)
           VALUES($1,$2,$3,'0',$4,$5,'self','pending')
           ON CONFLICT (packet_id, recipient_address) DO UPDATE SET status='pending'"#,
    )
    .bind(packet_id)
    .bind(user_addr)
    .bind(amount.to_string())
    .bind(nonce_num.to_string())
    .bind(&sig_hex)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({
        "packet_id": packet_id,
        "amount": amount.to_string(),
        "signature": format!("0x{}", sig_hex),
        "nonce": nonce_num,
        "deadline": deadline_num,
        "transaction": {
            "to": format!("0x{}", contract_addr.to_fixed_bytes().iter().map(|b| format!("{:02x}", b)).collect::<String>()),
            "data": calldata,
            "value": "0"
        }
    })))
}

// ===================== 自领: 确认 =====================

/// POST /api/v1/claim/confirm
pub async fn confirm(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let packet_id = body.get("packet_id")
        .and_then(|v| v.as_str()).and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| ApiError::BadRequest("packet_id required".into()))?;
    let recipient = body.get("recipient")
        .and_then(|v| v.as_str()).ok_or_else(|| ApiError::BadRequest("recipient required".into()))?;
    let tx_hash = body.get("tx_hash")
        .and_then(|v| v.as_str()).ok_or_else(|| ApiError::BadRequest("tx_hash required".into()))?;

    sqlx::query(
        r#"UPDATE claims SET status='confirmed', tx_hash=$1
           WHERE packet_id=$2 AND recipient_address=$3"#,
    )
    .bind(tx_hash)
    .bind(packet_id)
    .bind(recipient)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({"status":"confirmed","tx_hash":tx_hash})))
}

// ===================== 代领: 用户授权 → 后端广播 =====================

/// POST /api/v1/claim/proxy
///
/// 用户提供授权签名，后端验证后构造并广播 claim 交易
pub async fn proxy_claim(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let packet_id = body.get("packet_id")
        .and_then(|v| v.as_str()).and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| ApiError::BadRequest("packet_id required".into()))?;
    let user_addr = body.get("user_address")
        .and_then(|v| v.as_str()).ok_or_else(|| ApiError::BadRequest("user_address required".into()))?;
    let user_sig = body.get("user_signature")
        .and_then(|v| v.as_str()).ok_or_else(|| ApiError::BadRequest("user_signature required".into()))?;
    let proof_pw = body.get("proof")
        .and_then(|p| p.get("password")).and_then(|v| v.as_str());

    // 1. 验证用户授权签名: 用户签名了 "RedPacket: authorize claim {packet_id}"
    let auth_msg = format!("\x19Ethereum Signed Message:\n{}RedPacket: authorize claim {}", 
        (32 + packet_id.to_string().len()), packet_id);
    let auth_hash = ethers::core::utils::keccak256(auth_msg.as_bytes());
    let sig_bytes = hex::decode(user_sig.trim_start_matches("0x"))
        .map_err(|_| ApiError::BadRequest("Invalid signature format".into()))?;

    let recovered = ethers::core::types::Signature::try_from(sig_bytes.as_slice())
        .and_then(|s| s.recover(auth_hash))
        .map_err(|_| ApiError::BadRequest("Signature recovery failed".into()))?;

    if format!("{:?}", recovered).to_lowercase() != user_addr.to_lowercase() {
        return Err(ApiError::Unauthorized("Invalid user signature".into()));
    }

    // 2. 查红包 + 链配置 + 计算金额 (复用 prepare 逻辑)
    let pkt = sqlx::query_as::<_, (
        String,String,String,i32,String,i64,Option<String>,
    )>(
        r#"SELECT total_amount,remaining_amount,packet_type,head_count,sub_type,end_time,password_hash
           FROM packets WHERE id=$1 AND status='active'"#,
    )
    .bind(packet_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Packet not found or not active".into()))?;

    if pkt.5 <= chrono::Utc::now().timestamp() {
        return Err(ApiError::BadRequest("Packet has expired".into()));
    }

    // 口令校验
    if let Some(ref hash) = pkt.6 {
        if !hash.is_empty() {
            let input_hash = hex::encode(
                ethers::core::utils::keccak256(proof_pw.unwrap_or("").as_bytes())
            );
            if input_hash != *hash {
                return Err(ApiError::BadRequest("Invalid password".into()));
            }
        }
    }

    // 查链配置
    let chain_row = sqlx::query_as::<_, (String, String)>(
        "SELECT p.chain, cc.contract_address
         FROM packets p JOIN chain_configs cc ON cc.chain=p.chain WHERE p.id=$1",
    )
    .bind(packet_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::BadRequest("Chain not configured".into()))?;

    let contract_addr: H160 = chain_row.1.parse().unwrap_or_default();

    let chain_cfg = sqlx::query_as::<_, (i64,)>(
        "SELECT chain_id FROM chain_configs WHERE contract_address=$1",
    )
    .bind(&chain_row.1)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::BadRequest("Chain config not found".into()))?;
    let chain_id = chain_cfg.0 as u64;

    fn parse_wei(s: &str) -> u128 { s.parse().unwrap_or(0) }
    let remaining = parse_wei(&pkt.1);
    let head_count = pkt.3 as u128;
    let amount = remaining / head_count;
    let nonce_num = chrono::Utc::now().timestamp_millis() as u64;
    let deadline_num = (chrono::Utc::now().timestamp() + 1800) as u64;
    let recipient: H160 = user_addr.parse().unwrap_or_default();

    let onchain_id_proxy: Option<i64> = sqlx::query_scalar(
        "SELECT onchain_packet_id FROM packets WHERE id=$1",
    )
    .bind(packet_id)
    .fetch_optional(&state.db)
    .await?
    .unwrap_or(None);
    let chain_pid = onchain_id_proxy.filter(|&v| v > 0).map(|v| v as u64).unwrap_or(1);

    // 3. 签发平台签名
    let signature = state
        .signer
        .sign_claim(
            U256::from(chain_pid),
            recipient,
            U256::from(amount),
            U256::from(nonce_num),
            U256::from(deadline_num),
            chain_id,
            contract_addr,
        )
        .await?;
    let sig_hex = hex::encode(&signature[..]);

    // 4. 构造交易 calldata
    let calldata = encode_claim_call(
        &format!("0x{:x}", chain_pid), user_addr, &amount.to_string(),
        &nonce_num.to_string(), &deadline_num.to_string(),
        &format!("0x{}", sig_hex),
    );

    // 5. 从 chain_configs 获取 RPC URL
    let rpc = sqlx::query_as::<_, (String,)>(
        "SELECT rpc_url FROM chain_configs WHERE contract_address=$1",
    )
    .bind(&chain_row.1)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::BadRequest("RPC URL not found".to_string()))?;

    if rpc.0.is_empty() {
        return Err(ApiError::Internal("RPC URL not configured".into()));
    }

    // 6. 用 Relayer 钱包通过 eth_sendTransaction 广播
    let relayer_addr = format!("{:?}", state.relayer.address());
    let client = reqwest::Client::new();
    let rpc_resp = client
        .post(&rpc.0)
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "eth_sendTransaction",
            "params": [{
                "from": relayer_addr,
                "to": format!("0x{}", contract_addr.to_fixed_bytes().iter().map(|b| format!("{:02x}", b)).collect::<String>()),
                "data": calldata,
                "gas": "0x50000"
            }],
            "id": 1
        }))
        .send()
        .await
        .map_err(|e| ApiError::Internal(format!("RPC call failed: {}", e)))?;

    let rpc_body: serde_json::Value = rpc_resp.json().await
        .map_err(|e| ApiError::Internal(format!("RPC parse error: {}", e)))?;

    if let Some(err) = rpc_body.get("error") {
        return Err(ApiError::Internal(format!(
            "RPC error: {}", err.get("message").and_then(|m| m.as_str()).unwrap_or("unknown")
        )));
    }

    let tx_hash = rpc_body.get("result").and_then(|r| r.as_str())
        .ok_or_else(|| ApiError::Internal("RPC did not return tx_hash".into()))?;

    // 7. 记录
    sqlx::query(
        r#"INSERT INTO claims (packet_id,recipient_address,amount,fee,nonce,signature,claim_type,status,tx_hash)
           VALUES($1,$2,$3,'0',$4,$5,'proxy','confirmed',$6)
           ON CONFLICT (packet_id, recipient_address) DO UPDATE SET status='confirmed', tx_hash=$6"#,
    )
    .bind(packet_id)
    .bind(user_addr)
    .bind(amount.to_string())
    .bind(nonce_num.to_string())
    .bind(&sig_hex)
    .bind(tx_hash)
    .execute(&state.db)
    .await?;

    tracing::info!("Proxy claim: packet={} user={} tx={}", packet_id, user_addr, tx_hash);

    Ok(Json(json!({
        "status": "confirmed",
        "tx_hash": tx_hash,
        "packet_id": packet_id,
        "amount": amount.to_string(),
    })))
}
