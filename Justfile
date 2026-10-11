# Justfile for kd
# Standard command runner for Rust workspaces.
# Shared workstations keep jobs=2 to avoid CPU starvation. CI passes jobs=10.

set shell := ["bash", "-euo", "pipefail", "-c"]

jobs := "2"
nextest_profile := "default"
org := "cerebral-work"

# Display available commands
default:
    @just --list 2>/dev/null || make help

# Display this help message
help:
    @just --list 2>/dev/null || make help

# Compile the workspace with locked dependencies and host throttling
build:
    cargo build --workspace --locked -j {{jobs}}

# Format all code
fmt:
    cargo fmt --all

# Check code formatting without modifying
fmt-check:
    cargo fmt --all -- --check

# Sort Cargo.toml dependencies alphabetically
sort:
    @command -v cargo-sort >/dev/null 2>&1 || { echo "cargo-sort not found; install with: cargo install cargo-sort -j {{jobs}}" >&2; exit 1; }
    cargo sort --workspace

# Check Cargo.toml dependencies are sorted alphabetically
sort-check:
    @command -v cargo-sort >/dev/null 2>&1 || { echo "cargo-sort not found; install with: cargo install cargo-sort -j {{jobs}}" >&2; exit 1; }
    cargo sort --check --workspace

# Lint with clippy, warnings are errors
clippy:
    cargo clippy --workspace --all-targets --locked -j {{jobs}} -- -D warnings

# Run unit and integration tests via nextest with bounded threads
test:
    cargo nextest run --workspace --locked --build-jobs {{jobs}} --profile {{nextest_profile}}

# Run doc tests
doc-test:
    cargo test --doc --workspace --locked -j {{jobs}}

# Verify feature powerset
powerset:
    cargo hack --feature-powerset --no-dev-deps check --workspace -j {{jobs}}

# Guard public API against accidental breaking semver drift
semver:
    @command -v cargo-semver-checks >/dev/null 2>&1 || { echo "cargo-semver-checks not found" >&2; exit 1; }
    cargo semver-checks check-release --workspace

# Verify workspace compiles against declared MSRV floor (rust-version)
msrv:
    @msrv=$$(sed -n 's/^rust-version *= *"\(.*\)"/\1/p' Cargo.toml | head -1); \
    if rustup toolchain list | grep -q "^$$msrv"; then \
        cargo +"$$msrv" check --workspace --all-targets -j {{jobs}}; \
    else \
        echo "msrv: toolchain $$msrv not installed (install with: rustup toolchain install $$msrv)" >&2; \
    fi

# Audit licenses, bans, advisories, and sources across target triples
deny:
    cargo deny check

# The push gate: fmt, clippy, test, doc-test, deny
check: fmt-check clippy test doc-test deny

# What the CI rust job runs
ci:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets --locked -j {{jobs}} -- -D warnings
    cargo nextest run --workspace --locked --build-jobs {{jobs}} --profile ci
    cargo test --doc --workspace --locked -j {{jobs}}

# Repository hygiene: typos, sort, unused deps, specs, toolchain, pins
hygiene: typos sort-check machete spec-check toolchain-check pin-check
    for f in scripts/*.sh; do bash -n "$f"; done

# Spell check
typos:
    typos

# Hunt down unused dependencies bloating the build graph
machete:
    cargo machete

# Validate specs/ against LeanSpec contracts
spec-check:
    cargo xtask spec-check

# Verify active rustc matches rust-toolchain.toml pinned channel
toolchain-check:
    cargo xtask toolchain-check

# Verify dependency pins in specs/0004 match Cargo.lock
pin-check:
    cargo xtask pin-check

# Fetch pinned estate contracts into .contracts/ (contracts.lock)
fetch-contracts:
    cargo xtask fetch-contracts

# Vendor all dependencies into vendor/crates for offline builds
vendor:
    cargo vendor vendor/crates

# Verify that vendor/crates matches Cargo.lock
vendor-check:
    @if [ -d vendor/crates ]; then \
        cargo vendor --versioned-dirs vendor/crates >/dev/null; \
        git status --porcelain vendor/crates | grep -q . && { echo "vendor-check: vendor/crates is out of sync with Cargo.lock" >&2; exit 1; } || echo "vendor-check: ok"; \
    else \
        echo "vendor-check: vendor/crates directory not found (run make vendor first)" >&2; exit 1; \
    fi

# Build CLI, verify version string, and prove domain crate has zero I/O
smoke:
    cargo build -p kd --locked -j {{jobs}}
    @out="$$(cargo run -q -p kd --locked -j {{jobs}} -- --version)"; \
    echo "$$out"; \
    [[ "$$out" =~ ^kd\ [0-9]+\.[0-9]+\.[0-9]+\ \(.+\)$$ ]] || { echo "smoke: unexpected --version output" >&2; exit 1; }; \
    cargo run -q -p kd --locked -j {{jobs}} -- demo | grep -q '"done"' || { echo "smoke: demo did not reach done" >&2; exit 1; }; \
    if grep -rnE 'std::net|std::fs|tokio|std::process' crates/kd-domain/src; then \
        echo "smoke: kd-domain must not use I/O" >&2; \
        exit 1; \
    fi; \
    echo "smoke: ok"

# Check dependencies for known advisories
audit:
    cargo audit

# Write a CycloneDX SBOM (*.cdx.json)
sbom:
    cargo cyclonedx --format json

# Build API docs
doc:
    cargo doc --workspace --no-deps -j {{jobs}}

# Build release binaries for the host into dist/
dist:
    DIST_JOBS={{jobs}} ./scripts/dist.sh

# Roll CHANGELOG and bump version (VERSION=x.y.z)
release-notes:
    [[ "${VERSION:-}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "usage: make release-notes VERSION=x.y.z" >&2; exit 2; }
    [ -f CHANGELOG.md ] || { echo "CHANGELOG.md not found" >&2; exit 1; }
    grep -q '^## \[Unreleased\]' CHANGELOG.md || { echo "CHANGELOG.md has no [Unreleased] heading" >&2; exit 1; }
    sed -i '0,/^version = ".*"/s//version = "'"${VERSION}"'"/' Cargo.toml
    cargo update -w --offline 2>/dev/null || cargo update -w
    awk -v v="${VERSION}" -v d="$(date -u +%F)" ' \
        /^## \[Unreleased\]/ && !done { print; print ""; print "## [" v "] - " d; done = 1; next } \
        { print }' CHANGELOG.md > CHANGELOG.md.tmp
    mv CHANGELOG.md.tmp CHANGELOG.md
    echo "release-notes: set version ${VERSION}. Review the diff, then open a PR to development."

# Check the custody setup (sbx doctor or blackwall doctor)
custody-check:
    @if command -v sbx >/dev/null 2>&1; then \
        sbx doctor; \
    elif command -v blackwall >/dev/null 2>&1; then \
        blackwall doctor; \
    else \
        echo "custody-check: neither sbx nor blackwall installed" >&2; exit 1; \
    fi

# Open this repo as a herdr agent workspace
workspace:
    bash scripts/herdr-workspace.sh "$(pwd)"

# Install the pgt CLI into .tools/bin
pgt-install:
    cargo install --git https://github.com/cerebral-work/pgt --rev 54d90f5 pgt --root .tools --locked


