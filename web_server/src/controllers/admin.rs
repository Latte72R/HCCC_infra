use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPool, Row};

use crate::{database::RepositoryProvider, is_admin_user, request::UserContext};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RecentSubmission {
    id: i32,
    user_name: String,
    problem_id: i32,
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContestPeriod {
    begin: String,
    end: String,
    event_name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminTestcase {
    id: i32,
    input: Option<String>,
    expect: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminProblem {
    id: i32,
    title: String,
    test_target: String,
    score: i32,
    is_wrong_code: bool,
    testcase_count: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminProblemDetail {
    id: i32,
    title: String,
    statement: String,
    code: String,
    input_desc: Option<String>,
    output_desc: Option<String>,
    test_target: String,
    score: i32,
    is_wrong_code: bool,
    error_line_number: Option<i32>,
    testcases: Vec<AdminTestcase>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestcaseInput {
    input: Option<String>,
    expect: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemInput {
    title: String,
    statement: String,
    code: String,
    input_desc: Option<String>,
    output_desc: Option<String>,
    test_target: String,
    score: i32,
    is_wrong_code: bool,
    error_line_number: Option<i32>,
    testcases: Vec<TestcaseInput>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemCreated {
    status: &'static str,
    id: i32,
}

fn validate_problem(input: &ProblemInput) -> bool {
    matches!(
        input.test_target.as_str(),
        "ExitCode" | "StdOut" | "NoTestCase"
    ) && !input.title.is_empty()
        && !input.statement.is_empty()
        && !input.code.is_empty()
        && input.title.len() <= 200
        && input.statement.len() <= 100_000
        && input.code.len() <= 100_000
        && input.score >= 0
        && input.testcases.len() <= 100
        && input
            .testcases
            .iter()
            .all(|t| t.input.as_ref().map_or(true, |v| v.len() <= 10_000))
        && input
            .testcases
            .iter()
            .all(|t| t.expect.as_ref().map_or(true, |v| v.len() <= 10_000))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContestPeriodUpdate {
    begin: String,
    end: String,
    event_name: String,
}

fn parse_rfc3339(value: &str) -> Option<chrono::DateTime<chrono::FixedOffset>> {
    chrono::DateTime::parse_from_rfc3339(value).ok()
}

/// Current contest period (admin only). Values are RFC3339 strings.
pub async fn get_contest_period(
    context: UserContext,
    Extension(repository_provider): Extension<RepositoryProvider>,
) -> Result<Json<ContestPeriod>, StatusCode> {
    if !is_admin_user(context.user_id()) {
        return Err(StatusCode::FORBIDDEN);
    }
    let (begin, end) = repository_provider.contest_period().await;
    let event_name = repository_provider.event_name().await;
    Ok(Json(ContestPeriod {
        begin: begin.to_rfc3339(),
        end: end.to_rfc3339(),
        event_name,
    }))
}

/// Update the contest period (admin only). Both values must be valid RFC3339
/// timestamps with begin strictly before end. Event name is shown on the top page.
pub async fn update_contest_period(
    context: UserContext,
    Extension(pool): Extension<PgPool>,
    Json(update): Json<ContestPeriodUpdate>,
) -> Result<Json<CorrectionResponse>, StatusCode> {
    if !is_admin_user(context.user_id()) {
        return Err(StatusCode::FORBIDDEN);
    }
    let (Some(begin), Some(end)) = (parse_rfc3339(&update.begin), parse_rfc3339(&update.end))
    else {
        return Err(StatusCode::BAD_REQUEST);
    };
    if begin >= end {
        return Err(StatusCode::BAD_REQUEST);
    }
    if update.event_name.is_empty() || update.event_name.len() > 200 {
        return Err(StatusCode::BAD_REQUEST);
    }
    sqlx::query(
        "INSERT INTO contest_config (key, value) VALUES ('contest_begin', $1), ('contest_end', $2), ('event_name', $3) \
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value",
    )
    .bind(begin.to_rfc3339())
    .bind(end.to_rfc3339())
    .bind(&update.event_name)
    .execute(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(CorrectionResponse { status: "ok" }))
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
        "SELECT s.id, a.name AS user_name, p.id AS problem_id, p.title AS problem_title, \
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
                problem_id: row.get("problem_id"),
                problem_title: row.get("problem_title"),
                result: row.get("result"),
                error_message: row.get("error_message"),
                submitted_at: row.get("submitted_at"),
            })
            .collect(),
    }))
}

/// List all problems with judge metadata (admin only).
pub async fn list_problems(
    context: UserContext,
    Extension(pool): Extension<PgPool>,
) -> Result<Json<Vec<AdminProblem>>, StatusCode> {
    if !is_admin_user(context.user_id()) {
        return Err(StatusCode::FORBIDDEN);
    }
    let rows = sqlx::query(
        "SELECT p.id, p.title, p.test_target::text AS test_target, \
         p.score, p.is_wrong_code, count(t.id) AS testcase_count \
         FROM problems p LEFT JOIN testcases t ON t.problem_id = p.id \
         GROUP BY p.id ORDER BY p.id",
    )
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        rows.into_iter()
            .map(|row| AdminProblem {
                id: row.get("id"),
                title: row.get("title"),
                test_target: row.get("test_target"),
                score: row.get("score"),
                is_wrong_code: row.get("is_wrong_code"),
                testcase_count: row.get("testcase_count"),
            })
            .collect(),
    ))
}

/// Full problem detail with testcases (admin only).
pub async fn get_problem(
    Path(id): Path<i32>,
    context: UserContext,
    Extension(pool): Extension<PgPool>,
) -> Result<Json<AdminProblemDetail>, StatusCode> {
    if !is_admin_user(context.user_id()) {
        return Err(StatusCode::FORBIDDEN);
    }
    let row = sqlx::query(
        "SELECT id, title, statement, code, input_desc, output_desc, \
         test_target::text AS test_target, score, \
         is_wrong_code, error_line_number FROM problems WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;
    let cases =
        sqlx::query("SELECT id, input, expect FROM testcases WHERE problem_id = $1 ORDER BY id")
            .bind(id)
            .fetch_all(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(AdminProblemDetail {
        id: row.get("id"),
        title: row.get("title"),
        statement: row.get("statement"),
        code: row.get("code"),
        input_desc: row.get("input_desc"),
        output_desc: row.get("output_desc"),
        test_target: row.get("test_target"),
        score: row.get("score"),
        is_wrong_code: row.get("is_wrong_code"),
        error_line_number: row.get("error_line_number"),
        testcases: cases
            .into_iter()
            .map(|c| AdminTestcase {
                id: c.get("id"),
                input: c.get("input"),
                expect: c.get("expect"),
            })
            .collect(),
    }))
}

/// Create a problem with testcases (admin only). Empty testcase lists are
/// stored as a single (NULL, NULL) row, matching the seed convention.
pub async fn create_problem(
    context: UserContext,
    Extension(pool): Extension<PgPool>,
    Json(input): Json<ProblemInput>,
) -> Result<Json<ProblemCreated>, StatusCode> {
    if !is_admin_user(context.user_id()) {
        return Err(StatusCode::FORBIDDEN);
    }
    if !validate_problem(&input) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut transaction = pool
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = sqlx::query(
        "INSERT INTO problems (title, statement, code, input_desc, output_desc, test_target, is_wrong_code, error_line_number, score) \
         VALUES ($1, $2, $3, $4, $5, $6::testtarget, $7, $8, $9) RETURNING id",
    )
    .bind(&input.title)
    .bind(&input.statement)
    .bind(&input.code)
    .bind(&input.input_desc)
    .bind(&input.output_desc)
    .bind(&input.test_target)
    .bind(input.is_wrong_code)
    .bind(input.error_line_number)
    .bind(input.score)
    .fetch_one(&mut transaction)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let problem_id: i32 = row.get("id");
    insert_testcases(&mut transaction, problem_id, &input.testcases).await?;
    transaction
        .commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(ProblemCreated {
        status: "ok",
        id: problem_id,
    }))
}

/// Replace a problem and its testcases (admin only).
pub async fn update_problem(
    Path(id): Path<i32>,
    context: UserContext,
    Extension(pool): Extension<PgPool>,
    Json(input): Json<ProblemInput>,
) -> Result<Json<CorrectionResponse>, StatusCode> {
    if !is_admin_user(context.user_id()) {
        return Err(StatusCode::FORBIDDEN);
    }
    if !validate_problem(&input) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut transaction = pool
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let updated = sqlx::query(
        "UPDATE problems SET title = $1, statement = $2, code = $3, input_desc = $4, output_desc = $5, \
         test_target = $6::testtarget, is_wrong_code = $7, error_line_number = $8, score = $9 \
         WHERE id = $10",
    )
    .bind(&input.title)
    .bind(&input.statement)
    .bind(&input.code)
    .bind(&input.input_desc)
    .bind(&input.output_desc)
    .bind(&input.test_target)
    .bind(input.is_wrong_code)
    .bind(input.error_line_number)
    .bind(input.score)
    .bind(id)
    .execute(&mut transaction)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if updated.rows_affected() != 1 {
        return Err(StatusCode::NOT_FOUND);
    }
    sqlx::query("DELETE FROM testcases WHERE problem_id = $1")
        .bind(id)
        .execute(&mut transaction)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    insert_testcases(&mut transaction, id, &input.testcases).await?;
    transaction
        .commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(CorrectionResponse { status: "ok" }))
}

/// Delete a problem. Submissions and testcases follow via ON DELETE CASCADE.
/// Submits already judged keep no copy, so deletion is irreversible.
pub async fn delete_problem(
    Path(id): Path<i32>,
    context: UserContext,
    Extension(pool): Extension<PgPool>,
) -> Result<Json<CorrectionResponse>, StatusCode> {
    if !is_admin_user(context.user_id()) {
        return Err(StatusCode::FORBIDDEN);
    }
    let deleted = sqlx::query("DELETE FROM problems WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if deleted.rows_affected() != 1 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(Json(CorrectionResponse { status: "ok" }))
}

async fn insert_testcases(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    problem_id: i32,
    testcases: &[TestcaseInput],
) -> Result<(), StatusCode> {
    if testcases.is_empty() {
        sqlx::query("INSERT INTO testcases (problem_id, input, expect) VALUES ($1, NULL, NULL)")
            .bind(problem_id)
            .execute(&mut *transaction)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        return Ok(());
    }
    for case in testcases {
        sqlx::query("INSERT INTO testcases (problem_id, input, expect) VALUES ($1, $2, $3)")
            .bind(problem_id)
            .bind(&case.input)
            .bind(&case.expect)
            .execute(&mut *transaction)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    Ok(())
}
