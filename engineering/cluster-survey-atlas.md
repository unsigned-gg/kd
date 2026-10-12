# Cluster survey atlas

This page maps where each signal about this repo's work lives on the estate. It is a map of where to look. It does not replace a live probe. Date every number you add.

Per `~/.agents/sops/engineering-subdir-and-subagent-gruntwork.md`, a subagent fills this page from a surface file. A read-only probe fills the gaps.

## Observability stores

Not probed yet. This repo is a template. It runs no service.

## Dashboards

Not probed yet.

## Telemetry paths

Not probed yet.

## Data lakes and archives

Not probed yet.

## CI and runners

Known from the repo contract (2026-10-08):
- Spec 0005 (`specs/0005-ci-cache-and-runners.md`) specifies runner lanes and caches.

Measured from workflow inspection (2026-10-12):
- `.github/workflows/ci.yml` runs on GitHub-hosted `ubuntu-latest` runners.
- Toolchains managed via `dtolnay/rust-toolchain@stable` with `rustfmt` and `clippy`.
- Caching powered by `Swatinem/rust-cache@v2`.
- Active CI jobs: `fmt & clippy`, `test & doc-test`, `smoke (zero-i/o domain invariant)`, `spec-check`, `labeler`.
- Host throttling invariant: Cargo operations in Makefile and CI execute with `-j 2` to preserve host resources.

Not probed yet: live runner capacity, cache hit rates, the R2 sccache bucket.

## What this atlas does not hold

- Live state. Probe before you act.
- Secrets. Names only, never values.

## Known traps

None recorded yet.

## Not probed yet

Every section above that says "Not probed yet".
