-- 给 packets 表添加 tx_hash 列，用于后台 onchain_id 补充索引
ALTER TABLE packets ADD COLUMN IF NOT EXISTS tx_hash VARCHAR(128);

CREATE INDEX IF NOT EXISTS idx_packets_tx_hash ON packets(tx_hash);
