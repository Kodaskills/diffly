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
CMD ["--config", "config.toml"]
