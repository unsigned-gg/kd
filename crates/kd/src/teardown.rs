//! Graceful shutdown coordinator driving the 2-phase state machine with real system effects.
//!
//! Handles drain grace periods, systemd notification (`STOPPING=1`), PGID escalation,
//! and uncatchable force-kill execution.

use std::collections::BTreeMap;
use std::env;
use std::io;
use std::os::unix::net::UnixDatagram;
use std::time::{Duration, Instant};

use kd_domain::{
    ChildState, ExitStatus, ProcessGroupId, ProcessId, ProcessRecord, ShutdownAction,
    ShutdownConfig, ShutdownPhase, ShutdownStateMachine, Signal,
};

use crate::signal::SignalReceiver;
use crate::supervisor::{kill_process_group, reap_zombies, send_signal_safe};

/// Systemd readiness and stopping state notification.
pub fn notify_systemd(state: &str) -> io::Result<()> {
    if let Ok(socket_path) = env::var("NOTIFY_SOCKET") {
        if socket_path.starts_with('@') || socket_path.starts_with('/') {
            let socket = UnixDatagram::unbound()?;
            socket.send_to(state.as_bytes(), &socket_path)?;
        }
    }
    Ok(())
}

/// Active shutdown supervisor coordinating process teardown.
#[derive(Debug)]
pub struct TeardownCoordinator {
    state_machine: ShutdownStateMachine,
    tracked_processes: BTreeMap<ProcessId, ProcessRecord>,
    tracked_groups: Vec<ProcessGroupId>,
}

impl TeardownCoordinator {
    /// Construct a new teardown coordinator with timing configuration.
    #[must_use]
    pub fn new(config: ShutdownConfig) -> Self {
        Self {
            state_machine: ShutdownStateMachine::new(config),
            tracked_processes: BTreeMap::new(),
            tracked_groups: Vec::new(),
        }
    }

    /// Register a child process to be tracked and gracefully terminated upon shutdown.
    pub fn track_process(&mut self, record: ProcessRecord) {
        if !self.tracked_groups.contains(&record.pgid) {
            self.tracked_groups.push(record.pgid);
        }
        self.tracked_processes.insert(record.pid, record);
    }

    /// Access the underlying pure state machine.
    #[must_use]
    pub const fn state_machine(&self) -> &ShutdownStateMachine {
        &self.state_machine
    }

    /// Current phase of teardown.
    #[must_use]
    pub const fn phase(&self) -> &ShutdownPhase {
        self.state_machine.phase()
    }

    /// Execute a single supervision tick: harvest zombies, handle signals, and escalate phases.
    pub fn step(&mut self, receiver: Option<&SignalReceiver>, delta: Duration) -> Option<i32> {
        // 1. Harvest any dead children
        let reaped = reap_zombies();
        for (pid, status) in reaped {
            if let Some(record) = self.tracked_processes.get_mut(&pid) {
                record.state = ChildState::Terminated(status);
            }
        }

        // 2. Check for incoming signals
        if let Some(rx) = receiver {
            while let Ok(Some(sig)) = rx.try_recv() {
                let action = self.state_machine.handle_signal(sig);
                self.execute_action(action);
            }
        }

        // 3. Advance state machine clock if shutting down
        if self.state_machine.is_shutting_down() {
            let delta_ms = delta.as_millis().min(u64::MAX as u128) as u64;
            let action = self.state_machine.tick(delta_ms);
            self.execute_action(action);

            // 4. Check if all tracked processes have terminated
            let all_dead = !self.tracked_processes.is_empty()
                && self
                    .tracked_processes
                    .values()
                    .all(|p| p.state.is_terminated());
            if all_dead {
                let action = self.state_machine.mark_all_reaped();
                self.execute_action(action);
            }
        } else if !self.tracked_processes.is_empty() {
            // Check if tracked processes exited during normal running
            let all_dead = self
                .tracked_processes
                .values()
                .all(|p| p.state.is_terminated());
            if all_dead {
                let exit_code = self
                    .tracked_processes
                    .values()
                    .filter_map(|p| match p.state {
                        ChildState::Terminated(ExitStatus::Success) => Some(0),
                        ChildState::Terminated(ExitStatus::Exited(code)) => Some(code),
                        ChildState::Terminated(
                            ExitStatus::Signaled(sig) | ExitStatus::CoreDumped(sig),
                        ) => Some(128 + sig.as_raw()),
                        _ => None,
                    })
                    .next()
                    .unwrap_or(self.state_machine.config().default_exit_code);

                let _ = self.state_machine.force_terminate(exit_code);
            }
        }

        if let ShutdownPhase::Terminated { exit_code } = self.state_machine.phase() {
            Some(*exit_code)
        } else {
            None
        }
    }

    /// Run the supervision loop until termination, polling with bounded sleep intervals.
    pub fn run_loop(&mut self, receiver: &SignalReceiver) -> i32 {
        let mut last_tick = Instant::now();
        let poll_interval = Duration::from_millis(50);

        loop {
            let now = Instant::now();
            let elapsed = now.duration_since(last_tick);
            last_tick = now;

            if let Some(exit_code) = self.step(Some(receiver), elapsed) {
                return exit_code;
            }

            // Sleep up to poll_interval waiting for signal or process termination
            let _ = receiver.recv_timeout(poll_interval);
        }
    }

    fn execute_action(&mut self, action: ShutdownAction) {
        match action {
            ShutdownAction::None => {}
            ShutdownAction::EnterDrain {
                grace_period_ms,
                reason,
            } => {
                tracing::info!(grace_period_ms, reason = %reason, "initiating graceful shutdown drain");
                let _ = notify_systemd("STOPPING=1");

                // Forward the graceful drain signal to all supervised processes
                let drain_sig = match Signal::from_name(&reason) {
                    Ok(sig) if sig.can_catch() => sig,
                    _ => Signal::Term,
                };
                for &pgid in &self.tracked_groups {
                    let _ = kill_process_group(pgid, drain_sig);
                }
                for record in self.tracked_processes.values() {
                    if record.state.is_running() {
                        let _ = send_signal_safe(record.pid, drain_sig);
                    }
                }
            }
            ShutdownAction::EnterEscalate { timeout_ms } => {
                tracing::warn!(
                    timeout_ms,
                    "drain expired, escalating teardown with SIGTERM broadcast"
                );
                for &pgid in &self.tracked_groups {
                    let _ = kill_process_group(pgid, Signal::Term);
                }
                for record in self.tracked_processes.values() {
                    if record.state.is_running() {
                        let _ = send_signal_safe(record.pid, Signal::Term);
                    }
                }
            }
            ShutdownAction::EnterForceKill => {
                tracing::error!("escalate timeout expired, emitting unconditional SIGKILL");
                for &pgid in &self.tracked_groups {
                    let _ = kill_process_group(pgid, Signal::Kill);
                }
                for record in self.tracked_processes.values() {
                    if record.state.is_running() {
                        let _ = send_signal_safe(record.pid, Signal::Kill);
                    }
                }
            }
            ShutdownAction::Complete { exit_code } => {
                tracing::info!(exit_code, "all supervised processes terminated cleanly");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinator_normal_termination_exit_code() {
        let config = ShutdownConfig {
            drain_grace_period_ms: 1000,
            escalate_timeout_ms: 500,
            force_kill_timeout_ms: 200,
            default_exit_code: 0,
            double_sigint_fast_track: true,
        };
        let mut coordinator = TeardownCoordinator::new(config);
        let pid = ProcessId::new(99999);
        let pgid = ProcessGroupId::new(99999);
        let mut record = ProcessRecord::new(pid, pgid, "test-proc", true);
        // Simulate child already terminated with exit code 42
        record.state = ChildState::Terminated(ExitStatus::Exited(42));
        coordinator.track_process(record);

        // First step should detect process termination and complete with code 42
        let exit_code = coordinator.step(None, Duration::from_millis(10));
        assert_eq!(exit_code, Some(42));
        assert_eq!(
            *coordinator.phase(),
            ShutdownPhase::Terminated { exit_code: 42 }
        );
    }

    #[test]
    fn test_coordinator_drain_signal_forwarding() {
        let config = ShutdownConfig {
            drain_grace_period_ms: 1000,
            escalate_timeout_ms: 500,
            force_kill_timeout_ms: 200,
            default_exit_code: 0,
            double_sigint_fast_track: true,
        };
        let mut coordinator = TeardownCoordinator::new(config);
        // Process with dummy PID
        let pid = ProcessId::new(99998);
        let pgid = ProcessGroupId::new(99998);
        let record = ProcessRecord::new(pid, pgid, "test-drain-proc", true);
        coordinator.track_process(record);

        // Action: EnterDrain with SIGINT
        coordinator.execute_action(ShutdownAction::EnterDrain {
            grace_period_ms: 1000,
            reason: "Int".to_owned(),
        });
        // Non-existent PID/PGID won't crash execute_action (idempotent / ESRCH handled)
    }
}
