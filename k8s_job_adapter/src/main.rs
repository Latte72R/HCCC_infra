//! Kubernetes Job adapter for HCCC judging.
//!
//! Implements the `HCCC_RUNNER_EXECUTABLE` contract so `judge_server` can run
//! `test_runner` on Kubernetes instead of the local Docker socket:
//!
//! ```text
//! hccc-k8s-job-runner <image> <base64-asm> <exitcode|stdout|none> <base64-testcases-json>
//! ```
//!
//! The adapter creates one Job per invocation, waits for completion, replays
//! the container log on its own stderr, deletes the Job (TTL is set as a
//! backup), and exits with the same exit code `test_runner` produced, so the
//! judge maps it to `JudgeResult` exactly like the Docker path.
//!
//! Isolation notes:
//! - each Job pod drops ALL capabilities, forbids privilege escalation and
//!   uses the RuntimeDefault seccomp profile, without a service account token
//!   or service env links;
//! - network isolation is enforced with the `NetworkPolicy` in `k8s/`
//!   (deny all ingress/egress for Job pods), mirroring `--network=none`.

use k8s_openapi::api::batch::v1::{Job, JobSpec};
use k8s_openapi::api::core::v1::{
    Capabilities, Container, Pod, PodSecurityContext, PodSpec, PodTemplateSpec,
    ResourceRequirements, SeccompProfile, SecurityContext,
};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::api::{DeleteParams, ListParams, LogParams, PostParams};
use kube::{Api, Client};
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Exit code used when the adapter itself fails (maps to `SystemError`).
const ADAPTER_FAILURE_CODE: i32 = 7;

#[derive(Debug)]
struct Config {
    namespace: String,
    service_account: Option<String>,
    ttl_secs: i32,
    active_deadline_secs: i64,
    timeout_secs: u64,
    poll_secs: u64,
    cpu: String,
    memory: String,
    image_pull_policy: String,
}

impl Config {
    fn from_env() -> Self {
        Self {
            namespace: std::env::var("HCCC_JOB_NAMESPACE")
                .ok()
                .filter(|v| !v.is_empty())
                .or_else(in_cluster_namespace)
                .unwrap_or_else(|| "default".to_string()),
            service_account: std::env::var("HCCC_JOB_SERVICE_ACCOUNT")
                .ok()
                .filter(|v| !v.is_empty()),
            ttl_secs: env_int("HCCC_JOB_TTL_SECONDS", 120),
            active_deadline_secs: env_int("HCCC_JOB_ACTIVE_DEADLINE_SECS", 180),
            timeout_secs: env_int("HCCC_JOB_TIMEOUT_SECS", 180) as u64,
            poll_secs: env_int("HCCC_JOB_POLL_SECS", 2) as u64,
            cpu: std::env::var("HCCC_JOB_CPU").unwrap_or_else(|_| "500m".to_string()),
            memory: std::env::var("HCCC_JOB_MEMORY").unwrap_or_else(|_| "256Mi".to_string()),
            image_pull_policy: std::env::var("HCCC_JOB_IMAGE_PULL_POLICY")
                .unwrap_or_else(|_| "IfNotPresent".to_string()),
        }
    }
}

fn env_int<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn in_cluster_namespace() -> Option<String> {
    std::fs::read_to_string("/var/run/secrets/kubernetes.io/serviceaccount/namespace")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Build a unique, DNS-1123-safe Job name.
fn job_name() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!(
        "hccc-judge-{}-{}",
        millis % 100_000_000_000,
        std::process::id() % 100_000
    )
}

fn seccomp_runtime_default() -> SeccompProfile {
    SeccompProfile {
        type_: "RuntimeDefault".to_string(),
        ..Default::default()
    }
}

fn quantities(cpu: &str, memory: &str) -> BTreeMap<String, Quantity> {
    BTreeMap::from([
        ("cpu".to_string(), Quantity(cpu.to_string())),
        ("memory".to_string(), Quantity(memory.to_string())),
    ])
}

fn build_job(name: &str, image: &str, args: &[String], config: &Config) -> Job {
    let limits = quantities(&config.cpu, &config.memory);
    let container = Container {
        name: "runner".to_string(),
        image: Some(image.to_string()),
        image_pull_policy: Some(config.image_pull_policy.clone()),
        args: Some(args.to_vec()),
        resources: Some(ResourceRequirements {
            limits: Some(limits.clone()),
            requests: Some(limits),
            ..Default::default()
        }),
        security_context: Some(SecurityContext {
            allow_privilege_escalation: Some(false),
            capabilities: Some(Capabilities {
                drop: Some(vec!["ALL".to_string()]),
                ..Default::default()
            }),
            seccomp_profile: Some(seccomp_runtime_default()),
            ..Default::default()
        }),
        ..Default::default()
    };
    Job {
        metadata: ObjectMeta {
            name: Some(name.to_string()),
            labels: Some(BTreeMap::from([
                (
                    "app.kubernetes.io/name".to_string(),
                    "hccc-judge".to_string(),
                ),
                (
                    "app.kubernetes.io/component".to_string(),
                    "test-runner".to_string(),
                ),
            ])),
            ..Default::default()
        },
        spec: Some(JobSpec {
            backoff_limit: Some(0),
            ttl_seconds_after_finished: Some(config.ttl_secs),
            active_deadline_seconds: Some(config.active_deadline_secs),
            template: PodTemplateSpec {
                metadata: Some(ObjectMeta {
                    labels: Some(BTreeMap::from([(
                        "app.kubernetes.io/name".to_string(),
                        "hccc-judge".to_string(),
                    )])),
                    ..Default::default()
                }),
                spec: Some(PodSpec {
                    restart_policy: Some("Never".to_string()),
                    automount_service_account_token: Some(false),
                    enable_service_links: Some(false),
                    service_account_name: config.service_account.clone(),
                    security_context: Some(PodSecurityContext {
                        seccomp_profile: Some(seccomp_runtime_default()),
                        ..Default::default()
                    }),
                    containers: vec![container],
                    ..Default::default()
                }),
            },
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn job_finished(status: &k8s_openapi::api::batch::v1::JobStatus) -> bool {
    let complete = status
        .conditions
        .as_ref()
        .map(|conds| {
            conds
                .iter()
                .any(|c| (c.type_ == "Complete" || c.type_ == "Failed") && c.status == "True")
        })
        .unwrap_or(false);
    complete || status.succeeded.unwrap_or(0) > 0 || status.failed.unwrap_or(0) > 0
}

#[tokio::main]
async fn main() {
    let argv: Vec<String> = std::env::args().collect();
    if argv.len() != 5 {
        eprintln!(
            "usage: {} <image> <base64-asm> <exitcode|stdout|none> <base64-testcases>",
            argv[0]
        );
        std::process::exit(ADAPTER_FAILURE_CODE);
    }
    let (image, asm_b64, test_target, testcases_b64) = (
        argv[1].clone(),
        argv[2].clone(),
        argv[3].clone(),
        argv[4].clone(),
    );
    if !matches!(test_target.as_str(), "exitcode" | "stdout" | "none") {
        eprintln!("invalid test target: {test_target}");
        std::process::exit(ADAPTER_FAILURE_CODE);
    }

    if let Err(message) = run(image, asm_b64, test_target, testcases_b64).await {
        eprintln!("hccc-k8s-job-runner: {message}");
        std::process::exit(ADAPTER_FAILURE_CODE);
    }
}

async fn run(
    image: String,
    asm_b64: String,
    test_target: String,
    testcases_b64: String,
) -> Result<(), String> {
    let config = Config::from_env();
    let client = Client::try_default()
        .await
        .map_err(|e| format!("cannot build cluster client: {e}"))?;
    let jobs: Api<Job> = Api::namespaced(client.clone(), &config.namespace);
    let pods: Api<Pod> = Api::namespaced(client, &config.namespace);

    let name = job_name();
    let job = build_job(
        &name,
        &image,
        &[asm_b64, test_target, testcases_b64],
        &config,
    );
    jobs.create(&PostParams::default(), &job)
        .await
        .map_err(|e| format!("cannot create Job {name}: {e}"))?;

    let deadline = std::time::Instant::now() + Duration::from_secs(config.timeout_secs);
    loop {
        if std::time::Instant::now() > deadline {
            let _ = jobs.delete(&name, &DeleteParams::default()).await;
            return Err(format!(
                "Job {name} did not finish within {}s",
                config.timeout_secs
            ));
        }
        let status = jobs
            .get_status(&name)
            .await
            .map_err(|e| format!("cannot get Job {name} status: {e}"))?;
        if let Some(status) = status.status {
            if job_finished(&status) {
                return finish(&jobs, &pods, &name).await;
            }
        }
        tokio::time::sleep(Duration::from_secs(config.poll_secs)).await;
    }
}

/// Fetch the runner exit code and logs, replay logs on stderr, delete the Job,
/// and exit with the runner's code.
async fn finish(jobs: &Api<Job>, pods: &Api<Pod>, name: &str) -> Result<(), String> {
    let selector = format!("job-name={name}");
    let pod_list = pods
        .list(&ListParams::default().labels(&selector))
        .await
        .map_err(|e| format!("cannot list Job {name} pods: {e}"))?;
    let pod = pod_list
        .items
        .iter()
        .find(|p| {
            p.status
                .as_ref()
                .and_then(|s| s.phase.as_ref())
                .map(|phase| phase == "Succeeded" || phase == "Failed")
                .unwrap_or(false)
        })
        .or(pod_list.items.first())
        .ok_or_else(|| format!("Job {name} finished without pods"))?;
    let pod_name = pod.metadata.name.clone().unwrap_or_default();

    let exit_code = pod
        .status
        .as_ref()
        .and_then(|s| s.container_statuses.as_ref())
        .and_then(|statuses| statuses.first())
        .and_then(|c| c.state.as_ref())
        .and_then(|state| state.terminated.as_ref())
        .map(|t| t.exit_code)
        .unwrap_or(ADAPTER_FAILURE_CODE);

    match pods
        .logs(
            &pod_name,
            &LogParams {
                container: Some("runner".to_string()),
                ..Default::default()
            },
        )
        .await
    {
        Ok(logs) => eprint!("{logs}"),
        Err(e) => eprintln!("hccc-k8s-job-runner: cannot fetch logs of {pod_name}: {e}"),
    }

    if let Err(e) = jobs.delete(name, &DeleteParams::default()).await {
        eprintln!("hccc-k8s-job-runner: cannot delete Job {name}: {e}");
    }
    std::process::exit(exit_code);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_name_is_dns_safe_and_bounded() {
        for _ in 0..100 {
            let name = job_name();
            assert!(name.len() <= 63, "{name}");
            assert!(name.starts_with("hccc-judge-"), "{name}");
            assert!(
                name.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{name}"
            );
        }
    }

    #[test]
    fn job_spec_carries_args_and_hardening() {
        let config = Config {
            namespace: "hccc".to_string(),
            service_account: None,
            ttl_secs: 120,
            active_deadline_secs: 180,
            timeout_secs: 180,
            poll_secs: 2,
            cpu: "500m".to_string(),
            memory: "256Mi".to_string(),
            image_pull_policy: "IfNotPresent".to_string(),
        };
        let job = build_job(
            "hccc-judge-1-2",
            "runner:latest",
            &["a".to_string(), "exitcode".to_string(), "b".to_string()],
            &config,
        );
        let spec = job.spec.expect("spec");
        assert_eq!(spec.backoff_limit, Some(0));
        assert_eq!(spec.ttl_seconds_after_finished, Some(120));
        let pod = spec.template.spec.expect("pod spec");
        assert_eq!(pod.restart_policy.as_deref(), Some("Never"));
        assert_eq!(pod.automount_service_account_token, Some(false));
        assert_eq!(pod.containers.len(), 1);
        assert_eq!(
            pod.containers[0].args.as_ref().unwrap().len(),
            3,
            "image/test-target/testcases args preserved"
        );
        let sc = pod.containers[0].security_context.as_ref().expect("sc");
        assert_eq!(sc.allow_privilege_escalation, Some(false));
    }

    #[test]
    fn job_finished_detects_conditions_and_counters() {
        use k8s_openapi::api::batch::v1::JobCondition;
        let done = k8s_openapi::api::batch::v1::JobStatus {
            conditions: Some(vec![JobCondition {
                type_: "Complete".to_string(),
                status: "True".to_string(),
                ..Default::default()
            }]),
            ..Default::default()
        };
        assert!(job_finished(&done));
        let running = k8s_openapi::api::batch::v1::JobStatus {
            active: Some(1),
            ..Default::default()
        };
        assert!(!job_finished(&running));
    }
}
