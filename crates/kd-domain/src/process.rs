//! Strongly-typed process identifiers, exit statuses, and supervision policies.
//!
//! All structures in this module are pure data with zero I/O side effects.

use core::fmt;
use serde::{Deserialize, Serialize};

use crate::signal::Signal;

/// Strongly-typed Process Identifier (PID).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct ProcessId(pub u32);

impl ProcessId {
    /// Create a new `ProcessId`.
    #[must_use]
    pub const fn new(pid: u32) -> Self {
        Self(pid)
    }

    /// Return the numerical PID as a `u32`.
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self.0
    }

    /// Return the numerical PID as an `i32` for POSIX syscalls.
    #[must_use]
    pub const fn as_raw(self) -> i32 {
        self.0 as i32
    }
}

impl fmt::Display for ProcessId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PID({})", self.0)
    }
}

/// Strongly-typed Process Group Identifier (PGID).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct ProcessGroupId(pub u32);

impl ProcessGroupId {
    /// Create a new `ProcessGroupId`.
    #[must_use]
    pub const fn new(pgid: u32) -> Self {
        Self(pgid)
    }

    /// Return the numerical PGID as a `u32`.
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        self.0
    }

    /// Return the negative raw PGID used for POSIX group signaling (`kill(-pgid, sig)`).
    #[must_use]
    pub const fn as_negative_raw(self) -> i32 {
        -(self.0 as i32)
    }
}

impl fmt::Display for ProcessGroupId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PGID({})", self.0)
    }
}

/// Normalized exit status of a terminated child process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExitStatus {
    /// Clean termination with exit code 0.
    Success,
    /// Terminated normally with non-zero exit code.
    Exited(i32),
    /// Terminated by a fatal signal without core dump.
    Signaled(Signal),
    /// Terminated by a fatal signal with core dump.
    CoreDumped(Signal),
}

impl ExitStatus {
    /// Whether the process exited cleanly with code 0.
    #[must_use]
    pub const fn is_success(self) -> bool {
        matches!(self, Self::Success)
    }

    /// The exit code if the process exited normally.
    #[must_use]
    pub const fn code(self) -> Option<i32> {
        match self {
            Self::Success => Some(0),
            Self::Exited(c) => Some(c),
            Self::Signaled(_) | Self::CoreDumped(_) => None,
        }
    }

    /// The terminating signal if killed by a signal.
    #[must_use]
    pub const fn signal(self) -> Option<Signal> {
        match self {
            Self::Signaled(s) | Self::CoreDumped(s) => Some(s),
            Self::Success | Self::Exited(_) => None,
        }
    }

    /// Synthesize an exit status from raw POSIX `waitpid` status integer.
    #[must_use]
    pub fn from_raw_status(status: i32) -> Self {
        let exited = (status & 0x7f) == 0;
        if exited {
            let code = (status >> 8) & 0xff;
            if code == 0 {
                Self::Success
            } else {
                Self::Exited(code)
            }
        } else {
            let sig_num = status & 0x7f;
            let coredump = (status & 0x80) != 0;
            let sig = Signal::from_raw(sig_num).unwrap_or(Signal::Term);
            if coredump {
                Self::CoreDumped(sig)
            } else {
                Self::Signaled(sig)
            }
        }
    }
}

impl fmt::Display for ExitStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success => write!(f, "exit(0)"),
            Self::Exited(code) => write!(f, "exit({code})"),
            Self::Signaled(sig) => write!(f, "signaled({sig})"),
            Self::CoreDumped(sig) => write!(f, "core_dumped({sig})"),
        }
    }
}

/// Lifecycle state of a child or supervised process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChildState {
    /// Process is currently active and running.
    Running,
    /// Process has been paused/stopped by a signal (e.g. `SIGSTOP` or `SIGTSTP`).
    Stopped(Signal),
    /// Process has terminated and status was reaped.
    Terminated(ExitStatus),
}

impl ChildState {
    /// Whether the process is currently running.
    #[must_use]
    pub const fn is_running(self) -> bool {
        matches!(self, Self::Running)
    }

    /// Whether the process has terminated.
    #[must_use]
    pub const fn is_terminated(self) -> bool {
        matches!(self, Self::Terminated(_))
    }
}

/// Policy for subreaper zombie adoption (`PR_SET_CHILD_SUBREAPER`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SubreaperPolicy {
    /// Adopt orphaned descendant processes and reap them when they exit.
    #[default]
    AdoptOrphans,
    /// Do not configure process as subreaper; orphans reparent to PID 1.
    Disabled,
}

/// Target destination for signal dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KillTarget {
    /// Target a single specific process by PID.
    Process(ProcessId),
    /// Broadcast to an entire process group (`kill(-pgid, sig)`).
    ProcessGroup(ProcessGroupId),
    /// Freeze and signal an entire cgroup v2 path.
    Cgroup(String),
}

/// Policy governing process restart upon termination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SupervisionPolicy {
    /// Never restart upon termination.
    #[default]
    Never,
    /// Always restart regardless of exit status.
    RestartAlways,
    /// Restart only on non-zero or signaled exit up to `max_retries`.
    RestartOnFailure {
        /// Maximum number of retries permitted before giving up.
        max_retries: u32,
    },
}

/// Record of a supervised process tracked in memory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessRecord {
    /// Process ID.
    pub pid: ProcessId,
    /// Process Group ID.
    pub pgid: ProcessGroupId,
    /// Current execution state.
    pub state: ChildState,
    /// Human-readable label or command name.
    pub label: String,
    /// Whether this was directly spawned or adopted as an orphaned grandchild.
    pub is_direct_child: bool,
}

impl ProcessRecord {
    /// Create a new process record in `Running` state.
    #[must_use]
    pub fn new(
        pid: ProcessId,
        pgid: ProcessGroupId,
        label: impl Into<String>,
        is_direct: bool,
    ) -> Self {
        Self {
            pid,
            pgid,
            state: ChildState::Running,
            label: label.into(),
            is_direct_child: is_direct,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_id_raw() {
        let pid = ProcessId::new(12345);
        assert_eq!(pid.as_u32(), 12345);
        assert_eq!(pid.as_raw(), 12345);
        assert_eq!(pid.to_string(), "PID(12345)");
    }

    #[test]
    fn test_pgid_negative_raw() {
        let pgid = ProcessGroupId::new(999);
        assert_eq!(pgid.as_u32(), 999);
        assert_eq!(pgid.as_negative_raw(), -999);
        assert_eq!(pgid.to_string(), "PGID(999)");
    }

    #[test]
    fn test_exit_status_clean() {
        let status = ExitStatus::from_raw_status(0);
        assert!(status.is_success());
        assert_eq!(status.code(), Some(0));
        assert_eq!(status.signal(), None);
    }

    #[test]
    fn test_exit_status_code() {
        // POSIX exit code 42 shifted by 8 = 42 * 256 = 10752
        let status = ExitStatus::from_raw_status(42 << 8);
        assert!(!status.is_success());
        assert_eq!(status.code(), Some(42));
        assert_eq!(status.signal(), None);
    }

    #[test]
    fn test_exit_status_signaled() {
        // Killed by SIGTERM (15) without core dump
        let status = ExitStatus::from_raw_status(15);
        assert!(!status.is_success());
        assert_eq!(status.code(), None);
        assert_eq!(status.signal(), Some(Signal::Term));
    }
}
