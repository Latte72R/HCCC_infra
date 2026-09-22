//! Set up for database connection.

use axum::Extension;
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use tokio_postgres::NoTls;

use crate::constants::database_url;
use crate::repos_impl::{AccountsImpl, ProblemImpl, SubmissionImpl, UserImpl};

/// Connection pool of postgres
pub type ConnectionPool = Pool<PostgresConnectionManager<NoTls>>;

/// Create new `RepositoryProvider`.
pub async fn layer() -> Extension<RepositoryProvider> {
    let manager = PostgresConnectionManager::new_from_stringlike(database_url(), NoTls).unwrap();
    let pool = Pool::builder().build(manager).await.unwrap();

    ensure_default_admin(&pool).await;
    ensure_contest_config(&pool).await;
    ensure_submit_arch(&pool).await;

    Extension(RepositoryProvider(pool))
}

/// Seed the default administrator (admin / P@ssw0rd) on pre-seed-era
/// databases. Idempotent; mirrors scripts/migrations/003_seed_admin.sql.
/// Password column stores hex(SHA256(password)).
async fn ensure_default_admin(pool: &ConnectionPool) {
    let conn = pool.get().await.unwrap();
    conn.batch_execute(
        "INSERT INTO accounts (id, name, password) VALUES \
         (1, 'admin', 'b03ddf3ca2e714a6548e7495e2a03f5e824eaac9837cd7f159c67b90fb4b7342') \
         ON CONFLICT (id) DO NOTHING; \
         SELECT setval('accounts_id_seq', (SELECT greatest(max(id), 1) FROM accounts));",
    )
    .await
    .unwrap();
    // Seed rows use explicit ids; keep serial sequences in sync so admin
    // problem creation (which omits id) never collides.
    conn.batch_execute(
        "SELECT setval('problems_id_seq', (SELECT greatest(max(id), 1) FROM problems)); \
         SELECT setval('testcases_id_seq', (SELECT greatest(max(id), 1) FROM testcases)); \
         SELECT setval('submits_id_seq', (SELECT greatest(max(id), 1) FROM submits));",
    )
    .await
    .unwrap();
}

#[derive(Clone)]
pub struct RepositoryProvider(ConnectionPool);

impl RepositoryProvider {
    pub fn accounts(&self) -> AccountsImpl<'_> {
        AccountsImpl { pool: &self.0 }
    }

    pub fn user(&self) -> UserImpl<'_> {
        UserImpl { pool: &self.0 }
    }

    pub fn problem(&self) -> ProblemImpl<'_> {
        ProblemImpl { pool: &self.0 }
    }

    pub fn submission(&self) -> SubmissionImpl<'_> {
        SubmissionImpl { pool: &self.0 }
    }

    /// Current contest period. Prefers the admin-editable DB config and falls
    /// back to the CONTEST_BEGIN/CONTEST_END environment on fresh errors.
    pub async fn contest_period(
        &self,
    ) -> (
        chrono::DateTime<chrono::Local>,
        chrono::DateTime<chrono::Local>,
    ) {
        contest_period_from_pool(&self.0).await
    }
}

/// Read the contest period from contest_config with env fallback.
pub async fn contest_period_from_pool(
    pool: &ConnectionPool,
) -> (
    chrono::DateTime<chrono::Local>,
    chrono::DateTime<chrono::Local>,
) {
    let conn = pool.get().await.unwrap();
    let read = async |key: &str| {
        conn.query_opt("SELECT value FROM contest_config WHERE key = $1", &[&key])
            .await
            .ok()
            .flatten()
            .map(|row| row.get::<_, String>("value"))
    };
    let (default_begin, default_end) = crate::constants::contest_duration();
    let begin = read("contest_begin")
        .await
        .and_then(|v| parse_period(&v))
        .unwrap_or(default_begin);
    let end = read("contest_end")
        .await
        .and_then(|v| parse_period(&v))
        .unwrap_or(default_end);
    (begin, end)
}

/// Move architecture choice from problems to submits (idempotent).
/// Mirrors scripts/migrations/005_submit_arch.sql.
async fn ensure_submit_arch(pool: &ConnectionPool) {
    let conn = pool.get().await.unwrap();
    conn.batch_execute(
        "ALTER TABLE submits ADD COLUMN IF NOT EXISTS arch Arch NOT NULL DEFAULT 'x8664'; \
         ALTER TABLE problems DROP COLUMN IF EXISTS arch;",
    )
    .await
    .unwrap();
}
/// Parse an RFC3339 timestamp stored in contest_config.
fn parse_period(value: &str) -> Option<chrono::DateTime<chrono::Local>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Local))
}

/// Create contest_config and seed it from the environment when empty.
/// Idempotent; mirrors scripts/migrations/004_contest_config.sql.
async fn ensure_contest_config(pool: &ConnectionPool) {
    let conn = pool.get().await.unwrap();
    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS contest_config (key text primary key, value text not null);",
    )
    .await
    .unwrap();
    let (begin, end) = crate::constants::contest_duration();
    conn.execute(
        "INSERT INTO contest_config (key, value) VALUES ('contest_begin', $1), ('contest_end', $2) ON CONFLICT (key) DO NOTHING",
        &[&begin.to_rfc3339(), &end.to_rfc3339()],
    )
    .await
    .unwrap();
}
