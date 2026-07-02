-- RedPacket 代币配置表
-- 每个链的默认代币列表，前端 TokenSelector 使用

CREATE TABLE IF NOT EXISTS token_configs (
    id SERIAL PRIMARY KEY,
    chain VARCHAR(32) NOT NULL,
    token_address VARCHAR(64) NOT NULL,
    symbol VARCHAR(32) NOT NULL,
    name VARCHAR(128) NOT NULL,
    decimals INT NOT NULL DEFAULT 18,
    is_native BOOLEAN NOT NULL DEFAULT false,
    logo_url VARCHAR(512),
    sort_order INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(chain, token_address)
);

CREATE INDEX idx_token_configs_chain ON token_configs(chain);

-- 为每条已配置的链插入默认代币
INSERT INTO token_configs (chain, token_address, symbol, name, decimals, is_native, sort_order) VALUES
    ('LOCAL', 'native', 'ETH', 'Local ETH', 18, true, 0),
    ('ETH', 'native', 'ETH', 'Ether', 18, true, 0),
    ('BSC', 'native', 'BNB', 'Binance Coin', 18, true, 0)
ON CONFLICT (chain, token_address) DO NOTHING;
