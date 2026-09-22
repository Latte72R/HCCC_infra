use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPool, Row};

use crate::{is_admin_user, request::UserContext};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RecentSubmission {
    id: i32,
    user_name: String,
    problem_title: String,
    result: String,
    error_message: String,
    submitted_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    users: i64,
    problems: i64,
    submissions: i64,
    pending: i64,
    accepted: i64,
    recent_submissions: Vec<RecentSubmission>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JudgementCorrection {
    result: String,
    error_message: String,
}

#[derive(Serialize)]
pub struct CorrectionResponse {
    status: &'static str,
}

/// Update one result and retain the previous value for operator review.
pub async fn correct_judgement(
    Path(id): Path<i32>,
    context: UserContext,
    Extension(pool): Extension<PgPool>,
    Json(correction): Json<JudgementCorrection>,
) -> Result<Json<CorrectionResponse>, StatusCode> {
    if !is_admin_user(context.user_id()) {
        return Err(StatusCode::FORBIDDEN);
    }
    if !matches!(
        correction.result.as_str(),
        "AC" | "WA" | "WC" | "AE" | "LE" | "RE" | "TLE" | "Pending" | "SystemError"
    ) || correction.error_message.len() > 10_000
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    sqlx::query("CREATE TABLE IF NOT EXISTS admin_judge_audit (id bigserial PRIMARY KEY, submission_id integer NOT NULL REFERENCES submits(id), admin_user_id integer NOT NULL REFERENCES accounts(id), previous_result text NOT NULL, previous_error_message text NOT NULL, new_result text NOT NULL, new_error_message text NOT NULL, changed_at timestamptz NOT NULL DEFAULT now())")
        .execute(&pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut transaction = pool
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let previous = sqlx::query(
        "SELECT result::text AS result, error_message FROM submits WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut transaction)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;
    let previous_result: String = previous.get("result");
    let previous_error: String = previous.get("error_message");
    sqlx::query("UPDATE submits SET result = $1::judgeresult, error_message = $2 WHERE id = $3")
        .bind(&correction.result)
        .bind(&correction.error_message)
        .bind(id)
        .execute(&mut transaction)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    sqlx::query("INSERT INTO admin_judge_audit (submission_id, admin_user_id, previous_result, previous_error_message, new_result, new_error_message) VALUES ($1, $2, $3, $4, $5, $6)")
        .bind(id).bind(context.user_id()).bind(previous_result).bind(previous_error)
        .bind(&correction.result).bind(&correction.error_message)
        .execute(&mut transaction).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    transaction
        .commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(CorrectionResponse { status: "ok" }))
}

/// A small read-only operations view. No source code or password data is returned.
pub async fn overview(
    context: UserContext,
    Extension(pool): Extension<PgPool>,
) -> Result<Json<Overview>, StatusCode> {
    if !is_admin_user(context.user_id()) {
        return Err(StatusCode::FORBIDDEN);
    }

    let counts = sqlx::query(
        "SELECT (SELECT count(*) FROM accounts) AS users, \
         (SELECT count(*) FROM problems) AS problems, \
         (SELECT count(*) FROM submits) AS submissions, \
         (SELECT count(*) FROM submits WHERE result = 'Pending') AS pending, \
         (SELECT count(*) FROM submits WHERE result = 'AC') AS accepted",
    )
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let rows = sqlx::query(
        "SELECT s.id, a.name AS user_name, p.title AS problem_title, \
         s.result::text AS result, s.error_message, s.time AS submitted_at \
         FROM submits s JOIN accounts a ON a.id = s.user_id \
         JOIN problems p ON p.id = s.problem_id ORDER BY s.time DESC LIMIT 20",
    )
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(Overview {
        users: counts.get("users"),
        problems: counts.get("problems"),
        submissions: counts.get("submissions"),
        pending: counts.get("pending"),
        accepted: counts.get("accepted"),
        recent_submissions: rows
            .into_iter()
            .map(|row| RecentSubmission {
                id: row.get("id"),
                user_name: row.get("user_name"),
                problem_title: row.get("problem_title"),
                result: row.get("result"),
                error_message: row.get("error_message"),
                submitted_at: row.get("submitted_at"),
            })
            .collect(),
    }))
}
