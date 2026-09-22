FROM rust:1.98.1-bookworm AS builder

WORKDIR /app
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,id=hccc-web-bookworm,target=/app/target \
    cargo build --release && cp target/release/web_server /app/web_server

FROM debian:bookworm-slim AS runner
EXPOSE 55301
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
  libssl3 curl \
  && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/web_server ./web_server

CMD ["/app/web_server"]
