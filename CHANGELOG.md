# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Signals framework, process supervision, and terminal hygiene (`unsigned-gg/kd`): Linux `pidfd` lifecycle management (`pidfd_open`, `pidfd_send_signal`), `PR_SET_CHILD_SUBREAPER` zombie adoption and reaping, negative PGID broadcast signaling, declarative 2-phase graceful shutdown state machine (Drain → Escalate → ForceKill), and RAII `TerminalGuard`. Dual-licensed Apache-2.0 OR MIT.
- Port the cas-substrate release follow-ups (CER-2670): drop the dead `release` trigger and the never-run `attach` job from `sbom.yml` (a release made with `GITHUB_TOKEN` starts no workflows, so neither ever fired), and set `git_release_name = "v{{ version }}"` in `release-plz.toml` so the GitHub release title matches the tag. Spec 0005 names both behaviors.

### Added
- Rust monorepo vendoring and contract pinning: `Justfile` with POSIX awk-parsing `Makefile` wrapper, cargo vendoring support (`make vendor` into `vendor/crates`), `contracts.lock` with sha256-verified `cargo xtask fetch-contracts`, dependency `pin-check` in xtask, and `sbx.toml` custody configuration.
- Blackwall custody setup: `blackwall.toml` (claude provider for the delta trace, `make check` and `make hygiene` as the verify gate, PRs into `development`, automerge off because merge-gate squash-merges), a `tasks/` library README, the `custody-run` skill and `make custody-check`. `.blackwall/` is ignored.
- Release binaries: the `release` run builds musl archives for x86_64 and aarch64 with build provenance and `SHA256SUMS-<target>`, and attaches them to the GitHub release. `make dist` builds the host archives locally.

### Removed
- Non-native tooling: removed Moon tasks, `.moon/` configuration, `.prototools`, `moon-affected` CI job, and proto shims in favor of 100% native Rust tooling (`cargo`, `just`, `xtask`).

## [0.1.0] - 2026-10-08

### Added
- `make workspace` and `scripts/herdr-workspace.sh` open a repo as a herdr agent workspace: tab `agents` (Claude Code on Fable, Antigravity), tab `saga` (dagr, skald, edda, gates, runs, world) and tab `omp` (three workers). `make hygiene` checks shell script syntax.
- The estate Rust monorepo template: a Cargo workspace with a zero-I/O domain crate, a CLI crate and an `xtask`; a `Makefile` with the gate targets; LeanSpec specs with `make spec-check`; CI workflows; moon tasks; lefthook and gitleaks; `AGENTS.md` with four skills; Claude Code hooks; issue and PR templates; and `make new-repo` to start a repo from it.

### Changed
- The promotion PR from `development` to `main` now merges on green CI, by operator ruling on 2026-10-08. AGENTS.md, CONTRIBUTING.md, the release skill and spec 0003 say so. Repos made from the template inherit the rule. Spec 0003 also names the real required check `rust / cargo`.

### Fixed
- `make new-repo` runs `cargo fmt --all` after the rename. A name that sorts before `clap`, such as `cas`, left the CLI imports out of order and failed `make check`.
- `make new-repo` takes `REPO=<repo-name>` for links to the GitHub repo. Without it, `NAME=cas` for the repo `cas-substrate` wrote `cerebral-work/cas` into `Cargo.toml`.
- `make new-repo` deletes the template-only Makefile targets. Before, the rename rewrote them into a broken `new-repo` target in every new repo.
- The moon `test` task passes `--no-tests=warn` to nextest. A crate with no tests made `moon-affected` fail.
- The `release` workflow installs a pinned `gh` 2.101.0 before release-plz. Before this, every push to `main` failed with `gh: command not found`, and no tag or release was made. The release-plz step now runs only on `main`.
- The `release` job gets `pull-requests: read`. release-plz lists the pull requests of the released commit, and without this scope the call fails with 403 (measured on `cerebral-work/cas-substrate` `main`).
