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

CREATE INDEX IF NOT EXISTS idx_token_configs_chain ON token_configs(chain);

-- 为每条已配置的链插入默认代币
INSERT INTO token_configs (chain, token_address, symbol, name, decimals, is_native, sort_order) VALUES
    -- LOCAL 开发链
    ('LOCAL', 'native',   'ETH', 'Local ETH',        18, true,  0),
    ('LOCAL', '0x5FbDB2315678afecb367f032d93F642f64180aa3', 'RPT', 'RedPacket Test Token', 18, false, 1),

    -- Ethereum
    ('ETH',   'native',   'ETH', 'Ether',             18, true,  0),
    ('ETH',   '0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48', 'USDC', 'USD Coin',           6,  false, 1),
    ('ETH',   '0xdAC17F958D2ee523a2206206994597C13D831ec7', 'USDT', 'Tether USD',         6,  false, 2),
    ('ETH',   '0x6B175474E89094C44Da98b954EedeAC495271d0F', 'DAI',  'Dai Stablecoin',     18, false, 3),
    ('ETH',   '0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599', 'WBTC', 'Wrapped Bitcoin',    8,  false, 4),
    ('ETH',   '0x7D1AfA7B718fb893dB30A3aBc0Cfc608AaCfeBB0', 'MATIC','Polygon',            18, false, 5),

    -- BSC
    ('BSC',   'native',   'BNB', 'Binance Coin',      18, true,  0),
    ('BSC',   '0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d', 'USDC', 'USD Coin',           18, false, 1),
    ('BSC',   '0x55d398326f99059fF775485246999027B3197955', 'USDT', 'Tether USD',         18, false, 2),

    -- AB-Core
    ('AB-Core', 'native', 'ABC', 'AB-Core Coin',       18, true,  0),

    -- AB-iOT
    ('AB-iOT', 'native',  'ABT', 'AB-iOT Token',       18, true,  0)
ON CONFLICT (chain, token_address) DO NOTHING;
