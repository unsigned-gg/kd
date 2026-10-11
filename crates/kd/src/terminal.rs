//! RAII terminal hygiene guard restoring raw mode and cursor settings on drop or panic.
//!
//! Guarantees that terminal multiplexers, agent sessions, and operator shells
//! are never left with echo disabled, raw mode active, or cursor invisible.

use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};

static GUARD_ACTIVE: AtomicBool = AtomicBool::new(false);

/// RAII guard holding original termios settings and restoring them on drop or panic.
#[derive(Debug)]
pub struct TerminalGuard {
    is_tty: bool,
    orig_termios: Option<libc::termios>,
}

impl TerminalGuard {
    /// Inspect stdin and instantiate a terminal hygiene guard.
    ///
    /// # Errors
    /// Returns `io::Error` if stdin is a TTY and `tcgetattr` fails.
    pub fn init() -> io::Result<Self> {
        // SAFETY: isatty check on standard input fd.
        #[allow(unsafe_code)]
        let is_tty = unsafe { libc::isatty(libc::STDIN_FILENO) != 0 };

        if !is_tty {
            return Ok(Self {
                is_tty: false,
                orig_termios: None,
            });
        }

        #[allow(unsafe_code)]
        // SAFETY: zeroed libc::termios is valid initialization before tcgetattr.
        let mut termios: libc::termios = unsafe { std::mem::zeroed() };
        // SAFETY: tcgetattr into valid termios pointer.
        #[allow(unsafe_code)]
        let ret = unsafe { libc::tcgetattr(libc::STDIN_FILENO, &mut termios) };
        if ret != 0 {
            return Err(io::Error::last_os_error());
        }

        GUARD_ACTIVE.store(true, Ordering::SeqCst);

        Ok(Self {
            is_tty: true,
            orig_termios: Some(termios),
        })
    }

    /// Whether standard input is a live TTY.
    #[must_use]
    pub const fn is_tty(&self) -> bool {
        self.is_tty
    }

    /// Safely configure terminal into raw mode while ensuring restoration on drop.
    ///
    /// # Errors
    /// Returns `io::Error` if terminal settings cannot be modified.
    pub fn set_raw_mode(&mut self) -> io::Result<()> {
        if let Some(orig) = &self.orig_termios {
            let mut raw = *orig;
            // Standard raw mode modifications per termios(3)
            raw.c_iflag &= !(libc::BRKINT | libc::ICRNL | libc::INPCK | libc::ISTRIP | libc::IXON);
            raw.c_oflag &= !(libc::OPOST);
            raw.c_cflag |= libc::CS8;
            raw.c_lflag &= !(libc::ECHO | libc::ICANON | libc::IEXTEN | libc::ISIG);
            raw.c_cc[libc::VMIN] = 1;
            raw.c_cc[libc::VTIME] = 0;

            // SAFETY: tcsetattr with TCSANOW applies new termios attributes immediately.
            #[allow(unsafe_code)]
            let ret = unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &raw) };
            if ret != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }

    /// Explicitly restore original terminal attributes and re-enable cursor.
    pub fn restore(&mut self) {
        if let Some(orig) = &self.orig_termios {
            // SAFETY: tcsetattr with TCSANOW restores original terminal mode.
            #[allow(unsafe_code)]
            unsafe {
                libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, orig);
            }
            // Emit ANSI reset attributes: Show cursor (\x1b[?25h) and reset color/styling (\x1b[0m)
            let mut stdout = io::stdout();
            let _ = stdout.write_all(b"\x1b[?25h\x1b[0m");
            let _ = stdout.flush();
        }
        GUARD_ACTIVE.store(false, Ordering::SeqCst);
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        self.restore();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_guard_init() {
        let guard = TerminalGuard::init().expect("terminal guard init");
        // In automated tests or CI, is_tty may be false, but init should succeed cleanly.
        let _ = guard.is_tty();
    }
}
