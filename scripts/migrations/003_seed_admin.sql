-- Seed the default administrator on existing databases (idempotent).
-- Default credentials: admin / P@ssw0rd. Password column is hex(SHA256(password)).
-- NOTE: if id 1 is already taken by another account, pick a free id and set
-- ADMIN_USER_IDS to it instead.
INSERT INTO accounts (id, name, password) VALUES (
    1,
    'admin',
    'b03ddf3ca2e714a6548e7495e2a03f5e824eaac9837cd7f159c67b90fb4b7342'
) ON CONFLICT (id) DO NOTHING;
SELECT setval('accounts_id_seq', (SELECT greatest(max(id), 1) FROM accounts));
