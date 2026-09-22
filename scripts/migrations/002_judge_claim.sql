-- Migration for existing databases created before the multi-replica judge support.
-- Safe to run multiple times. Also applied automatically by judge_server at startup.
ALTER TABLE submits ADD COLUMN IF NOT EXISTS claimed_at timestamptz;
ALTER TABLE submits ADD COLUMN IF NOT EXISTS claimed_by text;
CREATE INDEX IF NOT EXISTS idx_submits_pending_claim
    ON submits (result, claimed_at, time) WHERE result = 'Pending';
