//! Module for running an elf created by submittion
//! and judge its correctness with testcases.

use crate::arch::constants::EXEC_CMD;
use crate::ExitCode;
use serde::Deserialize;
use std::process::{Output, Stdio};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// The test runner times out at 2000ms.
const TLE_SEC: u64 = 2;

async fn execute_program(input: Option<&str>) -> Output {
    tokio::time::timeout(Duration::from_secs(TLE_SEC), execute_program_inner(input))
        .await
        .unwrap_or_else(|_| std::process::exit(ExitCode::TLE as i32))
}

async fn execute_program_inner(input: Option<&str>) -> Output {
    let mut parts = EXEC_CMD.split_whitespace();
    let executable = parts.next().expect("execution command missing");
    let mut command = Command::new(executable);
    command
        .args(parts)
        .kill_on_drop(true)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if input.is_some() {
        command.stdin(Stdio::piped());
    }
    let mut child = command
        .spawn()
        .unwrap_or_else(|_| std::process::exit(ExitCode::RE as i32));
    if let Some(input) = input {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(format!("{input}\n").as_bytes()).await;
        }
    }
    child
        .wait_with_output()
        .await
        .unwrap_or_else(|_| std::process::exit(ExitCode::RE as i32))
}

/// Testcase
#[derive(Deserialize)]
struct Testcase {
    #[allow(dead_code)]
    id: i32,
    /// Input of testcase.
    input: Option<String>,
    /// Expect result.
    expect: Option<String>,
}

/// Answer output
#[derive(Deserialize)]
pub enum TestTarget {
    /// Exit status of `$ bash -c ./test_target`.
    #[serde(rename = "exitcode")]
    ExitCode,
    /// Output from stdout.
    #[serde(rename = "stdout")]
    StdOut,
    /// No test case.
    #[serde(rename = "none")]
    NoTestCase,
}

/// Testcase deserialized from json file.
#[derive(Deserialize)]
pub struct Testcases {
    /// Test target (exit code or stdout).
    pub test_target: TestTarget,
    /// Testcases.
    tests: Vec<Testcase>,
}

impl Testcases {
    pub fn new(test_target: TestTarget, testcases: &str) -> Self {
        Testcases {
            test_target,
            tests: serde_json::from_str(testcases).unwrap(),
        }
    }
}

/// Just exec `test_target`.
/// If the file exited successfully, it will be `AC`.
pub async fn just_exec() {
    let output = execute_program(None).await;

    if !output.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        std::process::exit(ExitCode::RE as i32);
    }

    std::process::exit(ExitCode::AC as i32);
}

/// Judge with testcases.
pub async fn with_testcase(testcases: Testcases) {
    for case in testcases.tests {
        // exec and test
        let output = execute_program(Some(case.input.as_deref().unwrap_or(""))).await;

        match testcases.test_target {
            TestTarget::ExitCode => {
                if !output.stderr.is_empty() {
                    eprintln!("{}", std::str::from_utf8(&output.stderr).unwrap());
                    std::process::exit(ExitCode::RE as i32);
                }

                let exit_status = output.status.code().unwrap();
                let expect: i32 = case.expect.expect("no testcase expect").parse().unwrap();
                if exit_status != expect {
                    eprintln!("output: {exit_status:?}");
                    std::process::exit(ExitCode::WA as i32);
                }
            }
            TestTarget::StdOut => {
                if output.status.code().unwrap() != 0 {
                    eprintln!("{}", std::str::from_utf8(&output.stderr).unwrap());
                    std::process::exit(ExitCode::RE as i32);
                }

                let stdout = String::from_utf8_lossy(&output.stdout);
                let stdout = stdout.strip_prefix("bbl loader\r\n").unwrap_or(&stdout);
                if stdout != case.expect.expect("no testcase expect") {
                    eprintln!("output: {stdout:?}");
                    std::process::exit(ExitCode::WA as i32);
                }
            }
            TestTarget::NoTestCase => panic!("invalid ExeOption"),
        }
    }
}
