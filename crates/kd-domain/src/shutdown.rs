//! Declarative 2-phase graceful shutdown state machine (Drain → Escalate → ForceKill).
//!
//! Provides deterministic shutdown transitions without blocking or asynchronous runtime dependencies.
//! Zero I/O is performed in this module.

use core::fmt;
use serde::{Deserialize, Serialize};

use crate::error::KdError;
use crate::signal::Signal;

/// Distinct lifecycle phases of the shutdown sequence.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShutdownPhase {
    /// Normal operation. Inbound work accepted.
    Running,
    /// First phase: inbound work rejected; workers drain in-flight tasks.
    Drain {
        /// Configured grace period in milliseconds.
        grace_period_ms: u64,
        /// Elapsed milliseconds in drain phase.
        elapsed_ms: u64,
        /// Trigger reason (e.g. `"SIGTERM"` or custom event).
        reason: String,
    },
    /// Second phase: drain grace period expired; workers forced to stop.
    Escalate {
        /// Configured escalation timeout in milliseconds.
        timeout_ms: u64,
        /// Elapsed milliseconds in escalation phase.
        elapsed_ms: u64,
    },
    /// Third phase: escalation timeout expired; uncatchable SIGKILL emitted.
    ForceKill {
        /// Configured force-kill timeout in milliseconds.
        timeout_ms: u64,
        /// Elapsed milliseconds in force-kill phase.
        elapsed_ms: u64,
    },
    /// Teardown completed. Process can safely exit.
    Terminated {
        /// Process exit code.
        exit_code: i32,
    },
}

impl ShutdownPhase {
    /// Return the string identifier of the current phase.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Running => "Running",
            Self::Drain { .. } => "Drain",
            Self::Escalate { .. } => "Escalate",
            Self::ForceKill { .. } => "ForceKill",
            Self::Terminated { .. } => "Terminated",
        }
    }

    /// Whether the state machine is in active running state.
    #[must_use]
    pub const fn is_running(&self) -> bool {
        matches!(self, Self::Running)
    }

    /// Whether teardown has begun (Drain, Escalate, or ForceKill).
    #[must_use]
    pub const fn is_shutting_down(&self) -> bool {
        matches!(
            self,
            Self::Drain { .. } | Self::Escalate { .. } | Self::ForceKill { .. }
        )
    }

    /// Whether the teardown sequence has reached the final terminated state.
    #[must_use]
    pub const fn is_terminated(&self) -> bool {
        matches!(self, Self::Terminated { .. })
    }
}

impl fmt::Display for ShutdownPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Running => write!(f, "Running"),
            Self::Drain {
                grace_period_ms,
                elapsed_ms,
                reason,
            } => {
                write!(
                    f,
                    "Drain({elapsed_ms}/{grace_period_ms}ms, reason={reason})"
                )
            }
            Self::Escalate {
                timeout_ms,
                elapsed_ms,
            } => {
                write!(f, "Escalate({elapsed_ms}/{timeout_ms}ms)")
            }
            Self::ForceKill {
                timeout_ms,
                elapsed_ms,
            } => {
                write!(f, "ForceKill({elapsed_ms}/{timeout_ms}ms)")
            }
            Self::Terminated { exit_code } => write!(f, "Terminated(exit_code={exit_code})"),
        }
    }
}

/// Side-effect instruction emitted when the state machine transitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShutdownAction {
    /// No state change required; maintain current execution.
    None,
    /// Draining initiated: notify external supervisor (`STOPPING=1`) and signal in-flight drain.
    EnterDrain {
        /// Allowed grace period in milliseconds.
        grace_period_ms: u64,
        /// Signal or reason initiating teardown.
        reason: String,
    },
    /// Drain expired: broadcast `SIGTERM` to all child process groups and close connections.
    EnterEscalate {
        /// Allowed escalation timeout in milliseconds.
        timeout_ms: u64,
    },
    /// Escalate expired: broadcast uncatchable `SIGKILL` and freeze cgroups.
    EnterForceKill,
    /// All tasks drained and processes reaped: exit process with status.
    Complete {
        /// Final exit code to pass to `exit(3)`.
        exit_code: i32,
    },
}

/// Configuration parameters for graceful shutdown timing and policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShutdownConfig {
    /// Inbound drain grace period in milliseconds (default: 10,000ms = 10s).
    pub drain_grace_period_ms: u64,
    /// Escalation timeout before force-kill in milliseconds (default: 5,000ms = 5s).
    pub escalate_timeout_ms: u64,
    /// Safety timeout during force kill before hard abort (default: 2,000ms = 2s).
    pub force_kill_timeout_ms: u64,
    /// Default exit code on successful termination.
    pub default_exit_code: i32,
    /// Fast-track to escalate/kill on second SIGINT (Ctrl+C twice).
    pub double_sigint_fast_track: bool,
}

impl Default for ShutdownConfig {
    fn default() -> Self {
        Self {
            drain_grace_period_ms: 10_000,
            escalate_timeout_ms: 5_000,
            force_kill_timeout_ms: 2_000,
            default_exit_code: 0,
            double_sigint_fast_track: true,
        }
    }
}

/// Deterministic, zero-I/O graceful shutdown state machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShutdownStateMachine {
    config: ShutdownConfig,
    phase: ShutdownPhase,
    sigint_count: u32,
}

impl ShutdownStateMachine {
    /// Construct a new shutdown state machine with specified timing configuration.
    #[must_use]
    pub const fn new(config: ShutdownConfig) -> Self {
        Self {
            config,
            phase: ShutdownPhase::Running,
            sigint_count: 0,
        }
    }

    /// Current active shutdown phase.
    #[must_use]
    pub const fn phase(&self) -> &ShutdownPhase {
        &self.phase
    }

    /// Configured timing parameters.
    #[must_use]
    pub const fn config(&self) -> &ShutdownConfig {
        &self.config
    }

    /// Whether the supervisor is in normal running state.
    #[must_use]
    pub const fn is_running(&self) -> bool {
        self.phase.is_running()
    }

    /// Whether shutdown has been initiated.
    #[must_use]
    pub const fn is_shutting_down(&self) -> bool {
        self.phase.is_shutting_down()
    }

    /// Whether shutdown is fully complete.
    #[must_use]
    pub const fn is_terminated(&self) -> bool {
        self.phase.is_terminated()
    }

    /// Ingest a signal and calculate necessary state machine actions.
    pub fn handle_signal(&mut self, signal: Signal) -> ShutdownAction {
        if signal == Signal::Int {
            self.sigint_count += 1;
            if self.config.double_sigint_fast_track && self.sigint_count >= 2 {
                // Operator pressed Ctrl+C twice: immediately bypass drain to force kill
                self.phase = ShutdownPhase::ForceKill {
                    timeout_ms: self.config.force_kill_timeout_ms,
                    elapsed_ms: 0,
                };
                return ShutdownAction::EnterForceKill;
            }
        }

        if !signal.is_termination() {
            return ShutdownAction::None;
        }

        match &self.phase {
            ShutdownPhase::Running => {
                let reason = signal.as_str().to_owned();
                self.phase = ShutdownPhase::Drain {
                    grace_period_ms: self.config.drain_grace_period_ms,
                    elapsed_ms: 0,
                    reason: reason.clone(),
                };
                ShutdownAction::EnterDrain {
                    grace_period_ms: self.config.drain_grace_period_ms,
                    reason,
                }
            }
            ShutdownPhase::Drain { .. } => {
                // Escalation signal while already draining: escalate immediately
                self.phase = ShutdownPhase::Escalate {
                    timeout_ms: self.config.escalate_timeout_ms,
                    elapsed_ms: 0,
                };
                ShutdownAction::EnterEscalate {
                    timeout_ms: self.config.escalate_timeout_ms,
                }
            }
            ShutdownPhase::Escalate { .. } => {
                // Signal during escalate: jump directly to force kill
                self.phase = ShutdownPhase::ForceKill {
                    timeout_ms: self.config.force_kill_timeout_ms,
                    elapsed_ms: 0,
                };
                ShutdownAction::EnterForceKill
            }
            ShutdownPhase::ForceKill { .. } | ShutdownPhase::Terminated { .. } => {
                ShutdownAction::None
            }
        }
    }

    /// Advance the internal clock by `delta_ms` and trigger phase transitions upon timeouts.
    pub fn tick(&mut self, delta_ms: u64) -> ShutdownAction {
        match &mut self.phase {
            ShutdownPhase::Running => ShutdownAction::None,
            ShutdownPhase::Drain {
                grace_period_ms,
                elapsed_ms,
                ..
            } => {
                *elapsed_ms = elapsed_ms.saturating_add(delta_ms);
                if *elapsed_ms >= *grace_period_ms {
                    let timeout = self.config.escalate_timeout_ms;
                    self.phase = ShutdownPhase::Escalate {
                        timeout_ms: timeout,
                        elapsed_ms: 0,
                    };
                    ShutdownAction::EnterEscalate {
                        timeout_ms: timeout,
                    }
                } else {
                    ShutdownAction::None
                }
            }
            ShutdownPhase::Escalate {
                timeout_ms,
                elapsed_ms,
            } => {
                *elapsed_ms = elapsed_ms.saturating_add(delta_ms);
                if *elapsed_ms >= *timeout_ms {
                    self.phase = ShutdownPhase::ForceKill {
                        timeout_ms: self.config.force_kill_timeout_ms,
                        elapsed_ms: 0,
                    };
                    ShutdownAction::EnterForceKill
                } else {
                    ShutdownAction::None
                }
            }
            ShutdownPhase::ForceKill {
                timeout_ms,
                elapsed_ms,
            } => {
                *elapsed_ms = elapsed_ms.saturating_add(delta_ms);
                if *elapsed_ms >= *timeout_ms {
                    // Force-kill timeout expired: complete anyway with failure code
                    self.phase = ShutdownPhase::Terminated { exit_code: 137 }; // 128 + SIGKILL (9)
                    ShutdownAction::Complete { exit_code: 137 }
                } else {
                    ShutdownAction::None
                }
            }
            ShutdownPhase::Terminated { .. } => ShutdownAction::None,
        }
    }

    /// Notify that all supervised processes or worker tasks have drained cleanly.
    pub fn mark_all_reaped(&mut self) -> ShutdownAction {
        if self.phase.is_shutting_down() {
            let code = self.config.default_exit_code;
            self.phase = ShutdownPhase::Terminated { exit_code: code };
            ShutdownAction::Complete { exit_code: code }
        } else {
            ShutdownAction::None
        }
    }

    /// Explicitly force the state machine to terminate with a specified exit code.
    ///
    /// # Errors
    /// Returns `KdError::InvalidPhaseTransition` if already terminated.
    pub fn force_terminate(&mut self, exit_code: i32) -> Result<ShutdownAction, KdError> {
        if self.phase.is_terminated() {
            return Err(KdError::InvalidPhaseTransition {
                current: self.phase.name().to_owned(),
                target: "Terminated".to_owned(),
            });
        }
        self.phase = ShutdownPhase::Terminated { exit_code };
        Ok(ShutdownAction::Complete { exit_code })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_full_lifecycle_drain_escalate_force_kill() {
        let config = ShutdownConfig {
            drain_grace_period_ms: 100,
            escalate_timeout_ms: 50,
            force_kill_timeout_ms: 20,
            default_exit_code: 0,
            double_sigint_fast_track: true,
        };
        let mut sm = ShutdownStateMachine::new(config);
        assert!(sm.is_running());

        // 1. Ingest SIGTERM -> Drain
        let action = sm.handle_signal(Signal::Term);
        assert_eq!(
            action,
            ShutdownAction::EnterDrain {
                grace_period_ms: 100,
                reason: "SIGTERM".to_owned()
            }
        );
        assert_eq!(sm.phase().name(), "Drain");

        // 2. Tick within drain
        assert_eq!(sm.tick(50), ShutdownAction::None);
        assert_eq!(sm.phase().name(), "Drain");

        // 3. Tick over drain grace -> Escalate
        assert_eq!(
            sm.tick(60),
            ShutdownAction::EnterEscalate { timeout_ms: 50 }
        );
        assert_eq!(sm.phase().name(), "Escalate");

        // 4. Tick over escalate timeout -> ForceKill
        assert_eq!(sm.tick(55), ShutdownAction::EnterForceKill);
        assert_eq!(sm.phase().name(), "ForceKill");

        // 5. Tick over force kill timeout -> Complete with 137
        assert_eq!(sm.tick(25), ShutdownAction::Complete { exit_code: 137 });
        assert!(sm.is_terminated());
    }

    #[test]
    fn test_clean_drain_completion() {
        let mut sm = ShutdownStateMachine::new(ShutdownConfig::default());
        sm.handle_signal(Signal::Int);
        assert_eq!(sm.phase().name(), "Drain");

        // Workers finished quickly
        let action = sm.mark_all_reaped();
        assert_eq!(action, ShutdownAction::Complete { exit_code: 0 });
        assert!(sm.is_terminated());
    }

    #[test]
    fn test_double_ctrl_c_fast_track() {
        let mut sm = ShutdownStateMachine::new(ShutdownConfig::default());
        let a1 = sm.handle_signal(Signal::Int);
        assert!(matches!(a1, ShutdownAction::EnterDrain { .. }));

        // Second Ctrl+C immediately triggers ForceKill
        let a2 = sm.handle_signal(Signal::Int);
        assert_eq!(a2, ShutdownAction::EnterForceKill);
        assert_eq!(sm.phase().name(), "ForceKill");
    }
}
