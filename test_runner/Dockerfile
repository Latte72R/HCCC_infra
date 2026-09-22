FROM rust:1.98.1-bookworm AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends musl-tools \
    && rm -rf /var/lib/apt/lists/* \
    && rustup target add x86_64-unknown-linux-musl
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl --bin test_runner --features x8664

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y --no-install-recommends binutils gcc libc6-dev \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/test_runner /work/
ENTRYPOINT ["/work/test_runner"]
