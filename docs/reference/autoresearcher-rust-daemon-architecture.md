# Autoresearcher Rust Daemon Architecture Plan

This plan outlines the steps to elevate `researchd` to a production-grade daemon, integrating Linux daemon best practices, robust low-level synchronization, and idiomatic patterns derived from Google's Comprehensive Rust curriculum and the `cerebral-docs` substrate designs.

## 1. Daemonization & systemd Integration

Following the `rust-linux-daemons` guide from `cerebral-docs`, `researchd` must integrate closely with systemd:

**Implementation Steps:**
- **`sd-notify` Integration:** Use the `sd-notify` crate with `Type=notify`. Signal `READY=1` only when the database pool is connected and the API server is bound. Send `STOPPING=1` on graceful shutdown. Add `WATCHDOG=1` integration tied to the main work loop (not a free-running timer, to avoid the watchdog trap).
- **Standalone Daemonization:** For environments that lack `systemd` or require `researchd` to run in a purely detached background state (traditional sysvinit style), use the `daemonize` crate (specifically the `cloudflare/daemonize` fork, which includes additional features used in tools like `boringtun`). This will handle double-forking, detaching from the controlling terminal, and redirecting standard streams safely.
- **Graceful Upgrades (`shellflip`):** Use the `cloudflare/shellflip` crate to implement zero-downtime process restarts and binary upgrades. It handles transferring state (like listening sockets) between the old and new process via IPC, functioning similarly to Go's `tableflip`, which complements or replaces systemd socket activation for state handoff.
- **Socket Activation:** Use `sd_listen_fds` to receive listening sockets from systemd, allowing zero-downtime restarts. Add `FDSTORE=1` if we need to persist state across binary upgrades.
- **Journald Logging:** The estate standard dictates `tracing` and `tracing-subscriber`. Journald will natively capture stdout/stderr without requiring in-process file rotation (`tracing-appender` only if file sinks are explicitly requested).

## 2. Agent Substrate & Durable Workflows

Autoresearch runs complex workflows across Python scripts, subagents, and Git. To make this production-grade, we will implement the Cerebral Agent Substrate (CAS) Engine:

**Implementation Steps:**
- **Durable Workflows (`autumn-harvest`):** Replace the current in-memory Saga orchestrator with `autumn-harvest`. It provides event-sourced, Postgres-backed durable workflow execution, guaranteeing that agent steps can survive process crashes and node reboots without losing state or repeating side effects.
- **Fast Local Storage (`horton`):** Integrate `horton`, a zero-dependency, zero-allocation LSM-tree KV store, for high-performance localized caching of research artifacts and LLM context vectors prior to global Postgres sync.
- **CRM & Governance (`wheelhorse`):** Implement the Wheelhorse pipeline—a reversibility-gated business development and CRM layer. This adds mandatory undo timers and strict operator approval gates before any outward-facing actions (like publishing or outreach) are taken by the agents.
- **Unix Domain Sockets (UDS):** Migrate local script-to-daemon communication to `tokio::net::UnixListener` instead of TCP, enforcing local-only access.
- **Atomic Worktree Rollbacks (`gitoxide`):** Adopt the `gix` (gitoxide) ODB pattern from `cas-substrate`. Before any destructive tool action, create an atomic worktree snapshot in memory (under 2ms). If a test-time compute branch fails or introduces unrecoverable errors, roll back the index and worktree state without polluting the disk or reflog.
- **Filesystem Locks (`flock`):** Use `fs4` for RAII-based file locks over critical shared resources.

## 3. Sandboxing & Priority Control

We must contain runaway scripts and AI-generated code, utilizing the tools established in `rust-native-agent-substrate`:

**Implementation Steps:**
- **In-Process Virtual Bash (`bashkit`):** For executing simple AI-generated shell commands and scripts, use the pure Rust `bashkit` interpreter. It provides POSIX shell compliance without ever invoking `fork` or `exec` system calls, completely isolating execution within an `InMemoryFs` or `OverlayFs`. This eliminates over 280 threat models and allows us to serialize/snapshot running scripts to pause or resume them effortlessly.
- **Landlock V2:** Use the `landlock` crate to sandbox the host `researchd` runner. Drop filesystem access rights to only the strictly required paths (`RESEARCHD_DOCS_ROOT`, `~/.cortex/`) after initialization, protecting against any host vulnerabilities.
- **eBPF Tracepoints (`Aya`):** Introduce an Aya eBPF tracepoint program (`#![no_std]`). When `researchd` does need to spawn heavy host toolchains like `cargo` or `make` outside of `bashkit`, use the eBPF hook to automatically clamp process priority to `nice 19` (autonice) and audit system calls.
- **Worker Isolation (Heavy Workloads):** For executing untrusted guest code that requires real Linux kernel interfaces (like compiled binaries), implement a `containerd-shim` (via `containerd-client`) so that containerd supervises the workload. Alternatively, embed `hyperlight` if micro-VM isolation is preferred for guest AI code.

## 4. Google's Comprehensive Rust Idioms & Substrate Patterns

Applying Google's Rust curriculum alongside `cerebral-docs` designs:

**Implementation Focus Areas:**
- **Typestate Pattern & PhantomData:** Encode the lifecycle of a research run using `PhantomData` (e.g., `Run<Draft>`, `Run<Verified>`). Invalid state transitions, like pushing a draft without a verification receipt, become compile-time errors.
- **Error Handling:** Domain libraries (`research-domain`) use `thiserror` for precise variants. The daemon binary (`researchd`) uses `anyhow` for context propagation.
- **MAD Stack UI:** If `researchd` needs an operator dashboard, use the MAD stack: `maud::html!` for zero-allocation compile-time HTML rendering, `axum` for serving, and `htmx` for dynamic DOM replacement.

## Execution Phasing

1. **Phase 1 (Foundations):** Wire `sd-notify`, Unix Domain Sockets, and `tracing` structured logging. Refactor error handling (`thiserror`/`anyhow`).
2. **Phase 2 (Durability & Rollback):** Integrate `gitoxide` for atomic in-memory worktree snapshots and rollbacks.
3. **Phase 3 (Sandboxing & Priority):** Implement Landlock V2 jails and `Aya` eBPF autonice hooks for sub-processes.
4. **Phase 4 (Containment):** Transition execution to a `containerd-shim` or `hyperlight` micro-VM model.

Does this accurately capture the capabilities you want to pull from the cerebral-docs repository?
