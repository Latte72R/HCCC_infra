use tokio_postgres::Row;

use crate::database::ConnectionPool;
use crate::entities::{Rank, User, UserObject};
use crate::repositories::Users;

/// Implementation for `Users`.
pub struct UserImpl<'a> {
    pub pool: &'a ConnectionPool,
}

#[axum::async_trait]
impl<'a> Users for UserImpl<'a> {
    /// Find a user by id.
    async fn find_user(&self, id: i32) -> Option<User> {
        let conn = self.pool.get().await.unwrap();
        let row = conn
            .query_opt("SELECT * FROM accounts WHERE id = $1", &[&id])
            .await
            .unwrap();

        row.map(std::convert::Into::into)
    }

    /// Get all users from database.
    async fn all_users(&self) -> Vec<UserObject> {
        let conn = self.pool.get().await.unwrap();
        let row = conn.query("SELECT * FROM accounts", &[]).await.unwrap();

        row.into_iter().map(std::convert::Into::into).collect()
    }

    /// Assemble a ranking from the current submission results.
    ///
    /// Scoring is derived on every request instead of being stored separately,
    /// so admin corrections, rejudges and deletions are reflected
    /// automatically. For each solved problem, only judged wrong attempts
    /// within the contest and strictly before the first current AC are
    /// penalized. Pending/SystemError are operational states and WC is kept
    /// non-penalizing for compatibility with the existing contest rules.
    async fn create_ranking(&self) -> Vec<Rank> {
        let (start, end) = crate::database::contest_period_from_pool(self.pool).await;
        let conn = self.pool.get().await.unwrap();
        let rows = conn
            .query(
                "WITH first_ac AS (
                    SELECT user_id, problem_id, MIN(time) AS first_ac_time
                    FROM submits
                    WHERE result = 'AC'
                      AND $1 <= time
                      AND time <= $2
                    GROUP BY user_id, problem_id
                ),
                problem_scores AS (
                    SELECT
                        fa.user_id,
                        fa.problem_id,
                        fa.first_ac_time,
                        GREATEST(
                            p.score::bigint - COUNT(*) FILTER (
                                WHERE s.time >= $1
                                  AND s.time < fa.first_ac_time
                                  AND s.result IN ('WA', 'AE', 'LE', 'RE', 'TLE')
                            ),
                            0
                        ) AS earned_score
                    FROM first_ac AS fa
                    JOIN problems AS p ON p.id = fa.problem_id
                    LEFT JOIN submits AS s
                      ON s.user_id = fa.user_id
                     AND s.problem_id = fa.problem_id
                    GROUP BY fa.user_id, fa.problem_id, fa.first_ac_time, p.score
                )
                SELECT
                    a.name AS name,
                    COALESCE(SUM(ps.earned_score), 0)::bigint AS score,
                    COALESCE(MAX(ps.first_ac_time), NOW()) AS max_time
                FROM accounts AS a
                LEFT JOIN problem_scores AS ps ON ps.user_id = a.id
                GROUP BY a.id, a.name
                ORDER BY a.name;",
                &[&start, &end],
            )
            .await
            .unwrap();

        let mut ranking = rows
            .into_iter()
            .map(|row| Rank::new(row.get("name"), row.get("score"), row.get("max_time")))
            .collect::<Vec<Rank>>();

        ranking.sort_by(|x, y| match (-x.score).cmp(&-y.score) {
            std::cmp::Ordering::Equal => x.time.cmp(&y.time),
            other => other,
        });

        ranking
            .into_iter()
            .enumerate()
            .map(|(rank, r)| r.set_rank(rank + 1))
            .collect()
    }

}

impl From<Row> for User {
    /// Convert SQL result to `User`.
    fn from(r: Row) -> Self {
        User::new("ok".to_string(), r.get("id"), r.get("name"), None)
    }
}

impl From<Row> for UserObject {
    /// Convert SQL result to `UserObject`.
    fn from(r: Row) -> Self {
        UserObject::new(r.get("id"), r.get("name"))
    }
}
