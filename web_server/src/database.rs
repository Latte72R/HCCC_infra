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
}
