-- =============================================
-- 012: 上线 Avalanche Fuji 测试网（AVAX C-Chain 测试网）
-- =============================================
-- 目标：对外只展示 AVAX-FUJI 一条链，其余链停用但保留配置（随时可重新启用）
--
-- 合约（RedPacketV1, UUPS 代理）:
--   代理    0x7dc7013fA5bFd9d29323206D5759138494b31FeB   <-- chain_configs 必须填这个
--   实现    0x03dc08c8bb7aA22de9f53b8Ebc42F9d4957448F9
--   owner   0x1c1F5DD3092eDd1108AD59Bc5B97764B3fe2068A
--
-- ⚠️ contract_address 必须是【代理】地址：EIP-712 的 verifyingContract 用的是它，
--    填实现地址会导致链上验签全部失败。
--
-- 幂等：scripts/init-db.sh 每次全量重跑所有迁移文件（无版本表），
--       所以这里全部用 ON CONFLICT / 条件 UPDATE 写法。

-- 1. 写入/更新 AVAX-FUJI 链配置
INSERT INTO chain_configs (chain, chain_id, rpc_url, contract_address, explorer_url, is_active)
VALUES (
    'AVAX-FUJI',
    43113,
    'https://api.avax-test.network/ext/bc/C/rpc',
    '0x7dc7013fA5bFd9d29323206D5759138494b31FeB',
    'https://testnet.snowtrace.io',
    true
)
ON CONFLICT (chain) DO UPDATE SET
    chain_id         = EXCLUDED.chain_id,
    rpc_url          = EXCLUDED.rpc_url,
    contract_address = EXCLUDED.contract_address,
    explorer_url     = EXCLUDED.explorer_url,
    is_active        = EXCLUDED.is_active;

-- 2. 其余链全部停用（只下架不删除，重新启用见下方说明）
--    重新启用某条链：UPDATE chain_configs SET is_active = true WHERE chain = 'LOCAL';
--    ⚠️ 重新启用前先核对它的 contract_address 是否还是当前部署的代理地址
UPDATE chain_configs
   SET is_active = false
 WHERE chain <> 'AVAX-FUJI';

-- 3. AVAX-FUJI 的原生代币（前端 TokenSelector 用）
INSERT INTO token_configs (chain, token_address, symbol, name, decimals, is_native, token_type, sort_order)
VALUES ('AVAX-FUJI', 'native', 'AVAX', 'Avalanche', 18, true, 'native', 0)
ON CONFLICT (chain, token_address) DO NOTHING;
