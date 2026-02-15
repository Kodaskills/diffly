# ─── Build stage ───
FROM rust:1.93-slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY Cargo.toml Cargo.lock* sailfish.toml ./
COPY src/ ./src/

RUN cargo build --release --features cli

# ─── Runtime stage ───
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /build/target/release/diffly /app/diffly

ENTRYPOINT ["/app/diffly"]
# Default subcommand: override in docker-compose with e.g.
#   command: ["diff", "--config", "diffly.toml"]
#   command: ["snapshot", "--config", "diffly.toml", "--out", "/app/snapshot"]
#   command: ["check-conflicts", "--config", "diffly.toml", "--snapshot", "/app/snapshot"]
CMD ["diff", "--config", "diffly.toml"]
