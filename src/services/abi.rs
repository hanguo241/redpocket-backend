// ===================== ABI 编码工具 =====================
//
// 提供合约调用数据的 hex 编码。
// TODO: 使用 ethers::core::abi 代替手动 hex 拼接

use ethers::core::types::U256;

/// hex 字符串左补零到 64 字符（32 字节，ABI uint256/address）
pub fn pad32(hex: &str) -> String {
    let s = hex.trim_start_matches("0x");
    format!("{:0>64}", s)
}

/// u64 → 32 字节 hex (ABI uint256)
pub fn u64_to_u256(val: u64) -> String {
    format!("{:0>64}", format!("{:x}", val))
}

/// 十进制字符串 → 32 字节 hex (ABI uint256)
pub fn dec_to_u256(val: &str) -> String {
    let trimmed = val.trim_start_matches("0x");
    let num = U256::from_dec_str(trimmed).unwrap_or_else(|_| U256::zero());
    format!("{:0>64}", format!("{:x}", num))
}

/// wei 字符串 → 32 字节 hex (ABI uint256)
pub fn wei_to_u256(val: &str) -> String {
    if val.is_empty() || val == "0" {
        return format!("{:0>64}", "0");
    }
    let num = U256::from_dec_str(val).unwrap_or_else(|_| U256::zero());
    format!("{:0>64}", format!("{:x}", num))
}

// ===================== createPacket 编码 =====================

/// 编码 createPacketNative(uint256,uint8,uint8,address,uint256,uint256,uint256,uint256,uint256,address,uint256)
/// selector: 0x1c303458
pub fn encode_create_packet_native(
    head_count: u32,
    packet_type: u8,
    sub_type: u8,
    signer: &str,
    start_time: u64,
    end_time: u64,
    min_ratio_bps: u32,
    max_ratio_bps: u32,
    fee_bps: u32,
    fee_collector: &str,
    gas_reserve: &str,
) -> String {
    let mut d = String::from("0x1c303458");
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
    d.push_str(&wei_to_u256(gas_reserve));
    d
}

/// 编码 createPacketERC20(address,uint256,uint256,uint8,uint8,address,uint256,uint256,uint256,uint256,uint256,address,uint256)
/// selector: 0xcfa88fcb
pub fn encode_create_packet_erc20(
    token: &str,
    total_amount: &str,
    head_count: u32,
    packet_type: u8,
    sub_type: u8,
    signer: &str,
    start_time: u64,
    end_time: u64,
    min_ratio_bps: u32,
    max_ratio_bps: u32,
    fee_bps: u32,
    fee_collector: &str,
    gas_reserve: &str,
) -> String {
    let mut d = String::from("0xcfa88fcb");
    d.push_str(&pad32(if token == "native" {
        "0x0000000000000000000000000000000000000000"
    } else {
        token
    }));
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
    d.push_str(&wei_to_u256(gas_reserve));
    d
}

// ===================== claim 编码 =====================

/// 编码 claim(uint256,address,uint256,uint256,uint256,bytes) 调用数据
/// selector: 0xedc19c89
pub fn encode_claim(
    onchain_packet_id: &str,
    recipient: &str,
    amount: &str,
    nonce: &str,
    deadline: &str,
    signature_hex: &str,
) -> String {
    let sig = signature_hex.trim_start_matches("0x");
    let sig_len = sig.len() / 2;
    let padded_len = ((sig.len() + 63) / 64) * 64;

    let mut d = String::from("0xedc19c89");
    d.push_str(&pad32(onchain_packet_id));
    d.push_str(&pad32(recipient));
    d.push_str(&dec_to_u256(amount));
    d.push_str(&dec_to_u256(nonce));
    d.push_str(&dec_to_u256(deadline));
    d.push_str(&u64_to_u256(192)); // offset to bytes (6*32=192)
    d.push_str(&u64_to_u256(sig_len as u64)); // length
    d.push_str(&format!("{:0<width$}", sig, width = padded_len));
    d
}

/// 编码 claimFor(uint256,address,uint256,uint256,uint256,bytes) 调用数据
/// selector: 0x91215604 — 与 claim 参数相同，但合约会从 gasReserve 报销 gas
pub fn encode_claim_for(
    onchain_packet_id: &str,
    recipient: &str,
    amount: &str,
    nonce: &str,
    deadline: &str,
    signature_hex: &str,
) -> String {
    let sig = signature_hex.trim_start_matches("0x");
    let sig_len = sig.len() / 2;
    let padded_len = ((sig.len() + 63) / 64) * 64;

    let mut d = String::from("0x91215604");
    d.push_str(&pad32(onchain_packet_id));
    d.push_str(&pad32(recipient));
    d.push_str(&dec_to_u256(amount));
    d.push_str(&dec_to_u256(nonce));
    d.push_str(&dec_to_u256(deadline));
    d.push_str(&u64_to_u256(192));
    d.push_str(&u64_to_u256(sig_len as u64));
    d.push_str(&format!("{:0<width$}", sig, width = padded_len));
    d
}

// ===================== admin 编码 =====================

/// 编码 withdrawPlatformFees(address,address,uint256)
/// selector: 0xf9cf5261
pub fn encode_withdraw_platform_fees(token: &str, to: &str, amount: &str) -> String {
    let token_addr = if token == "native" {
        "0x0000000000000000000000000000000000000000"
    } else {
        token
    };

    let mut d = String::from("0xf9cf5261");
    d.push_str(&pad32(token_addr));
    d.push_str(&pad32(to));
    d.push_str(&dec_to_u256(amount));
    d
}
