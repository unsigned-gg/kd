//! Typed domain error types for `kd`.
//!
//! All domain errors are pure values without I/O handles or external side effects.

use thiserror::Error;

/// Core domain error enumeration for process lifecycle and signal coordination.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum KdError {
    /// Attempted to construct or parse an invalid signal number or name.
    #[error("unknown or invalid signal: {0}")]
    InvalidSignal(String),

    /// Process handle not found in the supervised process tree.
    #[error("process id {0} not found in supervisor tree")]
    ProcessNotFound(u32),

    /// Process group handle not found.
    #[error("process group id {0} not found")]
    ProcessGroupNotFound(u32),

    /// Invalid shutdown state transition requested.
    #[error("invalid shutdown phase transition from {current} to {target}")]
    InvalidPhaseTransition {
        /// Current phase of the state machine.
        current: String,
        /// Requested target phase.
        target: String,
    },

    /// The requested operation requires supervisor capabilities that are disabled.
    #[error("supervisor capability not enabled: {0}")]
    CapabilityDisabled(&'static str),

    /// Timeout exceeded during graceful shutdown drain or escalation.
    #[error("shutdown timeout exceeded ({timeout_ms} ms) in phase: {phase}")]
    ShutdownTimeout {
        /// Active phase during timeout.
        phase: String,
        /// Configured timeout in milliseconds.
        timeout_ms: u64,
    },

    /// Process is in an incompatible state for the requested signal.
    #[error("process {pid} is in state {state}, cannot accept signal {signal}")]
    IncompatibleProcessState {
        /// Process identifier.
        pid: u32,
        /// Current process status string.
        state: String,
        /// Signal attempted.
        signal: String,
    },

    /// An internal invariant was breached.
    #[error("invariant violation: {0}")]
    InvariantViolation(&'static str),
}
