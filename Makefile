# Makefile for unsigned-gg/kd
# Shared workstations keep JOBS=2 to prevent CPU starvation. CI passes JOBS=10.
SHELL := bash
.SHELLFLAGS := -euo pipefail -c
.ONESHELL:
.DEFAULT_GOAL := help

JOBS ?= 2
NEXTEST_PROFILE ?= default
ORG ?= cerebral-work

export JOBS NEXTEST_PROFILE ORG

.PHONY: help default build fmt fmt-check sort sort-check clippy test doc-test powerset semver msrv \
	deny check ci hygiene typos machete spec-check toolchain-check pin-check fetch-contracts \
	vendor vendor-check smoke audit sbom doc dist release-notes custody-check workspace pgt-install

##@ Help

help: ## Display available Makefile targets
	@awk 'BEGIN {FS = ":.*##"; printf "\n\033[1mUsage:\033[0m\n  make \033[36m<target>\033[0m\n"} /^[a-zA-Z0-9_.-]+:.*?##/ { printf "  \033[36m%-24s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1;33m%s\033[0m\n", substr($$0, 5) }' $(MAKEFILE_LIST)

default: help

##@ Build & Verification

build: ## Compile the workspace with locked dependencies and host throttling
	cargo build --workspace --locked -j $(JOBS)

fmt: ## Format all code
	cargo fmt --all

fmt-check: ## Check code formatting without modifying
	cargo fmt --all -- --check

sort: ## Sort Cargo.toml dependencies alphabetically
	@command -v cargo-sort >/dev/null 2>&1 || { echo "cargo-sort not found; install with: cargo install cargo-sort -j $(JOBS)" >&2; exit 1; }
	cargo sort --workspace

sort-check: ## Check Cargo.toml dependencies are sorted alphabetically
	@command -v cargo-sort >/dev/null 2>&1 || { echo "cargo-sort not found; install with: cargo install cargo-sort -j $(JOBS)" >&2; exit 1; }
	cargo sort --check --workspace

clippy: ## Lint with clippy, warnings are errors
	cargo clippy --workspace --all-targets --locked -j $(JOBS) -- -D warnings

test: ## Run unit and integration tests via nextest (or cargo test fallback)
	@if command -v cargo-nextest >/dev/null 2>&1; then \
		cargo nextest run --workspace --locked --build-jobs $(JOBS) --profile $(NEXTEST_PROFILE); \
	else \
		cargo test --workspace --locked -j $(JOBS); \
	fi

doc-test: ## Run doc tests
	cargo test --doc --workspace --locked -j $(JOBS)

powerset: ## Verify feature powerset
	cargo hack --feature-powerset --no-dev-deps check --workspace -j $(JOBS)

semver: ## Guard public API against accidental breaking semver drift
	@command -v cargo-semver-checks >/dev/null 2>&1 || { echo "cargo-semver-checks not found" >&2; exit 1; }
	cargo semver-checks check-release --workspace

msrv: ## Verify workspace compiles against declared MSRV floor (rust-version)
	@msrv=$$(sed -n 's/^rust-version *= *"\(.*\)"/\1/p' Cargo.toml | head -1)
	if rustup toolchain list | grep -q "^$$msrv"; then
		cargo +"$$msrv" check --workspace --all-targets -j $(JOBS)
	else
		echo "msrv: toolchain $$msrv not installed (install with: rustup toolchain install $$msrv)" >&2
	fi

deny: ## Audit licenses, bans, advisories, and sources across target triples
	cargo deny check

check: fmt-check clippy test doc-test deny ## The push gate: fmt, clippy, test, doc-test, deny

ci: ## Run full CI Rust verification suite
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets --locked -j $(JOBS) -- -D warnings
	if command -v cargo-nextest >/dev/null 2>&1; then
		cargo nextest run --workspace --locked --build-jobs $(JOBS) --profile ci
	else
		cargo test --workspace --locked -j $(JOBS)
	fi
	cargo test --doc --workspace --locked -j $(JOBS)

##@ Hygiene & Contracts

hygiene: typos sort-check machete spec-check toolchain-check pin-check ## Repository hygiene: typos, sort, unused deps, specs, toolchain, pins
	for f in scripts/*.sh; do bash -n "$$f"; done

typos: ## Spell check
	typos

machete: ## Hunt down unused dependencies bloating the build graph
	cargo machete

spec-check: ## Validate specs/ against LeanSpec contracts
	cargo xtask spec-check

toolchain-check: ## Verify active rustc matches rust-toolchain.toml pinned channel
	cargo xtask toolchain-check

pin-check: ## Verify dependency pins in specs/0004 match Cargo.lock
	cargo xtask pin-check

fetch-contracts: ## Fetch pinned estate contracts into .contracts/ (contracts.lock)
	cargo xtask fetch-contracts

vendor: ## Vendor all dependencies into vendor/crates for offline builds
	cargo vendor vendor/crates

vendor-check: ## Verify that vendor/crates matches Cargo.lock
	@if [ -d vendor/crates ]; then
		cargo vendor --versioned-dirs vendor/crates >/dev/null
		git status --porcelain vendor/crates | grep -q . && { echo "vendor-check: vendor/crates is out of sync with Cargo.lock" >&2; exit 1; } || echo "vendor-check: ok"
	else
		echo "vendor-check: vendor/crates directory not found (run make vendor first)" >&2
		exit 1
	fi

smoke: ## Build CLI, verify version string, and prove kd-domain has zero I/O
	cargo build -p kd --locked -j $(JOBS)
	@out="$$(cargo run -q -p kd --locked -j $(JOBS) -- --version)"
	echo "$$out"
	[[ "$$out" =~ ^kd\ [0-9]+\.[0-9]+\.[0-9]+\ \(.+\)$$ ]] || { echo "smoke: unexpected --version output" >&2; exit 1; }
	cargo run -q -p kd --locked -j $(JOBS) -- demo | grep -q '"done"' || { echo "smoke: demo did not reach done" >&2; exit 1; }
	if grep -rnE 'std::net|std::fs|tokio|std::process' crates/kd-domain/src; then
		echo "smoke: kd-domain must not use I/O" >&2
		exit 1
	fi
	echo "smoke: ok"

##@ Release, Docs & Tooling

audit: ## Check dependencies for known advisories
	cargo audit

sbom: ## Write a CycloneDX SBOM (*.cdx.json)
	cargo cyclonedx --format json

doc: ## Build API docs
	cargo doc --workspace --no-deps -j $(JOBS)

dist: ## Build release binaries for the host into dist/
	DIST_JOBS=$(JOBS) ./scripts/dist.sh

release-notes: ## Roll CHANGELOG and bump version (VERSION=x.y.z)
	[[ "$${VERSION:-}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$$ ]] || { echo "usage: make release-notes VERSION=x.y.z" >&2; exit 2; }
	[ -f CHANGELOG.md ] || { echo "CHANGELOG.md not found" >&2; exit 1; }
	grep -q '^## \[Unreleased\]' CHANGELOG.md || { echo "CHANGELOG.md has no [Unreleased] heading" >&2; exit 1; }
	sed -i '0,/^version = ".*"/s//version = "'"$${VERSION}"'"/' Cargo.toml
	cargo update -w --offline 2>/dev/null || cargo update -w
	awk -v v="$${VERSION}" -v d="$$(date -u +%F)" ' \
		/^## \[Unreleased\]/ && !done { print; print ""; print "## [" v "] - " d; done = 1; next } \
		{ print }' CHANGELOG.md > CHANGELOG.md.tmp
	mv CHANGELOG.md.tmp CHANGELOG.md
	echo "release-notes: set version $${VERSION}. Review the diff, then open a PR to development."

custody-check: ## Check the custody setup (sbx doctor or blackwall doctor)
	@if command -v sbx >/dev/null 2>&1; then
		sbx doctor
	elif command -v blackwall >/dev/null 2>&1; then
		blackwall doctor
	else
		echo "custody-check: neither sbx nor blackwall installed" >&2
		exit 1
	fi

workspace: ## Open this repo as a herdr agent workspace
	bash scripts/herdr-workspace.sh "$$(pwd)"

pgt-install: ## Install the pgt CLI into .tools/bin
	cargo install --git https://github.com/cerebral-work/pgt --rev 54d90f5 pgt --root .tools --locked -j $(JOBS)
