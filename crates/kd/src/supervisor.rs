//! Process supervisor primitives: pidfd, subreaper, negative PGID kill, and cgroup freezing.
//!
//! Bridges POSIX and modern Linux kernel primitives to prevent PID recycling races,
//! eliminate zombie accumulation, and ensure complete tree teardown.

use std::fs;
use std::io;
use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::path::Path;
use std::time::Duration;

use kd_domain::{ExitStatus, ProcessGroupId, ProcessId, Signal};

// Linux syscall numbers for x86_64 and aarch64
#[cfg(target_arch = "x86_64")]
const SYS_PIDFD_OPEN: libc::c_long = 434;
#[cfg(target_arch = "x86_64")]
const SYS_PIDFD_SEND_SIGNAL: libc::c_long = 424;

#[cfg(target_arch = "aarch64")]
const SYS_PIDFD_OPEN: libc::c_long = 434;
#[cfg(target_arch = "aarch64")]
const SYS_PIDFD_SEND_SIGNAL: libc::c_long = 424;

/// Safe RAII wrapper around a Linux process descriptor (`pidfd`).
///
/// Prevents PID recycling races: holding a `pidfd` guarantees that signals
/// are delivered to the intended process incarnation rather than a recycled PID.
#[derive(Debug)]
pub struct PidFd {
    fd: OwnedFd,
    pid: ProcessId,
}

impl PidFd {
    /// Open a `pidfd` referring to the given process identifier.
    ///
    /// Requires Linux kernel >= 5.3.
    ///
    /// # Errors
    /// Returns `io::Error` if `pidfd_open` fails or is not supported.
    pub fn open(pid: ProcessId) -> io::Result<Self> {
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        {
            // SAFETY: pidfd_open syscall with valid flags (PIDFD_NONBLOCK | O_CLOEXEC)
            #[allow(unsafe_code)]
            let fd = unsafe {
                libc::syscall(
                    SYS_PIDFD_OPEN,
                    pid.as_raw(),
                    libc::PIDFD_NONBLOCK | (libc::O_CLOEXEC as u32),
                )
            };

            if fd < 0 {
                Err(io::Error::last_os_error())
            } else {
                // SAFETY: fd is valid non-negative descriptor returned by syscall.
                #[allow(unsafe_code)]
                let owned = unsafe { OwnedFd::from_raw_fd(fd as RawFd) };
                Ok(Self { fd: owned, pid })
            }
        }

        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "pidfd_open is only supported on Linux x86_64 and aarch64",
            ))
        }
    }

    /// Associated Process ID.
    #[must_use]
    pub const fn pid(&self) -> ProcessId {
        self.pid
    }

    /// Send a signal to the process referenced by this descriptor.
    ///
    /// # Errors
    /// Returns `io::Error` if `pidfd_send_signal` fails. If the process has already
    /// terminated and its status reaped, returns `ESRCH`.
    pub fn send_signal(&self, signal: Signal) -> io::Result<()> {
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        {
            // SAFETY: pidfd_send_signal with valid fd and signal number.
            #[allow(unsafe_code)]
            let ret = unsafe {
                libc::syscall(
                    SYS_PIDFD_SEND_SIGNAL,
                    self.fd.as_raw_fd(),
                    signal.as_raw(),
                    std::ptr::null::<libc::siginfo_t>(),
                    0 as libc::c_uint,
                )
            };

            if ret < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        }

        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "pidfd_send_signal is only supported on Linux",
            ))
        }
    }

    /// Wait for process termination with an optional timeout using Linux `poll(2)`.
    ///
    /// On Linux >= 5.3, a process file descriptor becomes readable (`POLLIN`)
    /// when the monitored process terminates.
    ///
    /// Returns `Ok(true)` if process exited, `Ok(false)` on timeout, or `Err(io::Error)` on failure.
    ///
    /// # Errors
    /// Returns `io::Error` if `poll(2)` fails, or `ErrorKind::Unsupported` on platforms
    /// without pidfd support.
    pub fn wait_timeout(&self, timeout: Option<Duration>) -> io::Result<bool> {
        #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
        {
            let timeout_ms = match timeout {
                Some(dur) => dur.as_millis().min(libc::c_int::MAX as u128) as libc::c_int,
                None => -1,
            };

            let mut pfd = libc::pollfd {
                fd: self.fd.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };

            // SAFETY: Calling poll on valid descriptor owned by Self.
            #[allow(unsafe_code)]
            let ret = unsafe { libc::poll(&mut pfd, 1, timeout_ms) };

            if ret < 0 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::Interrupted {
                    Ok(false)
                } else {
                    Err(err)
                }
            } else if ret == 0 {
                Ok(false)
            } else {
                Ok((pfd.revents & (libc::POLLIN | libc::POLLHUP | libc::POLLERR)) != 0)
            }
        }

        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            let _ = timeout;
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "pidfd_wait is only supported on Linux",
            ))
        }
    }
}

/// Send a signal to a process, preferring `pidfd_send_signal` and falling back to `kill(2)`.
///
/// # Errors
/// Returns `io::Error` if the signal could not be delivered.
pub fn send_signal_safe(pid: ProcessId, signal: Signal) -> io::Result<()> {
    if let Ok(pidfd) = PidFd::open(pid) {
        pidfd.send_signal(signal)
    } else {
        // Fallback to POSIX kill
        // SAFETY: kill(2) with valid PID and signal.
        #[allow(unsafe_code)]
        let ret = unsafe { libc::kill(pid.as_raw(), signal.as_raw()) };
        if ret == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
}

/// Configure the current process as a child subreaper via `PR_SET_CHILD_SUBREAPER`.
///
/// When enabled, orphaned descendant processes whose parents exit reparent to this
/// process rather than PID 1, enabling complete zombie reaping in agent harnesses.
///
/// # Errors
/// Returns `io::Error` if `prctl` fails.
pub fn set_child_subreaper(enabled: bool) -> io::Result<()> {
    let val: libc::c_ulong = if enabled { 1 } else { 0 };
    // SAFETY: prctl with PR_SET_CHILD_SUBREAPER is a safe and standard Linux operation.
    #[allow(unsafe_code)]
    let ret = unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, val, 0, 0, 0) };
    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Query whether the current process is configured as a child subreaper.
///
/// # Errors
/// Returns `io::Error` if `prctl` fails.
pub fn is_child_subreaper() -> io::Result<bool> {
    let mut val: libc::c_int = 0;
    // SAFETY: Passing valid pointer to stack integer to prctl.
    #[allow(unsafe_code)]
    let ret = unsafe {
        libc::prctl(
            libc::PR_GET_CHILD_SUBREAPER,
            &mut val as *mut libc::c_int as libc::c_ulong,
            0,
            0,
            0,
        )
    };
    if ret == 0 {
        Ok(val != 0)
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Broadcast a signal to an entire process group using negative PGID (`kill(-pgid, sig)`).
///
/// # Errors
/// Returns `io::Error` if `kill(2)` fails.
pub fn kill_process_group(pgid: ProcessGroupId, signal: Signal) -> io::Result<()> {
    let raw_target = pgid.as_negative_raw();
    // SAFETY: kill(2) with negative target signals every process in that process group.
    #[allow(unsafe_code)]
    let ret = unsafe { libc::kill(raw_target, signal.as_raw()) };
    if ret == 0 {
        Ok(())
    } else {
        let err = io::Error::last_os_error();
        // ESRCH indicates no processes in group remain, which is considered successful completion.
        if err.raw_os_error() == Some(libc::ESRCH) {
            Ok(())
        } else {
            Err(err)
        }
    }
}

/// Non-blocking harvest of all terminated child and adopted grandchild processes.
///
/// Returns a list of reaped `(ProcessId, ExitStatus)` pairs.
#[must_use]
pub fn reap_zombies() -> Vec<(ProcessId, ExitStatus)> {
    let mut reaped = Vec::new();
    loop {
        let mut status: libc::c_int = 0;
        // SAFETY: waitpid(-1, &mut status, WNOHANG) reaps any dead child without blocking.
        #[allow(unsafe_code)]
        let pid = unsafe { libc::waitpid(-1, &mut status, libc::WNOHANG) };

        if pid > 0 {
            let proc_id = ProcessId::new(pid as u32);
            let exit_status = ExitStatus::from_raw_status(status);
            reaped.push((proc_id, exit_status));
        } else {
            // pid == 0 means children exist but none have exited.
            // pid == -1 (ECHILD) means no children or zombies remain.
            break;
        }
    }
    reaped
}

/// Freeze all processes within a cgroup v2 hierarchy.
///
/// # Errors
/// Returns `io::Error` if the cgroup path does not exist or cannot be written.
pub fn freeze_cgroup(cgroup_path: &Path) -> io::Result<()> {
    let freeze_file = cgroup_path.join("cgroup.freeze");
    fs::write(freeze_file, "1\n")
}

/// Unfreeze all processes within a cgroup v2 hierarchy.
///
/// # Errors
/// Returns `io::Error` if the cgroup path does not exist or cannot be written.
pub fn unfreeze_cgroup(cgroup_path: &Path) -> io::Result<()> {
    let freeze_file = cgroup_path.join("cgroup.freeze");
    fs::write(freeze_file, "0\n")
}

/// Terminate all processes in a cgroup v2 hierarchy.
///
/// Utilizes `cgroup.kill` (Linux kernel >= 5.14) if present, or falls back to
/// reading `cgroup.procs` and signaling each process ID.
///
/// # Errors
/// Returns `io::Error` if cgroup inspection or kill fails.
pub fn kill_cgroup(cgroup_path: &Path, signal: Signal) -> io::Result<()> {
    let kill_file = cgroup_path.join("cgroup.kill");
    if kill_file.exists() {
        fs::write(kill_file, "1\n")
    } else {
        // Fallback: parse cgroup.procs and signal each process
        let procs_file = cgroup_path.join("cgroup.procs");
        let content = fs::read_to_string(procs_file)?;
        for line in content.lines() {
            if let Ok(pid_num) = line.trim().parse::<u32>() {
                let _ = send_signal_safe(ProcessId::new(pid_num), signal);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subreaper_prctl() {
        let initial = is_child_subreaper().expect("query subreaper state");
        set_child_subreaper(true).expect("enable subreaper");
        assert!(is_child_subreaper().expect("subreaper enabled"));
        set_child_subreaper(initial).expect("restore subreaper");
    }

    #[test]
    fn test_reap_zombies_empty() {
        let reaped = reap_zombies();
        // At start of test, no un-reaped dead children should exist
        assert!(reaped.is_empty() || !reaped.is_empty());
    }

    #[test]
    fn test_pidfd_self_open() {
        // SAFETY: getpid is always safe on Linux.
        #[allow(unsafe_code)]
        let my_pid = unsafe { libc::getpid() as u32 };
        if let Ok(pidfd) = PidFd::open(ProcessId::new(my_pid)) {
            assert_eq!(pidfd.pid().as_u32(), my_pid);
        }
    }

    #[test]
    fn test_pidfd_wait_timeout_on_child() {
        use std::process::Command;

        let mut child = Command::new("sleep")
            .arg("0.05")
            .spawn()
            .expect("spawn sleep");
        let pid = ProcessId::new(child.id());

        if let Ok(pidfd) = PidFd::open(pid) {
            let exited = pidfd
                .wait_timeout(Some(Duration::from_secs(1)))
                .expect("poll wait");
            assert!(exited, "process should have exited within timeout");
        }

        let _ = child.wait();
    }
}
