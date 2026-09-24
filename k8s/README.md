# HCCC on Kubernetes

This directory deploys HCCC with multi-replica `judge-server`: each
submission runs as a one-shot Kubernetes `Job` via `hccc-k8s-job-runner`
(`HCCC_RUNNER_EXECUTABLE`), and Pending submits are claimed with
`SELECT ... FOR UPDATE SKIP LOCKED` plus a lease, so replicas never judge
the same submit twice.

## 0. Images

The GitHub Actions workflows publish these application images from `main`:

```text
ghcr.io/latte72r/hccc-web:latest
ghcr.io/latte72r/hccc-judge:latest
ghcr.io/latte72r/hccc-frontend:latest
```

The test runner images remain configurable through `hccc-config`.

## 1. Namespace, Secrets, Config

```bash
kubectl apply -f k8s/namespace.yaml
kubectl -n hccc create secret generic hccc-postgres \
  --from-literal=username=kcs1959 \
  --from-literal=password='<strong-password>' \
  --from-literal=database-url='postgres://kcs1959:<strong-password>@db:5432/hccc_judge'
kubectl -n hccc create configmap hccc-config \
  --from-literal=admin-user-ids='1' \
  --from-literal=contest-begin='2026-09-23T00:00:00+09:00' \
  --from-literal=contest-end='2026-09-26T15:00:00+09:00' \
  --from-literal=contest-event-name='KCS Summer Camp '"'"'26' \
  --from-literal=runner-image-x8664='ghcr.io/humanccompilercontest/hccc_infra:test_runner_x8664-develop' \
  --from-literal=runner-image-riscv='ghcr.io/humanccompilercontest/hccc_infra:test_runner_riscv-develop'
```

`contest-begin` / `contest-end` は初回シード用です。運用中の変更は管理画面
(`/admin` の大会期間) から行います。

## 2. Migrate existing data (only if you have a Compose postgres:14 volume)

Postgres major versions cannot reuse the old data directory. From `HCCC_infra`
with the old stack's `.env`:

```bash
./scripts/migrate-pg14-to-pg18.sh
# then load the dump into the cluster DB port-forward:
kubectl -n hccc port-forward sts/db 5432:5432 &
psql "$DATABASE_URL" -f backups/hccc-pg14-backup-<stamp>.sql
psql "$DATABASE_URL" -f scripts/migrations/002_judge_claim.sql
```

Fresh installs need nothing: `scripts/init.sql` is mounted as the only
database initialization script and runs once on the first StatefulSet boot.

## 3. Deploy

Run Kustomize from the repository root so the database init ConfigMap can use
`scripts/init.sql` without escaping the Kustomize root:

```bash
kubectl apply -k .
kubectl -n hccc rollout status statefulset/db
kubectl -n hccc rollout status deploy/web-server
kubectl -n hccc rollout status deploy/frontend
kubectl -n hccc rollout status deploy/judge-server
kubectl -n hccc get jobs -w   # Jobs appear as submissions arrive
```

The included Ingress exposes `frontend:3000` through Traefik at
`hccc.latte72.net`. The backend remains cluster-internal.

## 4. Operate

- Scale judges: `kubectl -n hccc scale deploy/judge-server --replicas=5`.
  Claims expire after `JUDGE_CLAIM_LEASE_SECS` (default 300s), so crashed
  replicas' submits are re-judged automatically.
- Job pods are denied all ingress/egress by `networkpolicy.yaml`
  (equivalent of the Compose `--network=none` sandbox).
- Runner resources/timeouts: `HCCC_JOB_CPU`, `HCCC_JOB_MEMORY`,
  `HCCC_JOB_TIMEOUT_SECS`, `HCCC_JOB_TTL_SECONDS` (see
  `k8s_job_adapter/src/main.rs`).
