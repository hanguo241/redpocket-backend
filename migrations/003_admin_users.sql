-- 管理员表
CREATE TABLE IF NOT EXISTS admin_users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    role VARCHAR(16) NOT NULL DEFAULT 'operator',  -- super_admin | operator | readonly
    name VARCHAR(128) NOT NULL DEFAULT '',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 默认超级管理员 (密码: admin123)
-- 密码为 bcrypt hash，实际部署务必修改
INSERT INTO admin_users (email, password_hash, role, name)
VALUES ('admin@redpacket.com', '$2b$12$LJ3m4ys3Lk0TSwHnbfOMiOXPm1Qlq5GzGq5Yq5Yq5Yq5Yq5Yq5Yq', 'super_admin', 'Admin')
ON CONFLICT (email) DO NOTHING;
