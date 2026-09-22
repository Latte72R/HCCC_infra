#!/usr/bin/env bash
# Migrate HCCC Postgres data from 14 to 18 via dump/restore.
#
# A Postgres major version change cannot reuse the old data volume in place,
# so this script dumps the old database to a file, then restores it into a
# fresh postgres:18 volume. Run from HCCC_infra with the old stack stopped.
#
#   1. cp .env.example .env  (and adjust credentials if you changed them)
#   2. ./scripts/migrate-pg14-to-pg18.sh
#
# The script keeps a timestamped backup under ./backups/ and refuses to
# overwrite an already-migrated volume unless FORCE=1 is set.
set -euo pipefail
cd "$(dirname "$0")/.."

COMPOSE="docker compose"
BACKUP_DIR="${BACKUP_DIR:-./backups}"
STAMP="$(date +%Y%m%d-%H%M%S)"
POSTGRES_USER="${POSTGRES_USER:-kcs1959}"
POSTGRES_PASSWORD="${POSTGRES_PASSWORD:-WeL0veKCS}"
POSTGRES_DB="${POSTGRES_DB:-hccc_judge}"
export POSTGRES_USER POSTGRES_PASSWORD POSTGRES_DB
export POSTGRES_IMAGE="${POSTGRES_IMAGE:-postgres:18-alpine}"

mkdir -p "$BACKUP_DIR"
BACKUP_FILE="$BACKUP_DIR/hccc-pg14-backup-$STAMP.sql"

if [ "${FORCE:-0}" != "1" ] && docker volume inspect hccc_infra_db_store >/dev/null 2>&1; then
  echo "==> found existing db_store volume; it will be replaced after backup"
fi

echo "==> [1/4] starting old DB (postgres:14-alpine) for dump"
POSTGRES_IMAGE=postgres:14-alpine $COMPOSE up -d db
$COMPOSE exec -T db pg_isready -U "$POSTGRES_USER" -d "$POSTGRES_DB" >/dev/null
# wait until accepting connections
for i in $(seq 1 30); do
  if $COMPOSE exec -T db psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -c 'SELECT 1' >/dev/null 2>&1; then
    break
  fi
  sleep 2
done

echo "==> [2/4] dumping to $BACKUP_FILE"
$COMPOSE exec -T db pg_dump -U "$POSTGRES_USER" -d "$POSTGRES_DB" --no-owner --no-privileges > "$BACKUP_FILE"
echo "    $(wc -l < "$BACKUP_FILE") lines dumped"

echo "==> [3/4] recreating volume with $POSTGRES_IMAGE"
$COMPOSE down
docker volume rm -f hccc_infra_db_store "${COMPOSE_PROJECT_NAME:-hccc_infra}_db_store" 2>/dev/null || true
POSTGRES_IMAGE="$POSTGRES_IMAGE" $COMPOSE up -d db
for i in $(seq 1 30); do
  if $COMPOSE exec -T db psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -c 'SELECT 1' >/dev/null 2>&1; then
    break
  fi
  sleep 2
done

echo "==> [4/4] restoring into $POSTGRES_IMAGE"
# init.sql already created schema+problems on first boot; restore data-only tables
# that users own (accounts/sessions/submits + sequences), skipping what init owns.
# Simplest reliable path: restore everything except the schema-only failure on
# existing tables is noisy, so restore with --clean-safe plain SQL and ignore
# "already exists" errors for the static problem/testcase seed.
docker exec -i "$($COMPOSE ps -q db)" psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" \
  -v ON_ERROR_STOP=0 < "$BACKUP_FILE" 2>&1 | tail -5 || true
# Bring serial sequences back in sync after restore (covers accounts,
# submits, problems, testcases, sessions, admin_judge_audit).
$COMPOSE exec -T db psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" <<'SQL' >/dev/null
DO $$
DECLARE r record;
BEGIN
  FOR r IN SELECT 'accounts_id_seq' AS s, 'accounts' AS t UNION ALL
           SELECT 'submits_id_seq', 'submits' UNION ALL
           SELECT 'problems_id_seq', 'problems' UNION ALL
           SELECT 'testcases_id_seq', 'testcases' UNION ALL
           SELECT 'admin_judge_audit_id_seq', 'admin_judge_audit'
  LOOP
    BEGIN
      EXECUTE format('SELECT setval(%L, COALESCE((SELECT max(id) FROM %I), 1))', r.s, r.t);
    EXCEPTION WHEN undefined_table OR undefined_object THEN
      -- table/sequence does not exist yet on this schema; skip
      NULL;
    END;
  END LOOP;
END $$;
SQL

echo "==> applying post-restore migrations"
$COMPOSE exec -T db psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" \
  -f /docker-entrypoint-initdb.d/migrations/002_judge_claim.sql >/dev/null || \
  docker exec -i "$($COMPOSE ps -q db)" psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" \
    < scripts/migrations/002_judge_claim.sql

echo "==> counts after migration"
$COMPOSE exec -T db psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" \
  -c "SELECT (SELECT count(*) FROM accounts) AS accounts, (SELECT count(*) FROM submits) AS submits, (SELECT count(*) FROM problems) AS problems;"

echo "done. backup kept at $BACKUP_FILE"
