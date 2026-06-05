-- 添加链上 packet ID 字段，用于 claim 签名时匹配合约内的 packetId
ALTER TABLE packets ADD COLUMN IF NOT EXISTS onchain_packet_id BIGINT;
CREATE INDEX IF NOT EXISTS idx_packets_onchain_id ON packets(onchain_packet_id);
