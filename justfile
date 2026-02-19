# diffly — task runner (https://just.systems)

[group('help')]
default:
    @just --list

# ─── Dev ──────────────────────────────────────────────────────────────────────

# Run all unit tests
[group('dev')]
test:
    cargo test --locked --features cli

# Build the CLI binary (debug)
[group('dev')]
build:
    cargo build --features cli

# Build the CLI binary (release)
[group('dev')]
build-release:
    cargo build --locked --release --features cli

# Run clippy lints
[group('dev')]
lint:
    cargo clippy --features cli -- -D warnings

# Format source code
[group('dev')]
fmt:
    cargo fmt

# ─── Examples (Docker) ────────────────────────────────────────────────────────

# Run the PostgreSQL example (build + diff + teardown)
[group('examples')]
example-pg:
    docker compose -f examples/postgresql/docker-compose.yml up --build
    docker compose -f examples/postgresql/docker-compose.yml down -v

# Run the MySQL example
[group('examples')]
example-mysql:
    docker compose -f examples/mysql/docker-compose.yml up --build
    docker compose -f examples/mysql/docker-compose.yml down -v

# Run the MariaDB example
[group('examples')]
example-mariadb:
    docker compose -f examples/mariadb/docker-compose.yml up --build
    docker compose -f examples/mariadb/docker-compose.yml down -v

# Run the SQLite example
[group('examples')]
example-sqlite:
    docker compose -f examples/sqlite/docker-compose.yml up --build
    docker compose -f examples/sqlite/docker-compose.yml down -v

# Run all examples sequentially
[group('examples')]
example-all:
    just example-pg
    just example-mysql
    just example-mariadb
    just example-sqlite

# ─── Changelog ────────────────────────────────────────────────────────────────

# Preview the full changelog (requires: cargo install git-cliff)
[group('changelog')]
changelog:
    git-cliff --config cliff.toml

# Preview only unreleased changes since the last tag
[group('changelog')]
changelog-unreleased:
    git-cliff --config cliff.toml --unreleased

# ─── Release ──────────────────────────────────────────────────────────────────

# Tag locally using the version from Cargo.toml, with unreleased changelog as annotation
[group('release')]
tag:
    #!/usr/bin/env bash
    version=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')
    notes=$(git-cliff --config cliff.toml --unreleased --strip all 2>/dev/null)
    echo "Tagging v${version}…"
    git tag -a "v${version}" -m "Release v${version}" -m "${notes}"
    echo "Done. Run 'just push-tag' to trigger the release workflow."

# Push the current Cargo.toml version tag to origin — triggers the release workflow
[group('release')]
push-tag:
    #!/usr/bin/env bash
    version=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')
    git push origin "v${version}"

# Delete the current Cargo.toml version tag locally and on origin
[group('release')]
tag-remove:
    #!/usr/bin/env bash
    version=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')
    git tag -d "v${version}"
    # git push origin --delete "v${version}"

# Tag locally and push in one step
[group('release')]
release:
    just tag && just push-tag
