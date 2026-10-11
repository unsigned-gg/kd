//! `kd` - Modern signals framework, process supervisor, and terminal hygiene library.
//!
//! Provides Linux and POSIX lifecycle primitives for containerized agent workloads,
//! command runners, and terminal tools:
//!
//! - **Linux pidfd**: `PidFd` opens stable process descriptors to eliminate PID recycling races.
//! - **Child subreaper**: `set_child_subreaper` catches and adopts orphaned grandchild processes.
//! - **Zombie reaping**: Non-blocking `reap_zombies` cleans up terminated children.
//! - **PGID broadcast signaling**: Atomic negative-PGID kills (`kill(-pgid, sig)`).
//! - **Graceful 2-phase teardown**: `TeardownCoordinator` drives Drain → Escalate → ForceKill.
//! - **Terminal hygiene**: RAII `TerminalGuard` restores raw mode and cursor on drop or panic.
//!
//! # Example
//!
//! ```
//! use kd::{Signal, TerminalGuard, set_child_subreaper, trap_default_termination};
//!
//! // Establish terminal hygiene guard before any TTY modifications
//! let guard = TerminalGuard::init().expect("terminal hygiene guard");
//!
//! // Enable child subreaper to adopt and reap orphaned descendant processes
//! let _ = set_child_subreaper(true);
//!
//! // Listen for termination signals safely via non-blocking self-pipe
//! let receiver = trap_default_termination().expect("trap signals");
//! ```

#![deny(missing_docs)]

pub mod signal;
pub mod supervisor;
pub mod teardown;
pub mod terminal;

// Re-export pure domain types from kd-domain
pub use kd_domain::{
    ChildState, ExitStatus, KdError, KillTarget, ProcessGroupId, ProcessId, ProcessRecord,
    ShutdownAction, ShutdownConfig, ShutdownPhase, ShutdownStateMachine, Signal, SubreaperPolicy,
    SupervisionPolicy,
};

// Re-export system primitives
pub use signal::{trap_default_termination, trap_signals, SignalReceiver};
pub use supervisor::{
    freeze_cgroup, is_child_subreaper, kill_cgroup, kill_process_group, reap_zombies,
    send_signal_safe, set_child_subreaper, unfreeze_cgroup, PidFd,
};
pub use teardown::{notify_systemd, TeardownCoordinator};
pub use terminal::TerminalGuard;
