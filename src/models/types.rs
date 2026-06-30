use serde::{Deserialize, Serialize};

// ===================== 红包类型枚举 =====================

/// 红包类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PacketType {
    Normal,
    Password,
    Condition,
}

impl PacketType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PacketType::Normal => "normal",
            PacketType::Password => "password",
            PacketType::Condition => "condition",
        }
    }

    /// ABI packet_type 编码值: normal=0, password=1, condition=2
    pub fn to_abi_u8(&self) -> u8 {
        match self {
            PacketType::Normal => 0u8,
            PacketType::Password => 1u8,
            PacketType::Condition => 2u8,
        }
    }
}

impl std::fmt::Display for PacketType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for PacketType {
    fn from(s: &str) -> Self {
        match s {
            "password" => PacketType::Password,
            "condition" => PacketType::Condition,
            _ => PacketType::Normal,
        }
    }
}

// ===================== 子类型枚举 =====================

/// 红包金额分配方式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SubType {
    Average,
    Random,
}

impl SubType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SubType::Average => "average",
            SubType::Random => "random",
        }
    }

    /// ABI sub_type 编码值: average=0, random=1
    pub fn to_abi_u8(&self) -> u8 {
        match self {
            SubType::Average => 0u8,
            SubType::Random => 1u8,
        }
    }
}

impl std::fmt::Display for SubType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for SubType {
    fn from(s: &str) -> Self {
        match s {
            "random" => SubType::Random,
            _ => SubType::Average,
        }
    }
}

// ===================== 领取模式枚举 =====================

/// 领取模式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimMode {
    #[serde(rename = "self")]
    Self_,
    Proxy,
    Both,
}

impl ClaimMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ClaimMode::Self_ => "self",
            ClaimMode::Proxy => "proxy",
            ClaimMode::Both => "both",
        }
    }
}

impl std::fmt::Display for ClaimMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for ClaimMode {
    fn from(s: &str) -> Self {
        match s {
            "self" => ClaimMode::Self_,
            "proxy" => ClaimMode::Proxy,
            _ => ClaimMode::Both,
        }
    }
}

// ===================== 红包状态枚举 =====================

/// 红包链上状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PacketStatus {
    Active,
    Completed,
    Expired,
    Refunded,
}

impl PacketStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PacketStatus::Active => "active",
            PacketStatus::Completed => "completed",
            PacketStatus::Expired => "expired",
            PacketStatus::Refunded => "refunded",
        }
    }
}

impl std::fmt::Display for PacketStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for PacketStatus {
    fn from(s: &str) -> Self {
        match s {
            "completed" => PacketStatus::Completed,
            "expired" => PacketStatus::Expired,
            "refunded" => PacketStatus::Refunded,
            _ => PacketStatus::Active,
        }
    }
}

// ===================== 金额值类型 =====================

/// Wei 金额 — 在序列化时保持为十进制字符串
///
/// 内部用 String 存储以避免 u128 溢出和精度损失，
/// 同时确保前端接收到的 JSON 始终是字符串（而非数字）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Wei(pub String);

impl Wei {
    pub fn zero() -> Self {
        Wei("0".to_string())
    }

    pub fn new(s: &str) -> Self {
        Wei(s.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 尝试解析为 u128
    pub fn to_u128(&self) -> Option<u128> {
        self.0.parse().ok()
    }
}

impl Serialize for Wei {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Wei {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Wei(s))
    }
}

impl std::fmt::Display for Wei {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
