-- 修复默认管理员密码 hash
-- admin@redpacket.com / admin123
-- 使用正确的 bcrypt hash（之前在 migration 003 中使用了占位 hash）

UPDATE admin_users
SET password_hash = '$2b$10$5RWqxYHln0VMwAuohWyg0ejKzrI3S48Ykyawec/LGQg8lS0WkGmGO'
WHERE email = 'admin@redpacket.com';

-- 如果表中没有管理员（migration 003 可能因冲突未插入），则插入
INSERT INTO admin_users (email, password_hash, role, name)
SELECT 'admin@redpacket.com', '$2b$10$5RWqxYHln0VMwAuohWyg0ejKzrI3S48Ykyawec/LGQg8lS0WkGmGO', 'super_admin', 'Admin'
WHERE NOT EXISTS (SELECT 1 FROM admin_users WHERE email = 'admin@redpacket.com');
