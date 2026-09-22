use tokio_postgres::Row;

use crate::database::ConnectionPool;
use crate::entities::{JudgeResult, Submit};
use crate::repositories::Submits;

/// A struct for database connection.
pub struct SubmitImpl<'a> {
    pub pool: &'a ConnectionPool,
}

#[axum::async_trait]
impl<'a> Submits for SubmitImpl<'a> {
    /// Claim Pending submits with SELECT ... FOR UPDATE SKIP LOCKED so that
    /// concurrent judge replicas never process the same row twice.
    async fn claim_pending_submits(
        &self,
        limit: i64,
        claimed_by: &str,
        lease_secs: i32,
    ) -> Vec<Submit> {
        let conn = self.pool.get().await.unwrap();
        conn.query(
            "UPDATE submits SET claimed_at = now(), claimed_by = $1 \
             WHERE id IN ( \
               SELECT id FROM submits \
               WHERE result = 'Pending' \
                 AND (claimed_at IS NULL OR claimed_at < now() - ($3::int * interval '1 second')) \
               ORDER BY time ASC \
               LIMIT $2 \
               FOR UPDATE SKIP LOCKED \
             ) \
             RETURNING *",
            &[&claimed_by, &limit, &lease_secs],
        )
        .await
        .unwrap()
        .into_iter()
        .map(std::convert::Into::into)
        .collect()
    }

    /// Store Judged result and release the claim.
    /// Only touches rows that are still Pending, so an admin correction that
    /// landed while judging is never overwritten.
    async fn store_result(
        &self,
        result: JudgeResult,
        error_message: String,
        submit_id: i32,
    ) -> bool {
        let conn = self.pool.get().await.unwrap();
        let updated = conn
            .execute(
                "UPDATE submits SET result = $1, error_message = $2, \
                 claimed_at = NULL, claimed_by = NULL \
                 WHERE id = $3 AND result = 'Pending'",
                &[&result, &error_message, &submit_id],
            )
            .await
            .unwrap();
        updated == 1
    }
}

impl From<Row> for Submit {
    /// Convert SQL output to `Submit`.
    fn from(r: Row) -> Self {
        Submit::new(
            r.get("id"),
            r.get("user_id"),
            r.get("problem_id"),
            r.get("time"),
            r.get("asm"),
            r.get("error_message"),
            r.get("is_ce"),
            r.get("error_line_number"),
            r.get("result"),
        )
    }
}
