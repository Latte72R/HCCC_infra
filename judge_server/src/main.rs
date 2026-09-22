//! This crate is the server that judges submissions.  
//! It monitors the database tables, retrieves Pending submissions,
//! judges them using `test_runner`, and stores the results in the database.

use judge_server::database::RepositoryProvider;
use judge_server::entities::{Arch, JudgeResult, Problem, Submit, Testcase};
use judge_server::repositories::{Problems, Submits, Testcases};
use std::collections::HashMap;
use tokio::process::Command;
use tokio::time::{sleep, Duration};

/// Judge the submission using `test_runner`.
async fn judge(
    submit: &Submit,
    problem: &Problem,
    testcase: &Vec<Testcase>,
) -> (JudgeResult, Option<String>, i32) {
    if submit.is_ce {
        if problem.is_wrong_code && submit.error_line_number == problem.error_line_number {
            return (JudgeResult::AC, None, submit.id());
        } else {
            return (
                JudgeResult::WC,
                Some("Wrong Compile Error".to_string()),
                submit.id(),
            );
        }
    }

    let docker_container = match submit.arch {
        Arch::x8664 => std::env::var("HCCC_RUNNER_IMAGE_X8664").unwrap_or_else(|_| {
            "ghcr.io/humanccompilercontest/hccc_infra:test_runner_x8664-develop".to_string()
        }),
        Arch::riscv => std::env::var("HCCC_RUNNER_IMAGE_RISCV").unwrap_or_else(|_| {
            "ghcr.io/humanccompilercontest/hccc_infra:test_runner_riscv-develop".to_string()
        }),
    };

    let custom_runner = std::env::var("HCCC_RUNNER_EXECUTABLE").ok();
    let mut command = if let Some(ref executable) = custom_runner {
        Command::new(executable)
    } else {
        let mut command = Command::new("docker");
        command.args([
            "run",
            "--rm",
            "--network=none",
            "--pids-limit=64",
            "--cap-drop=ALL",
            "--security-opt=no-new-privileges",
            "--memory=128M",
            "--cpus=0.05",
        ]);
        command
    };
    let result = command
        .arg(docker_container)
        .arg(base64::encode(&submit.asm))
        .arg(problem.test_target.as_arg())
        .arg(base64::encode(
            serde_json::to_string(&testcase).expect("serialization failed"),
        ))
        .output()
        .await;

    if let Ok(result) = result {
        let judge_result = match result.status.code().unwrap_or(7) {
            0 => JudgeResult::AC,
            1 => JudgeResult::WA,
            2 => JudgeResult::WC,
            3 => JudgeResult::AE,
            4 => JudgeResult::LE,
            5 => JudgeResult::RE,
            6 => JudgeResult::TLE,
            _ => JudgeResult::SystemError,
        };

        // submit Compile Error to correct code
        if problem.is_wrong_code && !submit.is_ce && JudgeResult::AC == judge_result {
            return (JudgeResult::WA, None, submit.id());
        }

        let error_message = match judge_result {
            JudgeResult::AC => None,
            JudgeResult::SystemError => {
                tracing::debug!(
                    "SystemError: {}",
                    std::str::from_utf8(&result.stderr).unwrap()
                );
                Some("System error: Please contact the competition management.".to_string())
            }
            // get error message from stderr
            _ => std::str::from_utf8(&result.stderr)
                .map(std::string::ToString::to_string)
                .ok(),
        };

        (judge_result, error_message, submit.id())
    } else {
        (
            JudgeResult::SystemError,
            Some("Unknown Result.".to_string()),
            submit.id(),
        )
    }
}

/// Main function.
#[tokio::main]
async fn main() {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "judge_server=debug");
    }
    tracing_subscriber::fmt::init();

    let instance_id = std::env::var("JUDGE_INSTANCE_ID").unwrap_or_else(|_| {
        std::env::var("HOSTNAME")
            .map(|host| format!("{host}:{}", std::process::id()))
            .unwrap_or_else(|_| format!("pid:{}", std::process::id()))
    });
    let claim_limit: i64 = std::env::var("JUDGE_CLAIM_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4);
    let lease_secs: i32 = std::env::var("JUDGE_CLAIM_LEASE_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300);
    let poll = Duration::from_millis(
        std::env::var("JUDGE_POLL_MILLIS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5000),
    );
    tracing::info!("judge {instance_id} starting (claim_limit={claim_limit}, lease={lease_secs}s)");

    let repo = RepositoryProvider::new().await;
    let repo_submit = repo.submit();

    // Problem/testcase cache with periodic refresh so long-running replicas
    // (and K8s rollouts) pick up newly added problems without a restart.
    let mut problems: HashMap<i32, Problem> = HashMap::new();
    let mut testcases: HashMap<i32, Vec<Testcase>> = HashMap::new();
    let mut last_refresh: Option<std::time::Instant> = None;

    loop {
        let stale = last_refresh
            .map(|at| at.elapsed() > Duration::from_secs(30))
            .unwrap_or(true);
        if stale || problems.is_empty() {
            problems = repo
                .problem()
                .get_all_problems()
                .await
                .into_iter()
                .map(|problem| (problem.id(), problem))
                .collect();
            testcases = repo
                .testcase()
                .get_all_testcases(problems.keys().copied().collect())
                .await;
            last_refresh = Some(std::time::Instant::now());
        }

        let submits = repo_submit
            .claim_pending_submits(claim_limit, &instance_id, lease_secs)
            .await;
        if submits.is_empty() {
            sleep(poll).await;
            continue;
        }
        tracing::info!("judge {instance_id} claimed {} submit(s)", submits.len());

        for submit in &submits {
            let Some(problem) = problems.get(&submit.problem_id()) else {
                // Problem may have been added after the last refresh; reload once.
                tracing::warn!(
                    "Unknown problem id {}, refreshing cache",
                    submit.problem_id()
                );
                problems = repo
                    .problem()
                    .get_all_problems()
                    .await
                    .into_iter()
                    .map(|problem| (problem.id(), problem))
                    .collect();
                testcases = repo
                    .testcase()
                    .get_all_testcases(problems.keys().copied().collect())
                    .await;
                last_refresh = Some(std::time::Instant::now());
                let Some(problem) = problems.get(&submit.problem_id()) else {
                    tracing::error!("Unknown problem id {}", submit.problem_id());
                    store_with_retry(
                        &repo_submit,
                        JudgeResult::SystemError,
                        "Unknown problem".to_string(),
                        submit.id(),
                    )
                    .await;
                    continue;
                };
                judge_and_store(&repo_submit, submit, problem, &testcases).await;
                continue;
            };
            judge_and_store(&repo_submit, submit, problem, &testcases).await;
        }
    }
}

/// Judge one submit and persist the result, retrying transient DB errors.
/// A false return from store means the claim was lost (admin correction or
/// lease expiry with re-judge by another replica); the result is logged and
/// the row is left untouched.
async fn judge_and_store(
    repo_submit: &impl Submits,
    submit: &Submit,
    problem: &Problem,
    testcases: &HashMap<i32, Vec<Testcase>>,
) {
    let Some(testcase) = testcases.get(&submit.problem_id()) else {
        tracing::error!("Missing testcases for problem {}", submit.problem_id());
        store_with_retry(
            repo_submit,
            JudgeResult::SystemError,
            "Missing testcases".to_string(),
            submit.id(),
        )
        .await;
        return;
    };
    let ret = judge(submit, problem, testcase).await;
    let (judge_result, error_message, submit_id) = ret;
    if !store_with_retry(
        repo_submit,
        judge_result,
        error_message.unwrap_or_default(),
        submit_id,
    )
    .await
    {
        tracing::warn!("submit {submit_id}: claim lost, result not stored");
    }
}

async fn store_with_retry(
    repo_submit: &impl Submits,
    result: JudgeResult,
    error_message: String,
    submit_id: i32,
) -> bool {
    for attempt in 1..=3 {
        let stored = tokio::time::timeout(
            Duration::from_secs(10),
            repo_submit.store_result(result, error_message.clone(), submit_id),
        )
        .await;
        match stored {
            Ok(done) => return done,
            Err(_) => {
                tracing::warn!("submit {submit_id}: store attempt {attempt} timed out");
                sleep(Duration::from_secs(2)).await;
            }
        }
    }
    tracing::error!("submit {submit_id}: giving up store, lease will expire for retry");
    false
}
