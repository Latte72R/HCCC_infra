//! Set up for database connection.

use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use tokio_postgres::NoTls;

use crate::constants::database_url;
use crate::repos_impl::{ProblemImpl, SubmitImpl, TestcaseImpl};

/// Connection pool of postgres
pub type ConnectionPool = Pool<PostgresConnectionManager<NoTls>>;

#[derive(Clone)]
pub struct RepositoryProvider(ConnectionPool);

impl RepositoryProvider {
    /// Setup connection pool.
    ///
    /// # Panics
    /// It will panic when fail to build connection pool.
    #[must_use]
    pub async fn new() -> Self {
        let manager =
            PostgresConnectionManager::new_from_stringlike(database_url(), NoTls).unwrap();
        let pool = Pool::builder().build(manager).await.unwrap();

        let provider = RepositoryProvider(pool);
        provider.ensure_schema().await;
        provider
    }

    /// Bring pre-claim-era databases up to date (idempotent).
    /// Mirrors scripts/migrations/002_judge_claim.sql.
    async fn ensure_schema(&self) {
        let conn = self.0.get().await.unwrap();
        conn.batch_execute(
            "ALTER TABLE submits ADD COLUMN IF NOT EXISTS claimed_at timestamptz; \
             ALTER TABLE submits ADD COLUMN IF NOT EXISTS claimed_by text; \
             CREATE INDEX IF NOT EXISTS idx_submits_pending_claim \
             ON submits (result, claimed_at, time) WHERE result = 'Pending';",
        )
        .await
        .unwrap();
    }

    #[must_use]
    pub fn submit(&self) -> SubmitImpl<'_> {
        SubmitImpl { pool: &self.0 }
    }

    #[must_use]
    pub fn problem(&self) -> ProblemImpl<'_> {
        ProblemImpl { pool: &self.0 }
    }

    #[must_use]
    pub fn testcase(&self) -> TestcaseImpl<'_> {
        TestcaseImpl { pool: &self.0 }
    }
}
