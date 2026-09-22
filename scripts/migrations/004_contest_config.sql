-- Contest period config for existing databases (idempotent).
-- Rows are seeded from CONTEST_BEGIN/CONTEST_END at web startup when missing.
CREATE TABLE IF NOT EXISTS contest_config (
    key text primary key,
    value text not null
);
