# rust-monorepo-template

> Cold-forged Rust monorepo scaffold with strict machine sympathy, zero-cost
> boundaries, and zero patience for web-scale bloat.

Computers run on silicon, voltage, and memory barriers. They do not run on
optimism, vibes, or 400-crate dependency trees designed to pad a byte slice.

This repository is the canonical template for Rust services and tooling across
the unsigned estate. It compiles fast, enforces uncompromising architecture
discipline, and builds itself before it lets you write a single bug.

---

## The Philosophy: Respect the Metal

1. **Host Throttling (`-j 2`):** Silicon is shared. The Makefile locks
   compilations to `-j 2` by default. Thrashing all CPU cores to expand a macro
   tree while starving your desktop is poor manners. CI gets the full pipe; your
   workstation stays responsive.
2. **The Zero-I/O Domain (`crates/skel-domain`):** Pure logic lives isolated
   from the kernel. No sockets, no disk reads, no surprise async runtimes.
   Adapters handle the real world; domain logic stays testable in nanoseconds.
   `make smoke` verifies no I/O imports pollute the domain.
3. **Evidence Tiers Over Vibes:** We do not say "tests pass." We run
   `make check` and quote the exit code. If a command cannot reproduce a claim,
   the claim does not exist.
4. **Paranoid Supply Chain:** `wildcards = "deny"` in `deny.toml`. Multi-target
   license verification across Linux, macOS, and Windows. We like knowing what
   instructions run on our bare metal.
5. **Signed Heritage:** Every commit is cryptographically signed. No squashing.
   Clean, linear history where every transition is tracked.

---

## What You Get

| Component | Path | Responsibility |
|---|---|---|
| **Domain Core** | `crates/skel-domain` | Pure entities, finite-state transitions, repository ports. Zero I/O. |
| **CLI Adapter** | `crates/skel-cli` | Argument parsing via `clap`, structured logging, stderr/stdout hygiene. |
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
cargo generate --git git@github.com:cerebral-work/rust-monorepo-template.git --name <name>
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

