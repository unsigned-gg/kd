# kd

[![License: Apache-2.0 OR MIT](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](https://github.com/unsigned-gg/.github)

> Modern signal handling, process lifecycle, supervisor, and terminal hygiene framework in Rust.
> Dual-licensed Apache-2.0 OR MIT.

Computers run on silicon, voltage, and memory barriers. Signal handling and process
teardown require absolute rigor to prevent PID recycling races, avoid leaked zombies,
and protect operator terminals from stuck raw modes.

`kd` provides Linux and POSIX lifecycle primitives for containerized agent workloads,
subreapers, command runners, and terminal multiplexers:

- **Linux pidfd**: `PidFd` opens stable process descriptors to eliminate PID recycling races.
- **Child subreaper**: `set_child_subreaper` catches and adopts orphaned grandchild processes.
- **Zombie reaping**: Non-blocking `reap_zombies` cleans up terminated children.
- **PGID broadcast signaling**: Atomic negative-PGID kills (`kill(-pgid, sig)`).
- **Graceful 2-phase teardown**: `TeardownCoordinator` drives Drain → Escalate → ForceKill.
- **Terminal hygiene**: RAII `TerminalGuard` restores raw mode and cursor on drop or panic.

---

## What You Get

| Component | Path | Responsibility |
|---|---|---|
| **Domain Core** | `crates/kd-domain` | Pure signals, finite-state shutdown transitions, process status. Zero I/O. |
| **Systems Library & CLI** | `crates/kd` | Linux pidfd, subreaper, negative PGID kill, zombie harvest, 2-phase teardown, and RAII terminal hygiene. |
| **Repo Automation** | `xtask/` | Clean Rust automation (`spec-check`, `toolchain-check`, `pin-check`, `fetch-contracts`). No sloppy bash loops. |
| **Specifications** | `specs/` | LeanSpec registry. Architecture changes start as specs, not impromptu PRs. |
| **Quality Gates** | `Justfile`, `Makefile` | Modern `Justfile` with awk-parsing POSIX `Makefile` wrapper. Run `make help` or `just --list`. |
| **Tooling & CI** | `.github/`, `deny.toml`, `lefthook.yml`, `contracts.lock` | Strict git hooks, conventional commits, powerset checks, semver guards, contract pinning. |

---

## Forging a New Repository

### Option 1: GitHub Template
1. Click **Use this template** on GitHub.
2. Clone the new repository.
3. Run the rename pass:
   ```bash
   make new-repo NAME=<name>   # add REPO=<repo-name> if repo differs from crate prefix
   ```

### Option 2: cargo-generate
```bash
cargo generate --git git@github.com:unsigned-gg/kd.git --name <name>
cd <name>
git add -A && git commit -S -m "chore: initial commit from template"
make new-repo NAME=<name>
```

After renaming, run `make check` to verify the build, format, clippy, and
licenses pass on your host. Inspect the diff before pushing.

---

## Essential Navigation

- **Rules & Invariants:** Read [AGENTS.md](AGENTS.md) (the single source of truth for humans and agents).
- **Architecture Specs:** Browse [specs/README.md](specs/README.md).
- **Contributing:** Read [CONTRIBUTING.md](CONTRIBUTING.md).
- **All Targets:** Run `make help`.

## Principles

See the [unsigned-gg principles](https://github.com/unsigned-gg/.github) for the architectural doctrine and community standards.

