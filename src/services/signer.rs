use ethers::{
    core::{
        k256::ecdsa::SigningKey,
        types::{Bytes, H160, U256},
        utils::keccak256,
    },
    signers::{Signer, Wallet},
};

use crate::error::{ApiError, ApiResult};

/// EIP-712 签名服务 — 手动实现哈希计算
#[derive(Clone)]
pub struct SignerService {
    wallet: Wallet<SigningKey>,
}

impl SignerService {
    pub fn new(private_key: &str) -> ApiResult<Self> {
        let wallet: Wallet<SigningKey> = private_key
            .parse()
            .map_err(|e| ApiError::Crypto(format!("Invalid signer: {}", e)))?;
        Ok(Self { wallet })
    }

    pub fn address(&self) -> H160 {
        self.wallet.address()
    }

    pub async fn sign_claim(
        &self,
        packet_id: U256,
        recipient: H160,
        amount: U256,
        nonce: U256,
        deadline: U256,
        chain_id: u64,
        contract_address: H160,
    ) -> ApiResult<Bytes> {
        let digest = Self::hash_eip712(packet_id, recipient, amount, nonce, deadline, chain_id, contract_address);
        let mut sig = self.wallet.sign_hash(digest.into()).map_err(|e| ApiError::Crypto(e.to_string()))?;
        // ethers-rs 的 to_vec 中 v 可能是 0/1，合约需要 27/28
        let v = if sig.v < 27 { sig.v + 27 } else { sig.v };
        let mut bytes = Vec::with_capacity(65);
        let mut buf = [0u8; 32];
        sig.r.to_big_endian(&mut buf);
        bytes.extend_from_slice(&buf);
        sig.s.to_big_endian(&mut buf);
        bytes.extend_from_slice(&buf);
        bytes.push(v as u8);
        Ok(Bytes::from(bytes))
    }

    /// 完整的 EIP-712 digest: keccak256("\x19\x01" || domainSeparator || structHash)
    pub fn hash_eip712(
        packet_id: U256, recipient: H160, amount: U256, nonce: U256, deadline: U256,
        chain_id: u64, contract_address: H160,
    ) -> [u8; 32] {
        let ds = Self::hash_domain(chain_id, contract_address);
        let sh = Self::hash_struct(packet_id, recipient, amount, nonce, deadline);
        let mut buf = Vec::with_capacity(66);
        buf.extend_from_slice(&[0x19, 0x01]);
        buf.extend_from_slice(&ds);
        buf.extend_from_slice(&sh);
        keccak256(&buf)
    }

    /// domainSeparator = keccak256(abi.encode(typeHash, name, version, chainId, contract))
    fn hash_domain(chain_id: u64, contract_address: H160) -> [u8; 32] {
        let mut buf = Vec::with_capacity(32 * 5);
        buf.extend_from_slice(&keccak256(
            b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
        ));
        buf.extend_from_slice(&keccak256(b"RedPacket"));
        buf.extend_from_slice(&keccak256(b"1"));
        let mut tmp = [0u8; 32];
        // chainId as uint256 = 左补零到 32 字节
        tmp[24..].copy_from_slice(&chain_id.to_be_bytes());
        buf.extend_from_slice(&tmp);
        // contract as address = 左补零到 32 字节
        tmp = [0u8; 32];
        tmp[12..].copy_from_slice(&contract_address.0);
        buf.extend_from_slice(&tmp);
        keccak256(&buf)
    }

    /// structHash = keccak256(abi.encode(typeHash, packetId, recipient, amount, nonce, deadline))
    fn hash_struct(
        packet_id: U256, recipient: H160, amount: U256, nonce: U256, deadline: U256,
    ) -> [u8; 32] {
        let type_hash = keccak256(
            b"ClaimPayload(uint256 packetId,address recipient,uint256 amount,uint256 nonce,uint256 deadline)",
        );
        let mut buf = Vec::with_capacity(32 * 6);
        buf.extend_from_slice(&type_hash);
        let mut tmp = [0u8; 32];
        packet_id.to_big_endian(&mut tmp);
        buf.extend_from_slice(&tmp);
        // recipient: left-pad to 32
        tmp = [0u8; 32];
        tmp[12..].copy_from_slice(&recipient.0);
        buf.extend_from_slice(&tmp);
        amount.to_big_endian(&mut tmp);
        buf.extend_from_slice(&tmp);
        nonce.to_big_endian(&mut tmp);
        buf.extend_from_slice(&tmp);
        deadline.to_big_endian(&mut tmp);
        buf.extend_from_slice(&tmp);
        keccak256(&buf)
    }
}
