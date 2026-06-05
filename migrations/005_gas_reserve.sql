-- RedPacket gas 准备金 + 系统配置

-- 1. packets 表增加 gas 准备金相关字段
ALTER TABLE packets ADD COLUMN IF NOT EXISTS gas_reserve_wei VARCHAR(78) NOT NULL DEFAULT '0';
ALTER TABLE packets ADD COLUMN IF NOT EXISTS gas_used_wei VARCHAR(78) NOT NULL DEFAULT '0';
ALTER TABLE packets ADD COLUMN IF NOT EXISTS gas_estimate_multiplier REAL NOT NULL DEFAULT 1.2;

-- 2. 系统配置表（动态可调参数）
CREATE TABLE IF NOT EXISTS system_config (
    key VARCHAR(64) PRIMARY KEY,
    value TEXT NOT NULL,
    description VARCHAR(256) NOT NULL DEFAULT '',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 默认配置
INSERT INTO system_config (key, value, description) VALUES
    ('gas_per_claim', '100000', '每笔 claim 预估 gas 上限')
ON CONFLICT (key) DO NOTHING;
