---
status: in-progress
created: 2026-10-08
tags: [verification, gates, evidence, make]
priority: high
depends_on: [0001-template-architecture]
---

# 0002-verification-gates-and-evidence

## Overview

A claim of "done" needs proof. This spec names the gates that give proof. It
also names the evidence tiers, and it sets the `-j` rule.

Evidence from 2026-10-08:

- The `ci-rust` image bakes Rust 1.94.1. Its rustup directory is read-only to
  the runner user. CI cannot install a toolchain.
- The cortex CI `rust` job runs without `-j`. This is drift from the house rule.
- `nextest` is not installed in cortex or revenant CI. Their workflows use
  `|| cargo test`. That fallback hides failures.

## Design

### Gates

| Target | Runs | Use |
|---|---|---|
| `make check` | `fmt-check clippy test doc-test deny` | Before every push. House SOP. |
| `make hygiene` | `typos sort-check machete spec-check toolchain-check` | Before every push. |
| `make smoke` | Builds the CLI, runs `kd --version`, greps the domain crate for I/O | Before every PR. |
| `make ci` | `fmt-check clippy test doc-test` with `NEXTEST_PROFILE=ci` | What the CI `rust` job runs. |
| `make template-check` | `check`, `spec-check`, `smoke`, then the rename probe | Template repo only. |
| `cargo xtask ci` | `tools toolchain-check fmt-check clippy test doc-test deny typos machete` (XT-7) | What the CI `xtask-ci` job runs. |
| `cargo xtask install` | `ci`, then release builds of every binary (XT-8) | Before installing the binaries. |

The CI jobs `rust` and `hygiene` call these same targets. A gate that passes on
a workstation passes in CI.

### Evidence tiers

| Tier | Meaning | Example |
|---|---|---|
| verified | A command ran and the output proves the claim. | `make check` exit 0 |
| reported | Another agent or person said it. | "CI is green" in a handoff |
| heuristic | Inferred from a pattern. | "Same code, so it passes" |
| asserted | Stated with no source. | "It should work" |

Only `verified` satisfies a gate. A PR body lists the command and the result
for each claim. A grep that finds nothing is not proof of absence.

### The `-j` rule

Operator decision, Christian, 2026-10-08.

- Makefile sets `JOBS ?= 2`. Every cargo build, test and clippy call passes
  `-j $(JOBS)`.
- Nextest uses `--build-jobs $(JOBS)`. Its own `-j` flag sets test threads.
  Never use it for build jobs.
- Justfile and lefthook scripts use `-j 2` and `--build-jobs 2`.
- Shared workstations keep `JOBS=2`.

CI exception: CI calls make targets with `JOBS=10`. The `ci-rust` pod has a
CPU limit of 10 and runs one job at a time. The pod is ephemeral. No other
user shares it. The `_rust.yml` workflow also sets `CARGO_BUILD_JOBS` to the
same number.

### cargo xtask gates

Operator decision, Christian, 2026-10-13: `cargo xtask` is the one build
entrypoint for every estate repo, with nothing behind it but cargo and rustup.
The xtask gates sit beside the make and just targets above. Nothing is removed.

- **XT-1 (ubiquitous):** xtask shall provide the commands `tools`,
  `toolchain-check`, `fmt`, `fmt-check`, `clippy`, `test`, `doc-test`, `deny`,
  `typos`, `machete`, `ci` and `install`, and keep every other command under
  its name.
- **XT-2 (ubiquitous):** xtask shall keep the version of each tool it runs in
  one table: cargo-deny 0.19.9, typos-cli 1.49.0, cargo-machete 0.9.1 and
  cargo-nextest 0.9.104.
- **XT-3 (event-driven):** When `tools` runs, xtask shall ask each tool for its
  version the way the gates run it, with `.tools/bin` first on `PATH`. Only for
  a tool that is missing or at another version shall it run
  `cargo install --locked --version <pin> --root .tools --force <crate>`, under
  Rust 1.94.1 (`rustup run --install 1.94.1`).
- **XT-4 (unwanted behaviour):** If a tool still reports another version after
  its install, then `tools` shall fail and name the tool and both versions.
- **XT-5 (unwanted behaviour):** If `rustc --version` differs from the
  `channel` in `rust-toolchain.toml`, then `toolchain-check` shall fail and
  name both.
- **XT-6 (ubiquitous):** Each gate shall run from the repository root with
  `.tools/bin` first on `PATH`. N is `JOBS`, else 2.

  | Gate | Runs |
  |---|---|
  | `fmt` | `cargo fmt --all` |
  | `fmt-check` | `cargo fmt --all -- --check` |
  | `clippy` | `cargo clippy --workspace --all-targets --locked -j N -- -D warnings` |
  | `test` | `cargo nextest run --workspace --locked --build-jobs N`; `cargo test --workspace --locked -j N` when nextest is missing |
  | `doc-test` | `cargo test --doc --workspace --locked -j N` |
  | `deny` | `cargo deny check` |
  | `typos` | `typos` |
  | `machete` | `cargo machete` |

- **XT-7 (event-driven):** When `ci` runs, xtask shall run `tools`,
  `toolchain-check`, `fmt-check`, `clippy`, `test`, `doc-test`, `deny`,
  `typos` and `machete` in that order. It shall stop at the first step that
  fails, print one summary line per step, and exit non-zero if a step failed.
- **XT-8 (event-driven):** When `install` runs, xtask shall run `ci`, then
  `cargo build --release --locked --workspace --exclude xtask --bins -j N`.
- **XT-9 (event-driven):** When CI runs for a push or a pull request, the job
  `xtask-ci` shall run `cargo xtask ci` on the runner the Rust jobs use.

nextest reads `NEXTEST_PROFILE` itself, so CI picks the `ci` profile through
the environment, as `make ci` does.

## Plan

- [x] Write the Makefile targets with exact names from the table above.
- [x] Add `.config/nextest.toml` with profiles `default`, `ci`, `pre-push`.
- [x] Add `toolchain-check` to `xtask`.
- [x] Wire `rust` and `hygiene` CI jobs to the make targets.
- [ ] Prove each gate fails on a planted fault, then passes.
- [ ] Add the xtask gates XT-1 to XT-8, with unit tests named for the clauses.
- [ ] Add the CI job `xtask-ci` (XT-9).

## Test

Run the full gate set with the workstation default.

```bash
make check && make hygiene && make smoke
cargo xtask ci
```

Passing means all three exit 0. Then plant one fault per gate. A misformatted
file must fail `fmt-check`. A GPL crate must fail `deny`. A duplicate spec
number must fail `spec-check`. Remove each fault and confirm the gate passes
again.

## Notes

Nextest profile `ci` retries once and does not stop at the first failure.

Open question: add `cargo audit` and semver checks to `make check`. Today
`audit` runs weekly in CI only.

References:

- `cerebral-work/cortex@524a97b:Makefile`
- `codervisor/leanspec@ee122d6:skills/leanspec/SKILL.md`
