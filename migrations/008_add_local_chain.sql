-- Add LOCAL chain config for local Hardhat development

INSERT INTO chain_configs (chain, chain_id, rpc_url, contract_address) VALUES
    ('LOCAL', 31337, 'http://localhost:8545', '0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512')
ON CONFLICT (chain) DO NOTHING;
