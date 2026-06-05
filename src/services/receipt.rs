use crate::error::{ApiError, ApiResult};

const PACKET_CREATED_TOPIC: &str = "0xfeff5a9e606d1624a79886b6246af01ceaa9da69ba26af2782921a21b02bc524";

/// 通过 RPC 轮询交易回执，解析 PacketCreated 事件中的 onchain_packet_id
pub async fn fetch_onchain_packet_id(rpc_url: &str, tx_hash: &str) -> ApiResult<Option<u64>> {
    let client = reqwest::Client::new();

    for i in 0..10 {
        let resp = client
            .post(rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "eth_getTransactionReceipt",
                "params": [tx_hash],
                "id": 1
            }))
            .send()
            .await
            .map_err(|e| ApiError::Internal(format!("RPC error: {}", e)))?;

        let body: serde_json::Value = resp.json().await
            .map_err(|e| ApiError::Internal(format!("RPC parse: {}", e)))?;

        if body.get("result").and_then(|r| r.as_object()).is_none() {
            if i < 9 {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                continue;
            }
            return Ok(None);
        }

        let empty_logs = vec![];
        let logs = body["result"]["logs"].as_array().unwrap_or(&empty_logs);
        for log in logs {
            let empty_topics = vec![];
            let topics = log["topics"].as_array().unwrap_or(&empty_topics);
            if topics.is_empty() { continue; }
            if topics[0].as_str() == Some(PACKET_CREATED_TOPIC) {
                if let Some(hex) = topics[1].as_str() {
                    let s = hex.trim_start_matches("0x");
                    let id = u64::from_str_radix(s, 16).unwrap_or(0);
                    return Ok(Some(id));
                }
            }
        }
        return Ok(None);
    }
    Ok(None)
}
