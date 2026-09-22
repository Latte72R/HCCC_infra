use crate::entities::Testcase;

/// A trait for database connection.
#[axum::async_trait]
pub trait Testcases {
    async fn get_all_testcases(
        &self,
        problem_ids: Vec<i32>,
    ) -> std::collections::HashMap<i32, Vec<Testcase>>;
}
