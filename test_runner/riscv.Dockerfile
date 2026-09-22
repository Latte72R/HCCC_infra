ARG RISCV_TOOLCHAIN_IMAGE=ghcr.io/latte72r/riscv_toolchain_docker:master

FROM rust:1.98.1-bookworm AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends musl-tools \
    && rm -rf /var/lib/apt/lists/* \
    && rustup target add x86_64-unknown-linux-musl
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl --bin test_runner --no-default-features --features riscv

FROM ${RISCV_TOOLCHAIN_IMAGE}
ENV PATH="/opt/riscv/bin:${PATH}"
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/test_runner /work/
ENTRYPOINT ["/work/test_runner"]
