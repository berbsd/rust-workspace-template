# =============================================================================
# Justfile - Task Runner
# =============================================================================
# Common development commands for the workspace.
# Documentation: https://github.com/casey/just
#
# Usage:
#   just              # List all available commands
#   just <command>    # Run a specific command
#   just check        # Run all checks
#
# To add a new command:
#   1. Add a recipe below with a descriptive comment
#   2. Use `@` prefix to suppress command echo
#   3. Keep recipes to a handful of lines. Anything longer belongs in an
#      executable script under `bin/`, invoked as `@bin/<name>` — a real script
#      can be run directly, linted with shellcheck, and diffed without the
#      recipe indentation.
# =============================================================================

# Show all available commands
default:
    @just --list

# =============================================================================
# Quality Checks
# =============================================================================

# Run all checks (format, lint, test, security)
# Full gate: format, lint, test, spellcheck.
#
# Every step that compiles uses `--all-features`, and that is load-bearing
# rather than tidiness. `--all-features` changes the `--cfg feature=` flags
# reaching rustc, so a step without it resolves a *different* feature set,
# gets its own fingerprints, and compiles the whole workspace again.
#
# Measured 2026-09-10: `cargo build --all-targets --all-features` was a 0.52s
# no-op, and `cargo test --workspace --no-run` immediately recompiled 47 crates
# in 37s. Adding `--all-features` to that same command made it a no-op too. The
# clippy step here has always passed the flag; the test step had not, so every
# `just check` built two parallel artifact universes that shared nothing —
# neither Cargo's cache nor sccache can help, because the compilations genuinely
# differ.
#
# `.cargo/config.toml`'s `b`/`c`/`cl` aliases already force
# `--all-targets --all-features` for the same reason. Keep any new compiling
# step in step with them.
check:
    @clear
    @echo "=== Format Check ==="
    @cargo +nightly fmt --check
    @echo "=== Clippy ==="
    @cargo clippy --all-targets --all-features
    @echo "=== Tests ==="
    @just db-ensure
    @DATABASE_URL="${DATABASE_URL:-postgres://postgres:postgres@localhost:5432/postgres}" cargo test --all-features
    @echo "=== Typos ==="
    @typos
    @echo "=== Security Check ==="
    @cargo deny check
    @echo "=== All Checks Passed ==="

# Format all code (Rust + TOML)
fmt:
    @echo "Formatting Rust files..."
    @cargo +nightly fmt --all
    @echo "Formatting TOML files..."
    @RUST_LOG="warn" taplo format
    @echo "Done!"

# Run clippy with automatic fixes
fix:
    cargo clippy --fix --all-targets --all-features --allow-dirty --allow-staged

# Check for typos in source code
typos:
    typos

# Fix typos in source code
typos-fix:
    typos --write-changes

# =============================================================================
# Testing
# =============================================================================

# Run tests with nextest (faster test runner)
# `--all-features`, matching `check` — see its comment for why a step without
# it compiles the workspace a second time.
test:
    cargo nextest run --all-features

# Test specific crate
test-crate crate:
    cargo nextest run --all-features -p {{ crate }}

# Run only in-process library unit tests (fast, no Docker required)
test-unit:
    cargo nextest run --all-features --lib

# Run integration tests only — spins up testcontainers, needs Docker running
test-db:
    @echo "Integration tests require Docker."
    @docker info >/dev/null 2>&1 || { echo "Docker is not running"; exit 1; }
    cargo nextest run --all-features --test '*'

# Start a shared local Postgres for #[sqlx::test] (prototype)
pg-local:
    @docker info >/dev/null 2>&1 || { echo "Docker is not running"; exit 1; }
    docker rm -f sqlx-test-pg >/dev/null 2>&1 || true
    docker run -d --name sqlx-test-pg -p 5432:5432 \
        -e POSTGRES_PASSWORD=postgres pgvector/pgvector:pg18 >/dev/null
    @echo "Postgres up. Export:"
    @echo "  export DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres"

# Ensure a local Postgres is up and accepting connections, starting one if not.
#
# `#[sqlx::test]` reads DATABASE_URL and panics without it, so `just check` used
# to fail every DB-backed test with `DATABASE_URL must be set` unless you had
# run `pg-local` and exported the variable by hand. CI provisions a service
# container and so was always green on exactly the tests that failed locally —
# the two disagreed about what "run the tests" means, and local lost.
#
# Idempotent, unlike `pg-local`: it reuses a running container rather than
# `docker rm -f`-ing one somebody is using. Waits for readiness because the
# container accepts a port binding before Postgres itself will answer, and
# nextest starting into that gap fails as if the database were absent.
db-ensure:
    @docker info >/dev/null 2>&1 || { echo "Docker is not running — start it, or export DATABASE_URL to point elsewhere"; exit 1; }
    @test -n "$(docker ps -q -f name=^sqlx-test-pg$)" || just pg-local
    @for i in $(seq 1 30); do \
        docker exec sqlx-test-pg pg_isready -U postgres >/dev/null 2>&1 && exit 0; \
        sleep 1; \
    done; \
    echo "Postgres did not become ready in 30s"; exit 1

# Stop the shared local Postgres started by `pg-local`
pg-local-stop:
    docker rm -f sqlx-test-pg >/dev/null 2>&1 || true

# Run tests with coverage report
cov:
    cargo llvm-cov --html
    @echo "Coverage report: target/llvm-cov/html/index.html"

# =============================================================================
# Building
# =============================================================================

# Build release binaries
build-release:
    cargo build --release

# Clean build artifacts
clean:
    cargo clean

# =============================================================================
# Running
# =============================================================================

# Run specific service
run service:
    cargo run -p {{ service }}

# Watch for changes and check
watch:
    cargo watch -x check -x test

# =============================================================================
# Documentation
# =============================================================================

# Generate and open docs (all crates or specific crate)
doc crate="":
    #!/usr/bin/env bash
    if [ -z "{{ crate }}" ]; then
      cargo doc --open
    else
      cargo doc --open -p {{ crate }}
    fi

# =============================================================================
# Security & Maintenance
# =============================================================================

# Check for outdated dependencies
outdated:
    cargo outdated -R

# Check for security vulnerabilities
audit:
    cargo audit

# Check for unused dependencies (fast, heuristic)
unused:
    cargo machete

# `--all-features` is load-bearing: without it, feature-gated code never
# compiles, so every optional dependency (sqlx, utoipa, garde, mockall, …)
# is reported as unused. Matches the workspace convention of always pairing
# `--all-targets` with `--all-features`.

# Check for unused dependencies (thorough, requires nightly)
unused-thorough:
    cargo +nightly udeps --all-targets --all-features

# =============================================================================
# CI Helpers
# =============================================================================

# Run CI checks (same as CI pipeline)
ci: check
    @echo "CI checks passed!"

# =============================================================================
# Git Hooks & Compliance
# =============================================================================

# Install git hooks via lefthook
hooks:
    lefthook install

# Scan for secrets in the entire repo (tracked content + history)
secrets:
    betterleaks git . --no-banner --redact

# Verify commit history follows conventional commits
verify-commits:
    cog check

# Generate changelog (full)
changelog:
    git cliff -o CHANGELOG.md

# Preview changelog for latest release only
changelog-latest:
    git cliff --latest

# =============================================================================
# Docker
# =============================================================================

# Build Docker image for a service (uses cargo-chef for cached deps)
docker-build service:
    docker build --build-arg SERVICE={{ service }} -t {{ service }} .

# Run a service in Docker
docker-run service port="3000":
    docker run -p {{ port }}:3000 {{ service }}

# =============================================================================
# Releases
# =============================================================================

# Release workspace crates (patch/minor/major). Use --dry-run first.
release level="patch" *args="":
    cargo release {{ level }} {{ args }}

# Preview what a release would do (no changes made)
release-dry-run level="patch":
    cargo release {{ level }} --dry-run

# Check for semver violations in public API
semver-check:
    cargo semver-checks
