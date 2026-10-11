# Contributing

> Welcome. We build for the bare metal, respect CPU cache hierarchies, and
> don't run our infrastructure on optimism.

The complete operational constitution lives in [AGENTS.md](AGENTS.md). If you
are an autonomous agent or a human engineer with an editor, read that file first.
This document is the quick-start guide for biological contributors.

---

## Machine Setup (Zero Excuses)

1. **The Pinned Toolchain:** Install `rustup`. It detects `rust-toolchain.toml`
   and configures the exact compiler channel (Rust 1.94.1). Don't invent your
   own toolchain; determinism begins at compiler invocation.
2. **The Verification Arsenal:**
   ```bash
   cargo install cargo-nextest cargo-deny cargo-hack cargo-machete cargo-sort typos-cli --locked -j 2
   ```
   *(Notice the `-j 2`. Your CPU will thank you, and your desktop will remain responsive.)*
3. **Hardware-Enforced Git Hooks:**
   ```bash
   lefthook install
   ```
   *Lefthook runs gitleaks, formatting, dependency sorting, typos, and affected
   test sweeps before your code ever hits the network.*
4. **Compile-Time Caching (Optional but Civilized):**
   Add sccache to your local `~/.cargo/config.toml` (never commit this to the repo):
   ```toml
   [build]
   rustc-wrapper = "sccache"
   ```
5. **Verify Clean Baseline:**
   ```bash
   make check
   make hygiene
   ```

---

## The Field Manual: Rules in 60 Seconds

- **Think Before Typing (LeanSpec):** Major concepts or architectural modifications
  start in `specs/`. If you cannot specify the state machine in text, the compiler
  will not save you. Run `make spec-check`.
- **Throttling is Mandatory (`-j 2`):** Always pass `-j 2` to `cargo` on shared
  workstations. Do not starve the host.
- **Evidence Tiers Over Vibes:** When opening a PR, fill the `Verified` section
  with the exact commands and output. "Works on my machine" is an unverified assertion.
- **Conventional Commits & Clean Lineage:**
  `<type>(scope): <desc>` under 72 characters. Cryptographically sign every
  commit. No unsigned commits, no exceptions.
- **No Squashing:** We merge feature branches with explicit merge commits to
  preserve cryptographic signatures and bisectability.
- **Keep the Changelog Honest:** Add an entry under `## [Unreleased]` in
  `CHANGELOG.md` for any user-visible change in the same commit.
- **Run the Pre-Push Gauntlet:**
  ```bash
  make check && make hygiene && make smoke
  ```

---

## Conduct & Security

We take code security and host isolation seriously. Read [SECURITY.md](SECURITY.md)
and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

