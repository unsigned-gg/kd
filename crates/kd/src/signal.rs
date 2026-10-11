//! Signal trapping, registration, and asynchronous notification using a self-pipe.
//!
//! Avoids async-signal-safety hazards by delegating handling to a non-blocking pipe.
//! The signal handler performs only an async-signal-safe single-byte `write(2)`.

use std::io;
use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use kd_domain::Signal;

/// Global atomic pipe write fd used by the C-compatible signal handler.
static PIPE_WRITE_FD: AtomicI32 = AtomicI32::new(-1);
static HANDLER_LOCK: Mutex<()> = Mutex::new(());

/// Async-signal-safe signal handler function.
///
/// # Safety
/// Only async-signal-safe functions (`write`) are invoked.
#[allow(unsafe_code)]
extern "C" fn raw_signal_handler(signum: libc::c_int) {
    let fd = PIPE_WRITE_FD.load(Ordering::Relaxed);
    if fd >= 0 {
        let byte = signum as u8;
        // SAFETY: write(2) is explicitly listed as async-signal-safe in POSIX.1-2004.
        // We write 1 byte to a non-blocking pipe.
        unsafe {
            libc::write(fd, &byte as *const u8 as *const libc::c_void, 1);
        }
    }
}

/// Receiver end of the signal listening channel.
#[derive(Debug)]
pub struct SignalReceiver {
    read_fd: OwnedFd,
    _write_fd: OwnedFd,
    registered: Vec<Signal>,
}

impl SignalReceiver {
    /// Try to read a pending signal without blocking.
    pub fn try_recv(&self) -> io::Result<Option<Signal>> {
        let mut buf = [0u8; 1];
        let fd = self.read_fd.as_raw_fd();
        // SAFETY: read(2) into our stack buffer is safe. The fd is set to O_NONBLOCK.
        #[allow(unsafe_code)]
        let ret = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, 1) };

        if ret > 0 {
            let sig = Signal::from_raw(i32::from(buf[0])).unwrap_or(Signal::Term);
            Ok(Some(sig))
        } else if ret < 0 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::WouldBlock {
                Ok(None)
            } else {
                Err(err)
            }
        } else {
            Ok(None)
        }
    }

    /// Block waiting for the next signal up to `timeout`.
    pub fn recv_timeout(&self, timeout: Duration) -> io::Result<Option<Signal>> {
        let mut pollfd = libc::pollfd {
            fd: self.read_fd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };

        let timeout_ms = timeout.as_millis().min(i32::MAX as u128) as i32;

        // SAFETY: poll(2) with valid pollfd pointer.
        #[allow(unsafe_code)]
        let ret = unsafe { libc::poll(&mut pollfd, 1, timeout_ms) };

        if ret > 0 && (pollfd.revents & libc::POLLIN) != 0 {
            self.try_recv()
        } else if ret < 0 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::Interrupted {
                // Interrupted by signal: retry non-blocking read immediately
                self.try_recv()
            } else {
                Err(err)
            }
        } else {
            Ok(None)
        }
    }

    /// List of signals monitored by this receiver.
    #[must_use]
    pub fn registered_signals(&self) -> &[Signal] {
        &self.registered
    }
}

impl Drop for SignalReceiver {
    fn drop(&mut self) {
        let _guard = HANDLER_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        PIPE_WRITE_FD.store(-1, Ordering::SeqCst);

        // Reset signal handlers to default
        for sig in &self.registered {
            // SAFETY: Resetting sigaction to default disposition is safe.
            #[allow(unsafe_code)]
            unsafe {
                let mut sa: libc::sigaction = std::mem::zeroed();
                sa.sa_sigaction = libc::SIG_DFL;
                libc::sigaction(sig.as_raw(), &sa, std::ptr::null_mut());
            }
        }
    }
}

/// Register interest in standard POSIX signals and return a `SignalReceiver`.
///
/// # Errors
/// Returns `io::Error` if pipe creation or `sigaction` registration fails.
pub fn trap_signals(signals: &[Signal]) -> io::Result<SignalReceiver> {
    let _guard = HANDLER_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let mut pipefds = [0 as libc::c_int; 2];
    // SAFETY: pipe2 with O_NONBLOCK and O_CLOEXEC is safe and creates non-blocking descriptors.
    #[allow(unsafe_code)]
    let ret = unsafe { libc::pipe2(pipefds.as_mut_ptr(), libc::O_NONBLOCK | libc::O_CLOEXEC) };
    if ret != 0 {
        return Err(io::Error::last_os_error());
    }

    // SAFETY: pipe2 succeeded, transferring ownership to OwnedFd.
    #[allow(unsafe_code)]
    let (read_fd, write_fd) = unsafe {
        (
            OwnedFd::from_raw_fd(pipefds[0]),
            OwnedFd::from_raw_fd(pipefds[1]),
        )
    };

    PIPE_WRITE_FD.store(write_fd.as_raw_fd(), Ordering::SeqCst);

    for &sig in signals {
        if !sig.can_catch() {
            continue;
        }

        // SAFETY: Setting sigaction with our async-signal-safe handler.
        #[allow(unsafe_code)]
        unsafe {
            let mut sa: libc::sigaction = std::mem::zeroed();
            sa.sa_sigaction = raw_signal_handler as *const () as usize;
            sa.sa_flags = libc::SA_RESTART | libc::SA_NOCLDSTOP;
            libc::sigemptyset(&mut sa.sa_mask);

            let res = libc::sigaction(sig.as_raw(), &sa, std::ptr::null_mut());
            if res != 0 {
                return Err(io::Error::last_os_error());
            }
        }
    }

    Ok(SignalReceiver {
        read_fd,
        _write_fd: write_fd,
        registered: signals.to_vec(),
    })
}

/// Register default termination signals (`SIGINT`, `SIGTERM`, `SIGHUP`, `SIGQUIT`).
///
/// # Errors
/// Returns `io::Error` if handler registration fails.
pub fn trap_default_termination() -> io::Result<SignalReceiver> {
    trap_signals(&[Signal::Int, Signal::Term, Signal::Hup, Signal::Quit])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trap_and_read_signal() {
        let receiver = trap_signals(&[Signal::Usr1]).expect("trap USR1");

        // Send USR1 to self
        // SAFETY: raise(SIGUSR1) to self is safe.
        #[allow(unsafe_code)]
        unsafe {
            libc::raise(libc::SIGUSR1);
        }

        let received = receiver
            .recv_timeout(Duration::from_millis(200))
            .expect("read signal");
        assert_eq!(received, Some(Signal::Usr1));
    }
}
