//! Pure domain logic for `kd` (killdeath signal coordination framework).
//!
//! Operates strictly in userspace memory. Contains zero system call wrappers,
//! zero filesystem handles, zero network descriptors, zero process spawning, and
//! zero asynchronous runtimes. Domain entities, signal enumerations, and state transitions
//! remain completely deterministic, memory-safe, and independent of platform weather.
//!
//! Real syscall boundaries (`pidfd_open`, `libc::kill`, `libc::waitpid`, `prctl`)
//! are isolated strictly in the outer `kd` crate.
//!
//! # Example
//!
//! ```
//! use kd_domain::{Signal, ShutdownConfig, ShutdownStateMachine, ShutdownAction};
//!
//! let mut sm = ShutdownStateMachine::new(ShutdownConfig::default());
//! assert!(sm.is_running());
//!
//! // Handle termination signal: triggers drain phase
//! let action = sm.handle_signal(Signal::Term);
//! assert!(matches!(action, ShutdownAction::EnterDrain { .. }));
//! assert!(sm.is_shutting_down());
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod error;
pub mod process;
pub mod shutdown;
pub mod signal;

pub use error::KdError;
pub use process::{
    ChildState, ExitStatus, KillTarget, ProcessGroupId, ProcessId, ProcessRecord, SubreaperPolicy,
    SupervisionPolicy,
};
pub use shutdown::{ShutdownAction, ShutdownConfig, ShutdownPhase, ShutdownStateMachine};
pub use signal::Signal;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_io_domain_serialization() {
        let sig = Signal::Term;
        let serialized = serde_json::to_string(&sig).expect("serialize signal");
        assert_eq!(serialized, "\"Term\"");
        let deserialized: Signal = serde_json::from_str(&serialized).expect("deserialize signal");
        assert_eq!(deserialized, Signal::Term);

        let pid = ProcessId::new(42);
        let pid_json = serde_json::to_string(&pid).expect("serialize pid");
        assert_eq!(pid_json, "42");
    }
}
