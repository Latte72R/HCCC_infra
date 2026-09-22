FROM rust:1.98.1-bookworm AS builder

# Build judge_server and the Kubernetes Job runner adapter.
# Build context is the HCCC_infra workspace root (all members must be
# present for Cargo workspace resolution; target/ etc. excluded via .dockerignore).
WORKDIR /app
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,id=hccc-judge-bookworm,target=/app/target \
    cargo build --release -p judge_server -p k8s_job_adapter \
    && mkdir -p /out \
    && cp target/release/judge_server /out/judge_server \
    && cp target/release/hccc-k8s-job-runner /out/k8s-job-runner

FROM debian:bookworm-slim AS runner
WORKDIR /app

# Install docker client(for running test_runner via the local Docker socket)
RUN apt-get update && apt-get install -y \
  ca-certificates curl \
  && rm -rf /var/lib/apt/lists/*

ENV DOCKER_CLIENT_VERSION=29.8.1
RUN curl -fsSL https://download.docker.com/linux/static/stable/x86_64/docker-${DOCKER_CLIENT_VERSION}.tgz \
  | tar -xzC /usr/local/bin --strip=1 docker/docker

# Copy judge_server and the K8s Job adapter.
# In Kubernetes set HCCC_RUNNER_EXECUTABLE=/app/k8s-job-runner to run each
# submission as a Job instead of a local docker run.
COPY --from=builder /out/judge_server ./judge_server
COPY --from=builder /out/k8s-job-runner ./k8s-job-runner

CMD ["/app/judge_server"]
