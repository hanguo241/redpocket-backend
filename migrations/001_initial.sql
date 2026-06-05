-- RedPacket 后端数据库初始化

-- 项目方表
CREATE TABLE IF NOT EXISTS projects (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    app_key VARCHAR(64) NOT NULL UNIQUE,
    app_secret VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_projects_app_key ON projects(app_key);

-- 红包表
CREATE TABLE IF NOT EXISTS packets (
    id UUID PRIMARY KEY,
    project_id UUID REFERENCES projects(id),
    chain VARCHAR(32) NOT NULL,
    contract_address VARCHAR(64) NOT NULL DEFAULT '',
    creator_address VARCHAR(64) NOT NULL DEFAULT '',
    token_address VARCHAR(64) NOT NULL,
    total_amount VARCHAR(78) NOT NULL,        -- UINT256 max
    remaining_amount VARCHAR(78) NOT NULL,
    head_count INT NOT NULL,
    packet_type VARCHAR(16) NOT NULL,         -- normal | password | condition
    sub_type VARCHAR(16) NOT NULL,            -- average | random
    claim_mode VARCHAR(8) NOT NULL DEFAULT 'both',  -- self | proxy | both
    password_hash VARCHAR(64),
    start_time BIGINT NOT NULL DEFAULT 0,
    end_time BIGINT NOT NULL,
    signer_address VARCHAR(64) NOT NULL,
    fee_bps INT NOT NULL DEFAULT 100,
    fee_collector VARCHAR(64) NOT NULL,
    status VARCHAR(16) NOT NULL DEFAULT 'active',  -- active | completed | expired | refunded
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_packets_project ON packets(project_id);
CREATE INDEX idx_packets_status ON packets(status);

-- 领取记录表
CREATE TABLE IF NOT EXISTS claims (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    packet_id UUID NOT NULL REFERENCES packets(id),
    recipient_address VARCHAR(64) NOT NULL,
    amount VARCHAR(78) NOT NULL,
    fee VARCHAR(78) NOT NULL DEFAULT '0',
    nonce VARCHAR(78) NOT NULL,
    signature TEXT NOT NULL,
    claim_type VARCHAR(8) NOT NULL DEFAULT 'self',  -- self | proxy
    tx_hash VARCHAR(128),
    status VARCHAR(16) NOT NULL DEFAULT 'pending',  -- pending | confirmed | failed
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_claims_packet ON claims(packet_id);
CREATE INDEX idx_claims_recipient ON claims(recipient_address);
CREATE UNIQUE INDEX idx_claims_packet_recipient ON claims(packet_id, recipient_address);

-- 配置表（存储各链合约地址、RPC 等）
CREATE TABLE IF NOT EXISTS chain_configs (
    id SERIAL PRIMARY KEY,
    chain VARCHAR(32) NOT NULL UNIQUE,
    chain_id BIGINT NOT NULL,
    rpc_url VARCHAR(512) NOT NULL,
    contract_address VARCHAR(64) NOT NULL,
    explorer_url VARCHAR(512),
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO chain_configs (chain, chain_id, rpc_url, contract_address) VALUES
    ('ETH', 1, '', ''),
    ('BSC', 56, '', ''),
    ('AB-Core', 123, '', ''),
    ('AB-iOT', 456, '', ''),
    ('SOLANA', 0, '', ''),    -- Solana 不走 EVM 合约
    ('TRON', 0, '', '')       -- Tron 特殊处理
ON CONFLICT (chain) DO NOTHING;
