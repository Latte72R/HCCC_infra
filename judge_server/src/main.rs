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

    let docker_container = match problem.arch {
        Arch::x8664 => "ghcr.io/humanccompilercontest/hccc_infra:test_runner_x8664-develop",
        Arch::riscv => "ghcr.io/humanccompilercontest/hccc_infra:test_runner_riscv-develop",
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

    let repo = RepositoryProvider::new().await;
    let repo_submit = repo.submit();
    let problems: HashMap<_, _> = repo
        .problem()
        .get_all_problems()
        .await
        .into_iter()
        .map(|problem| (problem.id(), problem))
        .collect();
    let testcases: HashMap<_, _> = repo
        .testcase()
        .get_all_testcases(problems.keys().copied().collect())
        .await;

    loop {
        let submits = repo_submit.get_pending_submits().await;
        for submit in &submits {
            let Some(problem) = problems.get(&submit.problem_id()) else {
                tracing::error!("Unknown problem id {}", submit.problem_id());
                repo_submit
                    .store_result(
                        JudgeResult::SystemError,
                        "Unknown problem".to_string(),
                        submit.id(),
                    )
                    .await;
                continue;
            };
            let Some(testcase) = testcases.get(&submit.problem_id()) else {
                tracing::error!("Missing testcases for problem {}", submit.problem_id());
                repo_submit
                    .store_result(
                        JudgeResult::SystemError,
                        "Missing testcases".to_string(),
                        submit.id(),
                    )
                    .await;
                continue;
            };
            let ret = judge(submit, problem, testcase).await;
            let (judge_result, error_message, submit_id) = ret;
            repo_submit
                .store_result(
                    judge_result,
                    error_message.unwrap_or(String::new()),
                    submit_id,
                )
                .await;
        }
        sleep(Duration::from_millis(5000)).await;
    }
}
