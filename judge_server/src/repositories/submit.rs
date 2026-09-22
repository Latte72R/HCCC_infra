use crate::entities::{JudgeResult, Submit};

/// A trait for database connection.
#[axum::async_trait]
pub trait Submits {
    /// Atomically claim up to `limit` Pending submits for this judge replica.
    ///
    /// Claimed rows are leased to `claimed_by` for `lease_secs` seconds so that
    /// other replicas skip them. If a replica crashes, the lease expires and
    /// another replica can pick the row up again.
    async fn claim_pending_submits(
        &self,
        limit: i64,
        claimed_by: &str,
        lease_secs: i32,
    ) -> Vec<Submit>;
    /// Store a judged result. Returns true when the row was still Pending and
    /// actually updated; false means the claim was lost (e.g. an admin already
    /// corrected the judgement).
    async fn store_result(
        &self,
        result: JudgeResult,
        error_message: String,
        submit_id: i32,
    ) -> bool;
}
