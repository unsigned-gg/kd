//! Signal definitions, categories, and numerical conversions.
//!
//! Provides platform-independent signal representation and POSIX signal number mapping.
//! Zero I/O is performed in this module.

use core::fmt;
use core::str::FromStr;
use serde::{Deserialize, Serialize};

use crate::error::KdError;

/// Platform-independent enumeration of POSIX signals and supervisor control signals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(i32)]
pub enum Signal {
    /// Terminal hangup (POSIX.1-1990).
    Hup = 1,
    /// Terminal interrupt (Ctrl+C, POSIX.1-1990).
    Int = 2,
    /// Terminal quit (Ctrl+\, POSIX.1-1990).
    Quit = 3,
    /// Illegal instruction (POSIX.1-1990).
    Ill = 4,
    /// Trace/breakpoint trap (POSIX.1-2001).
    Trap = 5,
    /// Process abort signal (POSIX.1-1990).
    Abrt = 6,
    /// Bus error - bad memory access (POSIX.1-2001).
    Bus = 7,
    /// Erroneous arithmetic operation (POSIX.1-1990).
    Fpe = 8,
    /// Kill signal - uncatchable, unignorable (POSIX.1-1990).
    Kill = 9,
    /// User-defined signal 1 (POSIX.1-1990).
    Usr1 = 10,
    /// Invalid memory reference / segmentation violation (POSIX.1-1990).
    Segv = 11,
    /// User-defined signal 2 (POSIX.1-1990).
    Usr2 = 12,
    /// Broken pipe: write to pipe with no readers (POSIX.1-1990).
    Pipe = 13,
    /// Timer signal from alarm(2) (POSIX.1-1990).
    Alrm = 14,
    /// Termination signal (default graceful shutdown request, POSIX.1-1990).
    Term = 15,
    /// Child process stopped or terminated (POSIX.1-1990).
    Chld = 17,
    /// Continue executing, if stopped (POSIX.1-1990).
    Cont = 18,
    /// Stop process execution - uncatchable, unignorable (POSIX.1-1990).
    Stop = 19,
    /// Terminal stop signal (Ctrl+Z, POSIX.1-1990).
    Tstp = 20,
    /// Background process attempting read from TTY (POSIX.1-1990).
    Ttin = 21,
    /// Background process attempting write to TTY (POSIX.1-1990).
    Ttou = 22,
    /// Urgent condition on socket (POSIX.1-2001).
    Urg = 23,
    /// CPU time limit exceeded (POSIX.1-2001).
    Xcpu = 24,
    /// File size limit exceeded (POSIX.1-2001).
    Xfsz = 25,
    /// Virtual timer expired (POSIX.1-2001).
    Vtalrm = 26,
    /// Profiling timer expired (POSIX.1-2001).
    Prof = 27,
    /// Window size change (4.3BSD, POSIX.1-2001).
    Winch = 28,
    /// I/O now possible (4.2BSD).
    Io = 29,
    /// Bad system call (POSIX.1-2001).
    Sys = 31,
}

impl Signal {
    /// Return the raw POSIX signal number as an integer.
    #[must_use]
    pub const fn as_raw(self) -> i32 {
        self as i32
    }

    /// Parse a raw integer into a standard POSIX `Signal`.
    ///
    /// # Errors
    /// Returns `KdError::InvalidSignal` if the integer is not a known signal.
    pub fn from_raw(raw: i32) -> Result<Self, KdError> {
        match raw {
            1 => Ok(Self::Hup),
            2 => Ok(Self::Int),
            3 => Ok(Self::Quit),
            4 => Ok(Self::Ill),
            5 => Ok(Self::Trap),
            6 => Ok(Self::Abrt),
            7 => Ok(Self::Bus),
            8 => Ok(Self::Fpe),
            9 => Ok(Self::Kill),
            10 => Ok(Self::Usr1),
            11 => Ok(Self::Segv),
            12 => Ok(Self::Usr2),
            13 => Ok(Self::Pipe),
            14 => Ok(Self::Alrm),
            15 => Ok(Self::Term),
            17 => Ok(Self::Chld),
            18 => Ok(Self::Cont),
            19 => Ok(Self::Stop),
            20 => Ok(Self::Tstp),
            21 => Ok(Self::Ttin),
            22 => Ok(Self::Ttou),
            23 => Ok(Self::Urg),
            24 => Ok(Self::Xcpu),
            25 => Ok(Self::Xfsz),
            26 => Ok(Self::Vtalrm),
            27 => Ok(Self::Prof),
            28 => Ok(Self::Winch),
            29 => Ok(Self::Io),
            31 => Ok(Self::Sys),
            _ => Err(KdError::InvalidSignal(raw.to_string())),
        }
    }

    /// Canonical short signal name (e.g. `"SIGINT"`, `"SIGTERM"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hup => "SIGHUP",
            Self::Int => "SIGINT",
            Self::Quit => "SIGQUIT",
            Self::Ill => "SIGILL",
            Self::Trap => "SIGTRAP",
            Self::Abrt => "SIGABRT",
            Self::Bus => "SIGBUS",
            Self::Fpe => "SIGFPE",
            Self::Kill => "SIGKILL",
            Self::Usr1 => "SIGUSR1",
            Self::Segv => "SIGSEGV",
            Self::Usr2 => "SIGUSR2",
            Self::Pipe => "SIGPIPE",
            Self::Alrm => "SIGALRM",
            Self::Term => "SIGTERM",
            Self::Chld => "SIGCHLD",
            Self::Cont => "SIGCONT",
            Self::Stop => "SIGSTOP",
            Self::Tstp => "SIGTSTP",
            Self::Ttin => "SIGTTIN",
            Self::Ttou => "SIGTTOU",
            Self::Urg => "SIGURG",
            Self::Xcpu => "SIGXCPU",
            Self::Xfsz => "SIGXFSZ",
            Self::Vtalrm => "SIGVTALRM",
            Self::Prof => "SIGPROF",
            Self::Winch => "SIGWINCH",
            Self::Io => "SIGIO",
            Self::Sys => "SIGSYS",
        }
    }

    /// Parse a signal name (case-insensitive, optional `"SIG"` prefix).
    ///
    /// # Errors
    /// Returns `KdError::InvalidSignal` if the name does not match a known signal.
    pub fn from_name(name: &str) -> Result<Self, KdError> {
        let clean = name.trim();
        let upper = if let Some(stripped) = clean.strip_prefix("sig") {
            stripped
        } else if let Some(stripped) = clean.strip_prefix("SIG") {
            stripped
        } else {
            clean
        };

        if upper.eq_ignore_ascii_case("hup") || upper == "1" {
            Ok(Self::Hup)
        } else if upper.eq_ignore_ascii_case("int") || upper == "2" {
            Ok(Self::Int)
        } else if upper.eq_ignore_ascii_case("quit") || upper == "3" {
            Ok(Self::Quit)
        } else if upper.eq_ignore_ascii_case("ill") || upper == "4" {
            Ok(Self::Ill)
        } else if upper.eq_ignore_ascii_case("trap") || upper == "5" {
            Ok(Self::Trap)
        } else if upper.eq_ignore_ascii_case("abrt") || upper == "6" {
            Ok(Self::Abrt)
        } else if upper.eq_ignore_ascii_case("bus") || upper == "7" {
            Ok(Self::Bus)
        } else if upper.eq_ignore_ascii_case("fpe") || upper == "8" {
            Ok(Self::Fpe)
        } else if upper.eq_ignore_ascii_case("kill") || upper == "9" {
            Ok(Self::Kill)
        } else if upper.eq_ignore_ascii_case("usr1") || upper == "10" {
            Ok(Self::Usr1)
        } else if upper.eq_ignore_ascii_case("segv") || upper == "11" {
            Ok(Self::Segv)
        } else if upper.eq_ignore_ascii_case("usr2") || upper == "12" {
            Ok(Self::Usr2)
        } else if upper.eq_ignore_ascii_case("pipe") || upper == "13" {
            Ok(Self::Pipe)
        } else if upper.eq_ignore_ascii_case("alrm") || upper == "14" {
            Ok(Self::Alrm)
        } else if upper.eq_ignore_ascii_case("term") || upper == "15" {
            Ok(Self::Term)
        } else if upper.eq_ignore_ascii_case("chld") || upper == "17" {
            Ok(Self::Chld)
        } else if upper.eq_ignore_ascii_case("cont") || upper == "18" {
            Ok(Self::Cont)
        } else if upper.eq_ignore_ascii_case("stop") || upper == "19" {
            Ok(Self::Stop)
        } else if upper.eq_ignore_ascii_case("tstp") || upper == "20" {
            Ok(Self::Tstp)
        } else if upper.eq_ignore_ascii_case("ttin") || upper == "21" {
            Ok(Self::Ttin)
        } else if upper.eq_ignore_ascii_case("ttou") || upper == "22" {
            Ok(Self::Ttou)
        } else if upper.eq_ignore_ascii_case("urg") || upper == "23" {
            Ok(Self::Urg)
        } else if upper.eq_ignore_ascii_case("xcpu") || upper == "24" {
            Ok(Self::Xcpu)
        } else if upper.eq_ignore_ascii_case("xfsz") || upper == "25" {
            Ok(Self::Xfsz)
        } else if upper.eq_ignore_ascii_case("vtalrm") || upper == "26" {
            Ok(Self::Vtalrm)
        } else if upper.eq_ignore_ascii_case("prof") || upper == "27" {
            Ok(Self::Prof)
        } else if upper.eq_ignore_ascii_case("winch") || upper == "28" {
            Ok(Self::Winch)
        } else if upper.eq_ignore_ascii_case("io") || upper == "29" {
            Ok(Self::Io)
        } else if upper.eq_ignore_ascii_case("sys") || upper == "31" {
            Ok(Self::Sys)
        } else {
            Err(KdError::InvalidSignal(name.to_owned()))
        }
    }

    /// Whether this signal is an intentional termination request.
    #[must_use]
    pub const fn is_termination(self) -> bool {
        matches!(
            self,
            Self::Term | Self::Int | Self::Quit | Self::Hup | Self::Kill
        )
    }

    /// Whether this signal represents a fatal, non-recoverable error.
    #[must_use]
    pub const fn is_fatal(self) -> bool {
        matches!(
            self,
            Self::Kill
                | Self::Stop
                | Self::Segv
                | Self::Bus
                | Self::Ill
                | Self::Fpe
                | Self::Abrt
                | Self::Sys
        )
    }

    /// Whether the signal can be intercepted or caught by user processes.
    ///
    /// Per POSIX standards, `SIGKILL` and `SIGSTOP` cannot be caught or ignored.
    #[must_use]
    pub const fn can_catch(self) -> bool {
        !matches!(self, Self::Kill | Self::Stop)
    }

    /// Whether this signal notifies of child process state change (`SIGCHLD`).
    #[must_use]
    pub const fn is_child(self) -> bool {
        matches!(self, Self::Chld)
    }

    /// Whether this signal notifies of terminal window size change (`SIGWINCH`).
    #[must_use]
    pub const fn is_window_change(self) -> bool {
        matches!(self, Self::Winch)
    }

    /// Whether this signal is a user-defined signal (`SIGUSR1` or `SIGUSR2`).
    #[must_use]
    pub const fn is_user(self) -> bool {
        matches!(self, Self::Usr1 | Self::Usr2)
    }
}

impl fmt::Display for Signal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Signal {
    type Err = KdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_name(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_raw_roundtrip() {
        let signals = [
            Signal::Hup,
            Signal::Int,
            Signal::Quit,
            Signal::Kill,
            Signal::Term,
            Signal::Chld,
            Signal::Winch,
            Signal::Usr1,
            Signal::Usr2,
        ];
        for sig in signals {
            let raw = sig.as_raw();
            let parsed = Signal::from_raw(raw).expect("valid raw signal");
            assert_eq!(sig, parsed);
        }
    }

    #[test]
    fn test_signal_parsing_names() {
        assert_eq!(Signal::from_name("SIGTERM").unwrap(), Signal::Term);
        assert_eq!(Signal::from_name("term").unwrap(), Signal::Term);
        assert_eq!(Signal::from_name("2").unwrap(), Signal::Int);
        assert_eq!(Signal::from_name("sigint").unwrap(), Signal::Int);
        assert_eq!(Signal::from_name("9").unwrap(), Signal::Kill);
        assert!(Signal::from_name("INVALID_SIGNAL").is_err());
    }

    #[test]
    fn test_signal_categories() {
        assert!(Signal::Term.is_termination());
        assert!(Signal::Int.is_termination());
        assert!(Signal::Kill.is_termination());
        assert!(!Signal::Chld.is_termination());

        assert!(!Signal::Kill.can_catch());
        assert!(!Signal::Stop.can_catch());
        assert!(Signal::Term.can_catch());
        assert!(Signal::Int.can_catch());

        assert!(Signal::Chld.is_child());
        assert!(Signal::Winch.is_window_change());
        assert!(Signal::Usr1.is_user());
        assert!(Signal::Usr2.is_user());
    }
}
