# AGENTS.md - rust-monorepo-template

> The single source of operational truth for people and autonomous agents.
> Rules are stated once, enforced by gates, and respected by machines.

Computers run on silicon, voltage, and memory barriers. They do not run on
optimism, vibes, or hand-waving. If code cannot prove its correctness at a
compiler gate or a verification harness, it does not land.

`CLAUDE.md` points here. There are no competing rulebooks.

---

## Architecture: Respect the Silicon

Software is fastest when it aligns with the hardware beneath it:

| Path | Role | Machine Sympathy & Boundary Rule |
|---|---|---|
| `crates/skel-domain` | Core Domain | **Zero I/O.** Pure mathematical entities, finite states, and repository traits. No filesystem, no sockets, no background threads, no async runtimes. Computes in pure memory. |
| `crates/skel-cli` | Host Adapter | The untrusted boundary. Parses CLI arguments (`clap`), handles standard streams, configures tracing, and invokes domain logic. |
| `xtask/` | Task Automation | Native Rust tooling (`spec-check`, `toolchain-check`, `pin-check`, `fetch-contracts`). Replaces fragile shell scripts with type-checked automation. |
| `specs/` | LeanSpec Registry | Formal architecture contracts. If a design cannot be written clearly, it cannot be compiled cleanly. |
| `vendor/leanspec/` | Specification Method | Vendored LeanSpec methodology (MIT). Untouched, unedited. |
| `.agents/skills/` | Agent Capabilities | Directed workflows for autonomous contributors. |
| `.claude/` | Harness Integration | Claude Code settings, permissions, and pre-execution hooks. |

**The Zero-I/O Invariant:** Crates depend strictly downward. `skel-cli` calls
`skel-domain`. `skel-domain` calls nobody. If `skel-domain` ever imports
`std::net`, `std::fs`, `tokio`, or `std::process`, `make smoke` immediately
terminates the build. Domain logic belongs in memory; I/O belongs at the gate.

---

## Invariants & Rules That Bind

1. **LeanSpec First:** Architecture before keystrokes. Any new concept, schema,
   or dependency begins as a spec in `specs/`. Run `make spec-check`.
2. **Host Throttling (`-j 2` Mandatory):** Silicon is shared. Physical hosts
   run concurrent workloads. All interactive and scripted `cargo` invocations
   MUST specify `-j 2` (configured via `JOBS ?= 2` in the Makefile). Never raise
   this on a shared workstation. Ephemeral CI runners pass `JOBS=10` because
   isolated pods have no neighbors to starve.
3. **Evidence Tiers Over Vibes:**
   - `verified`: A command ran and the exact output proves the claim (e.g. `make check` exit 0).
   - `reported`: Another agent or human stated it.
   - `heuristic`: Inferred from a structural pattern.
   - `asserted`: Unsubstantiated opinion.
   *Only `verified` satisfies a gate.* A grep that returns nothing is not proof
   of absence; read the actual table or config.
4. **Supply Chain Paranoia:** Wildcards are forbidden (`wildcards = "deny"` in
   `deny.toml`). We do not permit arbitrary internet code to land on our bare
   metal. Multi-target platform graphs audit Linux, macOS, and Windows.
5. **Signed Heritage:** Every commit MUST be signed (`chris@todie.io`, GPG/SSH).
   Never `--no-gpg-sign`. We know who wrote every byte.
6. **Conventional Commits:** Header format `<type>(scope): <desc>` under 72
   characters. No novel-length commit headers. Merge and revert commits are
   honored.
7. **Branch Model & Merge Commits:** Feature branches branch from `development`.
   PRs target `development`. Merge with explicit merge commits only (`--no-ff`).
   Never squash; squashing destroys the signed commit lineage.
8. **Release on Green:** `main` is the release branch. Promotion PRs carry
   `development` to `main` and merge automatically on green CI.
9. **No Secrets in Repo:** `gitleaks` enforces this on pre-commit and CI.
10. **7-Bit ASCII Purity:** All committed files must be 100% pure 7-bit ASCII.
    No hidden unicode gremlins or non-breaking spaces disrupting the parser.

---

## Contract Gates (`Justfile` & `Makefile`)

The `Justfile` (and its awk-parsing POSIX `Makefile` wrapper) is the single source of truth for all builds and verification.
Run `make help` or `just --list` for the categorized registry.

| Target | Contract & Evidence |
|---|---|
| `make build` | Builds the workspace (`-j $(JOBS)`). |
| `make fmt-check` | Enforces rustfmt styling without deviation. |
| `make sort-check` | Verifies `Cargo.toml` dependencies are alphabetically sorted. |
| `make clippy` | Lints workspace with all warnings treated as hard errors (`-D warnings`). |
| `make test` | Executes tests via `cargo-nextest` with parallel worker bounds. |
| `make doc-test` | Proves all documentation code blocks compile and execute. |
| `make powerset` | Tests all feature permutations via `cargo-hack --feature-powerset`. |
| `make semver` | Detects breaking public API changes against baseline (`cargo-semver-checks`). |
| `make msrv` | Verifies the code compiles against declared MSRV (`rust-version = "1.85"`). |
| `make deny` | Validates licenses, bans, advisories, and sources across multi-target triples. |
| `make check` | The push gate: `fmt-check clippy test doc-test deny`. |
| `make hygiene` | Repository cleanliness: `typos`, `sort-check`, `machete`, `spec-check`, `toolchain-check`, `pin-check`. |
| `make pin-check` | Verifies `specs/0004` dependency pin tables against `Cargo.lock`. |
| `make fetch-contracts` | Fetches pinned estate contracts into `.contracts/` (`contracts.lock`). |
| `make vendor` | Vendors third-party dependencies into `vendor/` for offline compilation. |
| `make vendor-check` | Verifies `vendor/` directory is in sync with `Cargo.lock`. |
| `make smoke` | Builds CLI, verifies binary output, and proves domain crate has zero I/O. |
| `make template-check` | Full verification + clones and renames a probe repo to prove template purity. |


## Skills

Skills are in `.agents/skills/<name>/SKILL.md`. Read the skill before you start that kind of work.

| Skill | Use it when | Path |
|---|---|---|
| `spec-driven-dev` | You add a concept, change the architecture, or start a feature. | `.agents/skills/spec-driven-dev/SKILL.md` |
| `release` | You cut a version. | `.agents/skills/release/SKILL.md` |
| `ci-triage` | A CI run is red. | `.agents/skills/ci-triage/SKILL.md` |
| `new-repo` | You make a repo from this template. | `.agents/skills/new-repo/SKILL.md` |
| `custody-run` | You run agent work under blackwall custody. | `.agents/skills/custody-run/SKILL.md` |

## Approval routes

- A lane merges its own feature PR into `development` when CI is green.
- A lane merges the promotion PR from `development` to `main` when CI is green (spec 0003).
- Secrets, Terraform, DNS and platform apps need the operator. Prepare the change and stop.
- Force-push to `main`, unsigned commits and merging with red CI are never allowed.
- Code owners are listed in `.github/CODEOWNERS`.

## Where things are

- Specs: `specs/README.md` is the registry. The method is in `vendor/leanspec/`.
- Docs: `docs/README.md`. Reference and runbooks are under `docs/`.
- Engineering notes: `engineering/`.
- Hooks and their bypasses: `.claude/README.md`.
- Contribution guide: `CONTRIBUTING.md`.
