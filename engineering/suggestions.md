# Suggestions

Numbered, evidence-backed suggestions for this repo. Add one entry per suggestion.

Use this shape:

```text
N. <what>
   Why: <a fact, with the date you measured it>
   Effort: S | M | L
   Owner lane: <lane>
   Deck click needed: yes | no
```

## Open

1. Forward termination signals to supervised children on Drain entry.
   Why: On 2026-10-12, audit of `crates/kd/src/teardown.rs` lines 134-140 showed `EnterDrain` only calls `notify_systemd("STOPPING=1")`. Supervised processes do not receive signals until `EnterEscalate` begins.
   Effort: S
   Owner lane: unsigned-gg/kd
   Deck click needed: no

2. Support optional child command execution in `kd init`.
   Why: On 2026-10-12, audit of `crates/kd/src/bin/kd.rs` lines 163-181 showed `run_init` tracks zero child processes, preventing use as a container entrypoint wrapping an application.
   Effort: S
   Owner lane: unsigned-gg/kd
   Deck click needed: no

3. Add `.agents/persona.md` to `.gitignore`.
   Why: On 2026-10-12, running `git status` on a fresh lane workspace reported `.agents/persona.md` untracked. Sibling repos (`cerebral-work/saga`, `cerebral-work/revenant`) ignore it in `.gitignore`.
   Effort: S
   Owner lane: unsigned-gg/kd
   Deck click needed: no

4. Implement `PidFd::wait_timeout` exit polling.
   Why: On 2026-10-12, `PidFd` implements only `open` and `send_signal`. Linux 5.3+ supports epoll and poll on pidfds for termination detection, avoiding global `waitpid` polling.
   Effort: M
   Owner lane: unsigned-gg/kd
   Deck click needed: no
