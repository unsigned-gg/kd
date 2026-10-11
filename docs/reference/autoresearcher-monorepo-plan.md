## Goal Description
The `autoresearcher` repository has outgrown its origins as a pure GitBook-based research markdown site. It now houses multiple languages, runtimes, and an active autonomous daemon (`researchd`) and CLI.

This plan will restructure the monorepo into clear language/runtime directories (`rust/`, `python/`, `docs/`), re-organize the scattered markdown documentation and research files (including landing the D. Tolnay Rust research), and update the `README.md` to accurately reflect the repo's new identity and architecture.

## User Review Required
> [!WARNING]
> Moving the `Cargo.toml` and Rust crates into a `rust/` subdirectory will change how `cargo` commands are run from the root, although the root Makefile/Justfile will abstract this away. Are you okay with the Cargo workspace root living at `rust/Cargo.toml`?

> [!IMPORTANT]
> The current python files are scattered across `scripts/` and `tests/`. This plan proposes grouping them under a unified `python/` directory.

## Open Questions
> [!NOTE]
> 1. Do you want the GitBook configuration (`gitbook-docs.yaml`) updated to point to the new `docs/` directory, or is GitBook syncing no longer the primary focus?
> 2. For the "Rust research we found" (`records/dtolnay/`), I plan to land these into `docs/research/rust-ecosystem/` (or similar). Does that location work for you?

## Proposed Changes

### 1. Language and Runtime Directories
We will establish dedicated top-level directories for runtimes to prevent namespace pollution.

#### [NEW] `rust/`
We will move all Rust-specific configuration and crates here.
* Move `crates/` -> `rust/crates/`
* Move `xtask/` -> `rust/xtask/`
* Move `Cargo.toml`, `Cargo.lock`, `rustfmt.toml`, `clippy.toml`, `rust-toolchain.toml`, `deny.toml` -> `rust/`
* Update `Cargo.toml` to point to `rust/crates/*` and `rust/xtask`.

#### [NEW] `python/`
We will move Python automation and testing here.
* Move `scripts/` -> `python/scripts/`
* Move `tests/` (which contains python test files) -> `python/tests/`

### 2. Documentation and Research Consolidation
We will move all top-level research folders into a unified `docs/` directory so they don't clutter the root directory.

#### [NEW] `docs/`
* Move `agentic-systems/`, `business-development/`, `communication/`, `infra/`, `ma-cc-on-cygnus/`, `mcp-honeypot/`, `overview/`, `repos/`, `reverie-specs/`, `slurm-on-cygnus/`, `specs/`, `standards/`, `swarm-control/`, `templates/` into `docs/`.

#### Landing the Rust Research
* Move `records/dtolnay/` -> `docs/research/rust-ecosystem/dtolnay/` (or similar). This appropriately "lands" the research alongside the rest of the documentation.

### 3. Repository Configuration Updates
#### [MODIFY] `README.md`
Rewrite the README to introduce `autoresearcher` as a polyglot monorepo containing `researchd`, Python automation, and the Cerebral research records, rather than just a static site. Update the Mermaid diagram to reflect the new structure.

#### [MODIFY] `Makefile` and `Justfile`
Update the task runner commands to explicitly delegate to `cd rust && cargo ...` and `cd python && ...`.

## Verification Plan

### Automated Tests
* Run `make check` and `make test` to verify that all configurations and imports are still valid.
* Run `make spec-check` to ensure the directory moves didn't break static checks.

### Manual Verification
* Ensure `cargo check` inside the `rust/` directory succeeds.
* Verify the new `README.md` renders the Mermaid diagram correctly.
