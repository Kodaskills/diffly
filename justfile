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

# Tag and push a release — triggers the release workflow
[group('release')]
release version:
    @echo "Tagging v{{ version }}…"
    git tag -a "v{{ version }}" -m "Release v{{ version }}"
    git push origin "v{{ version }}"
