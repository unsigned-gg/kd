---
status: complete
created: 2026-10-11
tags: [pidfd, subreaper, process, supervisor, signals]
priority: critical
---

# SPEC-0001-process-supervision

## Overview

Modern Linux process lifecycle management, signal multiplexing, and container
process tree supervision implemented in pure Rust.

Classical POSIX process supervision suffers from fundamental race conditions:

1. PID recycling race: `kill(pid, sig)` operates on an integer `pid_t`. If the
   target terminates and its PID is reassigned before signal dispatch, the
   signal strikes an innocent, recycled process.
2. Zombie accumulation: When intermediate parent processes exit, child processes
   become orphaned. In container runtimes, orphaned processes reparent to PID 1.
   Without an active reaper, zombies accumulate and exhaust kernel PID tables.
3. Signal coalescing: Standard POSIX signals do not queue. Multiple rapid exits
   coalesce into a single `SIGCHLD`, causing naive 1:1 handlers to miss dead
   children.
4. Terminal corruption: Supervised CLI processes dying unexpectedly leave the
   operator terminal in raw mode with hidden cursors.

`kd` eliminates these hazards through Linux `pidfd` descriptors, child
subreaper configuration (`PR_SET_CHILD_SUBREAPER`), non-blocking burst zombie
harvesting, negative PGID signaling, and RAII terminal restoration.

## Design

### Mathematical Formalization of 2-Phase Shutdown

The shutdown coordinator is formalized as a deterministic finite-state
automaton with discrete-time transitions:

$$M = (S, \Sigma, \delta, s_0, F)$$

Where:
- State set $S = \{ \text{Running}, \text{Drain}, \text{Escalate}, \text{ForceKill}, \text{Terminated} \}$
- Initial state $s_0 = \text{Running}$
- Final states $F = \{ \text{Terminated}(c) \mid c \in \mathbb{Z} \}$
- Alphabet $\Sigma = \text{Signals} \cup \text{Ticks}(\Delta t) \cup \text{Events}$

The state configurations carry timing and status parameters:
- $\text{Drain}(t_d, e_d, r)$ with grace period $t_d$, elapsed time $e_d$, reason $r$.
- $\text{Escalate}(t_e, e_e)$ with timeout $t_e$, elapsed time $e_e$.
- $\text{ForceKill}(t_k, e_k)$ with timeout $t_k$, elapsed time $e_k$.
- $\text{Terminated}(c)$ with final exit code $c$.

The transition function $\delta: S \times \Sigma \to S \times \mathcal{A}$ defines:

1. Initial termination signal ($s \in \{ \text{SIGTERM}, \text{SIGINT}, \text{SIGHUP}, \text{SIGQUIT} \}$):
   $$\delta(\text{Running}, s) = (\text{Drain}(t_d, 0, s), \text{EnterDrain}(t_d, s))$$
2. Grace period drain expiration:
   $$\delta(\text{Drain}(t_d, e_d, r), \text{Tick}(\Delta t)) = \begin{cases}
     (\text{Drain}(t_d, e_d + \Delta t, r), \text{None}) & \text{if } e_d + \Delta t < t_d \\
     (\text{Escalate}(t_e, 0), \text{EnterEscalate}(t_e)) & \text{if } e_d + \Delta t \ge t_d
   \end{cases}$$
3. Escalation timeout expiration:
   $$\delta(\text{Escalate}(t_e, e_e), \text{Tick}(\Delta t)) = \begin{cases}
     (\text{Escalate}(t_e, e_e + \Delta t), \text{None}) & \text{if } e_e + \Delta t < t_e \\
     (\text{ForceKill}(t_k, 0), \text{EnterForceKill}) & \text{if } e_e + \Delta t \ge t_e
   \end{cases}$$
4. ForceKill safety timeout expiration:
   $$\delta(\text{ForceKill}(t_k, e_k), \text{Tick}(\Delta t)) = \begin{cases}
     (\text{ForceKill}(t_k, e_k + \Delta t), \text{None}) & \text{if } e_k + \Delta t < t_k \\
     (\text{Terminated}(137), \text{Complete}(137)) & \text{if } e_k + \Delta t \ge t_k
   \end{cases}$$
5. Double-SIGINT fast-track:
   $$\delta(\text{Drain}(t_d, e_d, r), \text{SIGINT}) = (\text{ForceKill}(t_k, 0), \text{EnterForceKill})$$
6. Clean drain completion:
   $$\delta(\text{Phase}, \text{AllReaped}) = (\text{Terminated}(c_{\text{default}}), \text{Complete}(c_{\text{default}})) \quad \forall \text{Phase} \in \{ \text{Drain}, \text{Escalate}, \text{ForceKill} \}$$

### Core Invariants

1. **Invariant I1 (Subreaper Adoption Catchment):**
   Setting `PR_SET_CHILD_SUBREAPER` ensures that for any descendant process
   $p \in \text{Descendants}(S)$, when $\text{parent}(p)$ terminates:
   $$\text{parent}(p) \leftarrow S$$
   Orphaned grandchild processes are captured by $S$ rather than leaking to PID 1.

2. **Invariant I2 (Exhaustive Non-Blocking Harvesting):**
   Reaping uses an unconditional loop over `waitpid(-1, &status, WNOHANG)`.
   Upon loop exit with $pid \le 0$, the set of dead child processes in the kernel
   is empty:
   $$|Z| = 0$$
   This prevents lost zombie status when multiple `SIGCHLD` signals coalesce.

3. **Invariant I3 (PID Race Elimination via PidFd):**
   Holding a valid `PidFd` guarantees that signal delivery targets the exact
   kernel `struct pid` instance allocated at spawn:
   $$\forall t > t_{\text{death}}(P), \quad \text{pidfd\_send\_signal}(\text{fd}_P, s) = \text{ESRCH}$$
   Signals are never misdelivered to recycled processes.

4. **Invariant I4 (Atomic Process Group Broadcast):**
   Group signaling executes `kill(-pgid, sig)`. The negative PGID targets all
   processes in group $G$ atomically, preventing escaped subprocesses.

5. **Invariant I5 (RAII Terminal Guard):**
   `TerminalGuard` records terminal mode via `tcgetattr`. The `Drop` handler
   restores original mode via `tcsetattr(TCSAFLUSH, ...)` and emits cursor-on
   ANSI sequences (`\x1b[?25h`). Terminal hygiene holds even on panic.

6. **Invariant I6 (Zero I/O Domain Separation):**
   Crate `kd-domain` contains zero syscalls, zero standard filesystem or network
   imports, and zero async runtimes. All system effects are isolated to crate `kd`.

### Error Contracts

| Condition | Syscall Return | Contract & Reaction |
|---|---|---|
| Target process already reaped | `ESRCH` | Idempotent success during teardown; target is dead. |
| No child processes exist | `ECHILD` | Reaping loop terminates cleanly; harvest complete. |
| Syscall interrupted | `EINTR` | Self-pipe handler consumes signal; poll retries. |
| Permission denied | `EPERM` | Propagates typed `KdError::PermissionDenied`. |
| Out of file descriptors | `EMFILE`/`ENFILE` | Fallback to safe POSIX `kill(2)` with warning log. |
| Subreaper unsupported | `EINVAL` | Surface capability error; container environment check. |

## Plan

- [x] Zero-I/O domain types and shutdown state machine in `crates/kd-domain`.
- [x] Linux `PidFd` wrapper (`pidfd_open` / `pidfd_send_signal`) in `crates/kd`.
- [x] `PR_SET_CHILD_SUBREAPER` configuration and querying.
- [x] Non-blocking `reap_zombies` burst loop.
- [x] Self-pipe async-signal-safe receiver (`pipe2` + `O_NONBLOCK` + `O_CLOEXEC`).
- [x] Teardown coordinator orchestrating Drain -> Escalate -> ForceKill.
- [x] RAII `TerminalGuard` terminal mode restoration.
- [x] CLI commands: `demo`, `run`, `init`, `signals`, `reap`.
- [x] LeanSpec contract verification and `-j 2` CI pipeline.

## Test

Run unit tests, integration tests, and doc tests across the workspace:

```bash
cargo test --workspace --locked -j 2
```

To verify the shutdown sequence and terminal hygiene demo:

```bash
cargo run -q -p kd --locked -j 2 -- demo | grep -q '"status": "done"'
```

## Notes

- Linux `pidfd_open` requires Linux kernel >= 5.3 (syscall 434).
- `pidfd_send_signal` requires Linux kernel >= 5.1 (syscall 424).
- On platforms without pidfd support, `send_signal_safe` gracefully falls back
  to standard POSIX `kill(2)`.
- Self-pipe write fd is stored in an `AtomicI32` and written using async-signal-safe
  `write(2)`. No memory allocation occurs in the signal handler.
- Conforms to LeanSpec architecture methodology.
