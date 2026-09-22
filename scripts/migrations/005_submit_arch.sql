-- Move architecture choice from problems to submits (idempotent).
-- Submitters now choose arch at submit time; ranking counts distinct
-- solved problems, so solving in both arches adds no extra score.
ALTER TABLE submits ADD COLUMN IF NOT EXISTS arch Arch NOT NULL DEFAULT 'x8664';
ALTER TABLE problems DROP COLUMN IF EXISTS arch;
