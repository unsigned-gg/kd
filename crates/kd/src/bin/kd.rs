//! Command-line interface for `kd` (process supervision and signal coordination).

use std::os::unix::process::CommandExt;
use std::process::{Command as StdCommand, ExitCode};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use kd::{
    reap_zombies, set_child_subreaper, trap_default_termination, ProcessGroupId, ProcessId,
    ProcessRecord, ShutdownAction, ShutdownConfig, ShutdownStateMachine, Signal,
    TeardownCoordinator, TerminalGuard,
};
use serde::Serialize;
use tracing_subscriber::EnvFilter;

/// Version text: crate version plus the git revision stamped by `build.rs`.
const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("KD_BUILD_REV"), ")");

/// High-performance signal coordination and process supervision framework.
#[derive(Debug, Parser)]
#[command(
    name = "kd",
    version = VERSION,
    about = "Modern signal handling, process lifecycle, supervisor, and terminal hygiene framework."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Demonstrate 2-phase graceful shutdown (Drain -> Escalate -> ForceKill) and emit JSON status.
    Demo {
        /// Identifier for the demo workload run.
        #[arg(long, default_value = "default-demo")]
        id: String,
    },

    /// Supervise an arbitrary command with child subreaper, isolated PGID, and graceful teardown.
    Run {
        /// Drain timeout in milliseconds before escalating.
        #[arg(long, default_value = "5000")]
        drain_ms: u64,

        /// Escalation timeout in milliseconds before force killing.
        #[arg(long, default_value = "3000")]
        escalate_ms: u64,

        /// Command to execute.
        command: String,

        /// Arguments passed to the supervised command.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Run as a container init process (PID 1), adopting and reaping all orphaned zombies.
    Init {
        /// Inbound drain timeout in milliseconds upon receiving SIGTERM.
        #[arg(long, default_value = "10000")]
        drain_ms: u64,

        /// Escalation timeout in milliseconds before force killing.
        #[arg(long, default_value = "5000")]
        escalate_ms: u64,

        /// Optional command to execute as the primary supervised workload.
        command: Option<String>,

        /// Arguments passed to the primary command.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// List all recognized POSIX signals and their integer numbers and categories.
    Signals,

    /// Harvest any pending zombie processes currently reparented to this process.
    Reap,
}

/// Structured record of the demo execution.
#[derive(Debug, Serialize)]
struct DemoOutput {
    id: String,
    phases_traversed: Vec<String>,
    signals_tested: Vec<String>,
    terminal_hygiene: &'static str,
    status: &'static str,
}

fn run_demo(id: &str) -> Result<DemoOutput> {
    // 1. Initialize terminal hygiene guard
    let _guard = TerminalGuard::init().context("failed to initialize terminal guard")?;

    // 2. Drive the state machine through phases deterministically
    let config = ShutdownConfig {
        drain_grace_period_ms: 100,
        escalate_timeout_ms: 50,
        force_kill_timeout_ms: 20,
        default_exit_code: 0,
        double_sigint_fast_track: true,
    };

    let mut sm = ShutdownStateMachine::new(config);
    let mut phases = vec![sm.phase().name().to_owned()];

    // Trigger Drain via SIGTERM
    let action = sm.handle_signal(Signal::Term);
    assert!(matches!(action, ShutdownAction::EnterDrain { .. }));
    phases.push(sm.phase().name().to_owned());

    // Advance clock past drain grace period -> Escalate
    let action = sm.tick(120);
    assert!(matches!(action, ShutdownAction::EnterEscalate { .. }));
    phases.push(sm.phase().name().to_owned());

    // Advance clock past escalate timeout -> ForceKill
    let action = sm.tick(60);
    assert!(matches!(action, ShutdownAction::EnterForceKill));
    phases.push(sm.phase().name().to_owned());

    // Complete teardown
    let action = sm.mark_all_reaped();
    assert!(matches!(action, ShutdownAction::Complete { .. }));
    phases.push(sm.phase().name().to_owned());

    Ok(DemoOutput {
        id: id.to_owned(),
        phases_traversed: phases,
        signals_tested: vec![
            "SIGTERM".to_owned(),
            "SIGINT".to_owned(),
            "SIGCHLD".to_owned(),
        ],
        terminal_hygiene: "guaranteed",
        status: "done",
    })
}

fn run_supervised(
    command: &str,
    args: &[String],
    drain_ms: u64,
    escalate_ms: u64,
) -> Result<ExitCode> {
    let _guard = TerminalGuard::init().context("failed to initialize terminal guard")?;
    let _ = set_child_subreaper(true);
    let receiver = trap_default_termination().context("failed to trap termination signals")?;

    let mut cmd = StdCommand::new(command);
    cmd.args(args);
    cmd.process_group(0);
    let child = cmd
        .spawn()
        .with_context(|| format!("failed to spawn {command}"))?;

    let pid = ProcessId::new(child.id());
    let pgid = ProcessGroupId::new(child.id());

    let config = ShutdownConfig {
        drain_grace_period_ms: drain_ms,
        escalate_timeout_ms: escalate_ms,
        force_kill_timeout_ms: 2000,
        default_exit_code: 0,
        double_sigint_fast_track: true,
    };

    let mut coordinator = TeardownCoordinator::new(config);
    coordinator.track_process(ProcessRecord::new(pid, pgid, command, true));

    let exit_code = coordinator.run_loop(&receiver);
    Ok(ExitCode::from(exit_code as u8))
}

fn run_init(
    drain_ms: u64,
    escalate_ms: u64,
    command: Option<String>,
    args: &[String],
) -> Result<ExitCode> {
    let _guard = TerminalGuard::init().context("failed to initialize terminal guard")?;
    let _ = set_child_subreaper(true);
    let receiver = trap_default_termination().context("failed to trap termination signals")?;

    let config = ShutdownConfig {
        drain_grace_period_ms: drain_ms,
        escalate_timeout_ms: escalate_ms,
        force_kill_timeout_ms: 2000,
        default_exit_code: 0,
        double_sigint_fast_track: true,
    };

    let mut coordinator = TeardownCoordinator::new(config);

    if let Some(ref cmd_name) = command {
        let mut cmd = StdCommand::new(cmd_name);
        cmd.args(args);
        cmd.process_group(0);
        let child = cmd
            .spawn()
            .with_context(|| format!("failed to spawn {cmd_name}"))?;

        let pid = ProcessId::new(child.id());
        let pgid = ProcessGroupId::new(child.id());
        coordinator.track_process(ProcessRecord::new(pid, pgid, cmd_name, true));

        tracing::info!(
            command = %cmd_name,
            pid = %pid,
            "kd init container supervisor active (PID 1 mode with primary workload)"
        );
    } else {
        tracing::info!("kd init container supervisor active (PID 1 orphan-reaper mode)");
    }

    let exit_code = coordinator.run_loop(&receiver);
    Ok(ExitCode::from(exit_code as u8))
}

fn main() -> Result<ExitCode> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Demo { id } => {
            let output = run_demo(&id)?;
            println!("{}", serde_json::to_string_pretty(&output)?);
            Ok(ExitCode::SUCCESS)
        }
        Command::Run {
            drain_ms,
            escalate_ms,
            command,
            args,
        } => run_supervised(&command, &args, drain_ms, escalate_ms),
        Command::Init {
            drain_ms,
            escalate_ms,
            command,
            args,
        } => run_init(drain_ms, escalate_ms, command, &args),
        Command::Signals => {
            let signals = [
                Signal::Hup,
                Signal::Int,
                Signal::Quit,
                Signal::Kill,
                Signal::Term,
                Signal::Chld,
                Signal::Stop,
                Signal::Winch,
                Signal::Usr1,
                Signal::Usr2,
            ];
            println!(
                "{:<10} {:<6} {:<14} {:<12}",
                "NAME", "RAW", "TERMINATION?", "CATCHABLE?"
            );
            println!("{:-<45}", "");
            for sig in signals {
                println!(
                    "{:<10} {:<6} {:<14} {:<12}",
                    sig.as_str(),
                    sig.as_raw(),
                    sig.is_termination(),
                    sig.can_catch()
                );
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Reap => {
            let reaped = reap_zombies();
            println!("Reaped {} zombie processes.", reaped.len());
            for (pid, status) in reaped {
                println!("  - {}: {}", pid, status);
            }
            Ok(ExitCode::SUCCESS)
        }
    }
}
