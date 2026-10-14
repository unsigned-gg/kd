---
status: in-progress
created: 2026-10-08
tags: [dependencies, licence, deny, pgt]
priority: high
depends_on: [0001-template-architecture]
---

# 0004-dependency-and-licence-policy

## Overview

This spec sets the licence of `kd` and the rules for external code.

`kd` is open-source and dual-licensed. `[workspace.package]` sets
`license = "Apache-2.0 OR MIT"`. The copyright holder is Christian M.
Todie / unsigned-gg. Contact: chris@todie.io.

## Design

### Licence rules

- Copyleft is denied. `deny.toml` uses the cargo-deny v2 schema, which has no
  `deny` list. A licence passes only when it is in `allow`. No GPL, AGPL or
  LGPL licence is in `allow`, so all of them fail.
- `cargo deny check` runs in `make check` and in the CI job `cargo-deny`.
- A new licence needs an operator decision before it enters the allow list.

### External dependencies

- Every external dependency is pinned and recorded before first use.
- Each `[workspace.dependencies]` entry has a comment that says why.
- `make pin-check` (`cargo xtask pin-check`) validates that the table below matches `Cargo.lock`.

### Pinned dependencies

| Crate | Version | Licence | Use | Pin |
|---|---|---|---|---|
| `anyhow` | 1.0.104 | MIT OR Apache-2.0 | Context-carrying errors in binaries and xtask. | Cargo.lock |
| `clap` | 4.6.7 | MIT OR Apache-2.0 | Command-line argument parsing for the CLI. | Cargo.lock |
| `libc` | 0.2.190 | MIT OR Apache-2.0 | POSIX and Linux syscalls for pidfd, prctl, and termios. | Cargo.lock |
| `serde` | 1.0.229 | MIT OR Apache-2.0 | Serialization and deserialization derive. | Cargo.lock |
| `serde_json` | 1.0.151 | MIT OR Apache-2.0 | Structured JSON output for CLI and tests. | Cargo.lock |
| `sha2` | 0.11.0 | MIT OR Apache-2.0 | Cryptographic hash for contract pinning and xtask. | Cargo.lock |
| `thiserror` | 2.0.21 | MIT OR Apache-2.0 | Typed domain error enums. | Cargo.lock |
| `tracing` | 0.1.44 | MIT | Structured logging facade. | Cargo.lock |
| `tracing-subscriber` | 0.3.23 | MIT | Log formatting and RUST_LOG filtering. | Cargo.lock |

### pgt

pgt is not a dependency of any template crate. The root `Cargo.toml` holds the
line below as a comment. A repo that needs pgt removes the comment mark:

```toml
pgt-core = { git = "https://github.com/cerebral-work/pgt", rev = "54d90f5" }
```

A crate that needs pgt uses a git dependency by rev. The rev is never a branch.
`make pgt-install` installs the CLI into `.tools/bin`. That directory is
gitignored. CI mints a read token only in jobs that build a crate with this
dependency. The token comes from the reverie GitHub App pattern.

The licence gap is Patrick Gray's to settle. Until he does, nobody
redistributes pgt code or copies it into this repo.

### rig

Decision, Christian, 2026-10-08: rig is evaluated and pinned at 0.44.0 (MIT).
The template does not include it. A repo adopts it when it needs model calls.
It calls models through the LiteLLM gateway, not vendor APIs.

### CI tools

A tool that a workflow downloads at run time is an external dependency too.
Pin it by version and sha256 and record it here.

| Tool | Version | Licence | Use | Pin |
|---|---|---|---|---|
| `gh` (cli/cli) | 2.101.0, published 2026-09-15 | MIT | `release.yml`: the release-plz action calls `gh` in its git-config step. The `ci-rust` image has no `gh`. | sha256 for `linux_amd64` and `linux_arm64` in `release.yml`. Checked 2026-10-08 against `gh_2.101.0_checksums.txt` and a local `sha256sum` of each tarball. |

The `gh` step is temporary. Remove it when the `ci-rust` image ships `gh`
(OPS-1636).

`cargo xtask tools` builds the gate tools from crates.io (spec 0002, XT-2 and
XT-3). The version table lives in `xtask/src/gates.rs`. `cargo install
--locked` builds each one from its published lock file, and crates.io checks
every crate's sha256. They go into `.tools/`, which is gitignored.

| Tool | Version | Licence | Use | Pin |
|---|---|---|---|---|
| `cargo-deny` | 0.19.9, published 2026-06-15 | MIT OR Apache-2.0 | `cargo xtask deny` | crates.io version, `--locked`. The `ci-rust` image bakes the same version. |
| `typos-cli` | 1.49.0, published 2026-08-03 | MIT OR Apache-2.0 | `cargo xtask typos` | crates.io version, `--locked`. The `ci-rust` image bakes the same version. |
| `cargo-machete` | 0.9.1, published 2025-08-15 | MIT | `cargo xtask machete` | crates.io version, `--locked`. |
| `cargo-nextest` | 0.9.104, published 2025-09-15 | Apache-2.0 OR MIT | `cargo xtask test` | crates.io version, `--locked`. |

### Vendoring & offline builds

For airgapped environments or offline reproducibility:
- `make vendor` (or `cargo vendor vendor/crates`) vendors all crates.io dependencies
  into `vendor/crates`.
- `make vendor-check` proves `vendor/crates` matches `Cargo.lock`.
- `.cargo/config.toml` includes a commented source-replacement block to activate
  offline vendored builds.
- LeanSpec methodology stays vendored under `vendor/leanspec` (MIT).

### Contracts & upstream schemas

Cross-monorepo interfaces, protobufs, and schemas from upstream estate repositories:
- Pinned in `contracts.lock` by sha256 checksum and 40-character commit hash.
- Fetched safely on demand into `.contracts/` via `make fetch-contracts` (`cargo xtask fetch-contracts`).
- Verified against lockfile checksums; fails closed on any mismatch.
- `.contracts/` is gitignored; contracts are never committed unverified.
- `make pin-check` (`cargo xtask pin-check`) validates that dependency pin tables
  bind the lockfiles and CI tool versions.

## Plan

- [x] Write `deny.toml` with the licence allow list. Copyleft fails by omission (v2 schema).
- [x] Add the commented pgt example to the root `Cargo.toml`.
- [x] Add `make pgt-install`.
- [x] Add a why-comment to every workspace dependency.
- [ ] Ask Patrick for a pgt licence statement. Track it outside this repo.

## Test

Plant a copyleft crate, then run the gate. It must fail. Remove it and run the
gate again. It must pass.

```bash
cargo deny check licenses sources
```

Passing means exit 0 on the clean tree. A GPL-3.0 crate in the tree makes it
exit non-zero. A git source other than pgt also makes it exit non-zero.

## Notes

Open question: does pgt get a licence or stay internal-only? The answer
decides whether instantiated repos may publish a crate that depends on it.

References:

- `cerebral-work/pgt@54d90f5:Cargo.toml`
- `cerebral-work/cortex@524a97b:deny.toml`
