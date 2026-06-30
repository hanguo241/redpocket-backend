-- RedPacket 平台手续费模型
-- gross_amount: 发红包人支付的红包总额
-- total_amount / remaining_amount: 扣除平台费后可领取的红包池
-- platform_fee_wei: 创建红包时计提的平台手续费

ALTER TABLE packets ADD COLUMN IF NOT EXISTS gross_amount VARCHAR(78) NOT NULL DEFAULT '0';
ALTER TABLE packets ADD COLUMN IF NOT EXISTS platform_fee_wei VARCHAR(78) NOT NULL DEFAULT '0';
ALTER TABLE packets ALTER COLUMN fee_bps SET DEFAULT 20;

UPDATE packets
SET gross_amount = total_amount
WHERE gross_amount = '0';

INSERT INTO system_config (key, value, description) VALUES
    ('platform_fee_bps', '20', '平台手续费，单位 bps，20 = 0.2% = 千分之二'),
    ('refund_window_seconds', '86400', '红包默认有效期/退款窗口，86400 = 24 小时')
ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, description = EXCLUDED.description;
