# Roadmap improvements

The engineering-side roadmap in three horizons. Every item has a "what" and a "done when". Mark every item "proposed" until the owner accepts it.

Science and paid runs stay with their approver. They do not belong here.

## Now

1. **Spec 0004 dependency pin synchronization** (proposed).
   What: Synchronize line 44 of `specs/0004-dependency-and-licence-policy.md` to `sha2 0.11.0` following the merge of PR #1 and PR #6.
   Done when: `cargo run -p xtask -j 2 -- pin-check` executes with exit code 0 against `Cargo.lock`.

2. **Signal delivery on graceful drain entry** (proposed).
   What: Forward the triggering signal (`SIGTERM`, `SIGINT`, or `SIGHUP`) to all tracked process groups and child processes immediately upon entering `Drain` phase in `TeardownCoordinator::execute_action`.
   Done when: An integration test confirms that triggering drain delivers `SIGTERM` to a running child at time zero, and the coordinator transitions to `Terminated` cleanly upon child exit.

3. **Container init child command wrapping** (proposed).
   What: Add optional trailing command argument support to `kd init` (`kd init [--drain-ms N] [--] <cmd> [args...]`). When provided, spawn and supervise `<cmd>` with subreaper enabled and forward signals to it.
   Done when: `kd init -- /bin/sh -c "exit 0"` executes the command, reaps any background grandchild processes, and exits with code 0.

4. **PidFd termination waiting via poll** (proposed).
   What: Implement `PidFd::wait_timeout(&self, timeout: Duration) -> io::Result<Option<ExitStatus>>` using `poll(2)` on Linux kernel >= 5.3 to enable deterministic, race-free single process termination detection.
   Done when: A unit test spawns a child process and waits for its exit descriptor using `PidFd::wait_timeout` without relying on global `SIGCHLD` delivery.

## Next

5. **Cgroup v2 unified supervisor integration** (proposed).
   What: Integrate cgroup v2 freezing and termination into `TeardownCoordinator` for workloads configured with cgroups. Freeze cgroups prior to force-kill escalation to prevent process escape.
   Done when: An integration test proves a child process spawning detached forks inside a supervised cgroup cannot leave running processes after coordinator teardown.

6. **Async-signal-safe event log buffer** (proposed).
   What: Add a bounded, lock-free, zero-allocation ring buffer recording signal receipts, zombie reap events, and phase transitions in memory. Expose through `kd status --json` or stdout upon exit.
   Done when: `kd run --record-log` outputs valid JSON recording every lifecycle event, and no heap allocation occurs within signal handler context.

7. **PTY master and slave session multiplexing** (proposed).
   What: Add pseudo-terminal (PTY) allocation and window change (`SIGWINCH`) forwarding to `kd run`. Maintain terminal raw mode and cursor visibility across agent subshells and interactive multiplexers.
   Done when: `kd run --pty <cmd>` allocates a PTY, forwards `SIGWINCH` resize events, and restores terminal settings upon exit or panic.

## Later

8. **Linux Landlock LSM unprivileged sandboxing** (proposed).
   What: Introduce optional Landlock ABI v1-v5 path restriction rulesets to `kd run` (`kd run --sandbox-ro <path> --sandbox-rw <path>`), restricting filesystem access before `execve` while preserving pidfd supervisor authority.
   Done when: A supervised command attempting unauthorized filesystem writes fails with permission denied, while allowed paths remain accessible.

9. **eBPF process tree audit probe** (proposed).
   What: Create an optional `kd-ebpf` crate attaching to `sched_process_fork`, `sched_process_exec`, and `sched_process_exit` tracepoints to audit grandchild processes and detect namespace unshare escapes.
   Done when: The probe detects and reports any process escaping process group boundaries or breaking out of PID namespaces.

10. **Cortex mesh coordination and drain heartbeat** (proposed).
    What: Wire `TeardownCoordinator` into the local Cortex mesh IPC socket to publish graceful drain intent and state transitions to peer agents and services before terminating.
    Done when: `kd` emits a structured drain message to the local agent socket before initiating child teardown.
