// ===================== ABI 编码工具 =====================
//
// 使用 ethers-rs 提供的标准 ABI 编码替代手动 hex 拼接。
// 所有函数返回 "0x" 前缀的 hex 字符串，符合 RPC 调用格式。

use ethers::core::types::{H160, U256};
use ethers::core::utils::keccak256;

#[cfg(test)]
mod tests {
    use super::*;

    fn test_selector(signature: &str, expected_hex: &str) {
        let actual = hex::encode(fn_selector(signature));
        assert_eq!(actual, expected_hex, "selector mismatch for {}", signature);
    }

    #[test]
    fn test_all_selectors() {
        test_selector("createPacketNative(uint256,uint8,uint8,address,uint256,uint256,uint256,uint256,uint256,address,uint256)", "1c303458");
        test_selector("createPacketERC20(address,uint256,uint256,uint8,uint8,address,uint256,uint256,uint256,uint256,uint256,address,uint256)", "cfa88fcb");
        test_selector("claim(uint256,address,uint256,uint256,uint256,bytes)", "edc19c89");
        test_selector("claimFor(uint256,address,uint256,uint256,uint256,bytes)", "91215604");
        test_selector("withdrawPlatformFees(address,address,uint256)", "f9cf5261");
    }

    #[test]
    fn test_encode_create_packet_native() {
        let result = encode_create_packet_native(
            10,        // head_count
            0,         // packet_type (normal)
            1,         // sub_type (random)
            "0x1234567890123456789012345678901234567890",
            0,         // start_time
            1000000,   // end_time
            7000,      // min_ratio_bps
            10000,     // max_ratio_bps
            20,        // fee_bps
            "0x2222222222222222222222222222222222222222",
            "1000000", // gas_reserve
        );
        assert!(result.starts_with("0x1c303458"), "should start with createPacketNative selector");
        assert_eq!(result.len() % 2, 0, "hex should have even length");
    }

    #[test]
    fn test_encode_create_packet_erc20() {
        let result = encode_create_packet_erc20(
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", // USDC
            "1000000000",
            5,
            1,     // password
            0,     // average
            "0x1234567890123456789012345678901234567890",
            0,
            2000000,
            7000,
            10000,
            20,
            "0x2222222222222222222222222222222222222222",
            "500000",
        );
        assert!(result.starts_with("0xcfa88fcb"), "should start with createPacketERC20 selector");
    }

    #[test]
    fn test_encode_claim() {
        let result = encode_claim(
            "0x42",
            "0x1234567890123456789012345678901234567890",
            "5000000000000000000",
            "1700000000",
            "1700001800",
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1b",
        );
        assert!(result.starts_with("0xedc19c89"), "should start with claim selector");
    }

    #[test]
    fn test_encode_claim_for() {
        let result = encode_claim_for(
            "0x42",
            "0x1234567890123456789012345678901234567890",
            "5000000000000000000",
            "1700000000",
            "1700001800",
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1b",
        );
        assert!(result.starts_with("0x91215604"), "should start with claimFor selector");
    }

    #[test]
    fn test_encode_withdraw_platform_fees() {
        let result = encode_withdraw_platform_fees(
            "native",
            "0x1234567890123456789012345678901234567890",
            "1000000000000000000",
        );
        assert!(result.starts_with("0xf9cf5261"), "should start with withdraw selector");
    }

    /// 验证新编码输出长度与旧实现预期一致
    #[test]
    fn test_output_lengths() {
        // createPacketNative: selector(4) + 11 * uint256(32) = 4 + 352 = 356 bytes = 712 hex chars + "0x"
        let native = encode_create_packet_native(10, 0, 1, "0x1111111111111111111111111111111111111111", 0, 100, 7000, 10000, 20, "0x2222222222222222222222222222222222222222", "0");
        let native_bytes = (native.len() - 2) / 2; // hex chars without "0x" → bytes
        assert_eq!(native_bytes, 4 + 32 * 11, "createPacketNative should be 4 + 11*32 bytes");

        // claim: selector(4) + 5*uint256(32) + bytes(dynamic) = 4 + 160 + 32(offset) + 32(length) + padded_sig
        let claim = encode_claim("0x1", "0x1111111111111111111111111111111111111111", "100", "1", "1000", "0xab");
        // offset = 6*32=192, length=1, padded to 32 → total: 4 + 5*32 + 32(offset) + 32(length) + 32(padded bytes) = 4 + 160 + 96 = 260
        assert_eq!((claim.len() - 2) / 2, 260, "claim with 1-byte sig should be 260 bytes");
    }
}

/// 从 Solidity 函数签名计算 4 字节 selector
fn fn_selector(signature: &str) -> Vec<u8> {
    keccak256(signature.as_bytes())[..4].to_vec()
}

/// ABI 编码参数部分（不含 selector）
fn abi_encode(params: &[ethers::core::abi::Token]) -> Vec<u8> {
    ethers::core::abi::encode(params)
}

/// 组合 selector + ABI 编码参数 → "0x" 前缀 hex 字符串
fn encode_calldata(signature: &str, params: &[ethers::core::abi::Token]) -> String {
    let selector = fn_selector(signature);
    let encoded = abi_encode(params);
    let full = [selector.as_slice(), encoded.as_slice()].concat();
    format!("0x{}", hex::encode(full))
}

/// 解析十六进制地址字符串 → H160，遇到 "native" 返回零地址
fn parse_addr(s: &str) -> H160 {
    let cleaned = if s == "native" || s.is_empty() {
        "0x0000000000000000000000000000000000000000"
    } else {
        s.trim_start_matches("0x")
    };
    // 如果缺少 0x 前缀则补上
    let with_prefix = if cleaned.starts_with("0x") {
        cleaned.to_string()
    } else {
        format!("0x{}", cleaned)
    };
    with_prefix.parse().unwrap_or_else(|_| {
        tracing::error!("abi: invalid address '{}', using zero address", s);
        H160::zero()
    })
}

/// 解析十进制字符串 → U256
fn parse_u256(s: &str) -> U256 {
    let trimmed = s.trim_start_matches("0x");
    U256::from_dec_str(trimmed).unwrap_or_else(|_| {
        tracing::error!("abi: invalid uint256 decimal '{}', using 0", s);
        U256::zero()
    })
}

/// 解析十六进制字符串 → Vec<u8> (bytes 类型)
fn parse_hex(s: &str) -> Vec<u8> {
    hex::decode(s.trim_start_matches("0x")).unwrap_or_else(|_| {
        tracing::error!("abi: invalid hex '{}', using empty bytes", s);
        vec![]
    })
}

/// 创建一个 H160 地址的 Token
fn addr_token(s: &str) -> ethers::core::abi::Token {
    ethers::core::abi::Token::Address(parse_addr(s))
}

/// 创建一个 U256 的 Token（从十进制字符串）
fn uint_token(s: &str) -> ethers::core::abi::Token {
    ethers::core::abi::Token::Uint(parse_u256(s))
}

/// 创建一个 u64 的 U256 Token
fn u64_token(v: u64) -> ethers::core::abi::Token {
    ethers::core::abi::Token::Uint(U256::from(v))
}

fn bytes_token(s: &str) -> ethers::core::abi::Token {
    ethers::core::abi::Token::Bytes(parse_hex(s))
}

// ===================== createPacket 编码 =====================

/// 编码 createPacketNative(uint256,uint8,uint8,address,uint256,uint256,uint256,uint256,uint256,address,uint256)
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
    // verify selector matches contract
    debug_assert_eq!(
        hex::encode(fn_selector("createPacketNative(uint256,uint8,uint8,address,uint256,uint256,uint256,uint256,uint256,address,uint256)")),
        "1c303458",
        "selector mismatch for createPacketNative"
    );

    encode_calldata(
        "createPacketNative(uint256,uint8,uint8,address,uint256,uint256,uint256,uint256,uint256,address,uint256)",
        &[
            u64_token(head_count as u64),
            u64_token(packet_type as u64),   // uint8 → ABI uint256
            u64_token(sub_type as u64),      // uint8 → ABI uint256
            addr_token(signer),
            u64_token(start_time),
            u64_token(end_time),
            u64_token(min_ratio_bps as u64),
            u64_token(max_ratio_bps as u64),
            u64_token(fee_bps as u64),
            addr_token(fee_collector),
            uint_token(gas_reserve),
        ],
    )
}

/// 编码 createPacketERC20(address,uint256,uint256,uint8,uint8,address,uint256,uint256,uint256,uint256,uint256,address,uint256)
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
    debug_assert_eq!(
        hex::encode(fn_selector("createPacketERC20(address,uint256,uint256,uint8,uint8,address,uint256,uint256,uint256,uint256,uint256,address,uint256)")),
        "cfa88fcb",
        "selector mismatch for createPacketERC20"
    );

    encode_calldata(
        "createPacketERC20(address,uint256,uint256,uint8,uint8,address,uint256,uint256,uint256,uint256,uint256,address,uint256)",
        &[
            addr_token(token),
            uint_token(total_amount),
            u64_token(head_count as u64),
            u64_token(packet_type as u64),
            u64_token(sub_type as u64),
            addr_token(signer),
            u64_token(start_time),
            u64_token(end_time),
            u64_token(min_ratio_bps as u64),
            u64_token(max_ratio_bps as u64),
            u64_token(fee_bps as u64),
            addr_token(fee_collector),
            uint_token(gas_reserve),
        ],
    )
}

// ===================== claim 编码 =====================

/// 编码 claim(uint256,address,uint256,uint256,uint256,bytes)
pub fn encode_claim(
    onchain_packet_id: &str,
    recipient: &str,
    amount: &str,
    nonce: &str,
    deadline: &str,
    signature_hex: &str,
) -> String {
    debug_assert_eq!(
        hex::encode(fn_selector("claim(uint256,address,uint256,uint256,uint256,bytes)")),
        "edc19c89",
        "selector mismatch for claim"
    );

    encode_calldata(
        "claim(uint256,address,uint256,uint256,uint256,bytes)",
        &[
            uint_token(onchain_packet_id),
            addr_token(recipient),
            uint_token(amount),
            uint_token(nonce),
            uint_token(deadline),
            bytes_token(signature_hex),
        ],
    )
}

/// 编码 claimFor(uint256,address,uint256,uint256,uint256,bytes)
pub fn encode_claim_for(
    onchain_packet_id: &str,
    recipient: &str,
    amount: &str,
    nonce: &str,
    deadline: &str,
    signature_hex: &str,
) -> String {
    debug_assert_eq!(
        hex::encode(fn_selector("claimFor(uint256,address,uint256,uint256,uint256,bytes)")),
        "91215604",
        "selector mismatch for claimFor"
    );

    encode_calldata(
        "claimFor(uint256,address,uint256,uint256,uint256,bytes)",
        &[
            uint_token(onchain_packet_id),
            addr_token(recipient),
            uint_token(amount),
            uint_token(nonce),
            uint_token(deadline),
            bytes_token(signature_hex),
        ],
    )
}

// ===================== admin 编码 =====================

/// 编码 withdrawPlatformFees(address,address,uint256)
pub fn encode_withdraw_platform_fees(token: &str, to: &str, amount: &str) -> String {
    debug_assert_eq!(
        hex::encode(fn_selector("withdrawPlatformFees(address,address,uint256)")),
        "f9cf5261",
        "selector mismatch for withdrawPlatformFees"
    );

    encode_calldata(
        "withdrawPlatformFees(address,address,uint256)",
        &[
            addr_token(token),
            addr_token(to),
            uint_token(amount),
        ],
    )
}
