-- RedPacket: 代币标准类型 + 启用状态

-- 1. 新增 token_type 字段（代币标准）
ALTER TABLE token_configs ADD COLUMN IF NOT EXISTS token_type VARCHAR(16) NOT NULL DEFAULT 'erc20';

-- 2. 新增 is_active 字段（启用/禁用）
ALTER TABLE token_configs ADD COLUMN IF NOT EXISTS is_active BOOLEAN NOT NULL DEFAULT true;

-- 3. 迁移现有数据
--    is_native=true → token_type='native'
UPDATE token_configs SET token_type = 'native' WHERE is_native = true;
--    ERC20 链的 token_type 保持 'erc20'（默认值，无需更新）
--    SOLANA 链的代币后面单独插入

-- 4. 补一些 logo_url（仅示例，可后续在 admin UI 中修改）
-- UPDATE token_configs SET logo_url = 'https://cryptologos.cc/logos/ethereum-eth-logo.png' WHERE symbol = 'ETH';

-- 5. 为 SOLANA 链插入默认代币
INSERT INTO token_configs (chain, token_address, symbol, name, decimals, is_native, token_type, sort_order) VALUES
    ('SOLANA', 'native',                                                                        'SOL',  'Solana',            9,  true,  'native',       0),
    ('SOLANA', 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',                                'USDC', 'USD Coin',          6,  false, 'spl-token',    1),
    ('SOLANA', 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB',                                'USDT', 'Tether USD',        6,  false, 'spl-token',    2),
    ('SOLANA', 'mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytf3JQPdt',                                  'mSOL', 'Marinade staked SOL', 9, false, 'spl-token',    3),
    ('SOLANA', 'DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263',                                'BONK', 'Bonk',              5,  false, 'spl-token',    4),
    ('SOLANA', 'EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm',                                'WIF',  'dogwifhat',         6,  false, 'spl-token',    5)
ON CONFLICT (chain, token_address) DO NOTHING;

-- 6. 也补几条 TRON（预配置，方便后续）
INSERT INTO token_configs (chain, token_address, symbol, name, decimals, is_native, token_type, sort_order) VALUES
    ('TRON', 'native',    'TRX',  'TRON',             6,  true,  'native',   0),
    ('TRON', 'TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t', 'USDT', 'Tether USD', 6, false, 'trc20',    1)
ON CONFLICT (chain, token_address) DO NOTHING;
