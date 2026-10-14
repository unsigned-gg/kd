//! Repo automation. Run with `cargo xtask <command>`.
//!
//! Commands:
//! - `spec-check`: validate `specs/` (see `spec_check.rs`).
//! - `toolchain-check`: verify the active Rust matches `rust-toolchain.toml`.
//! - `pin-check`: verify dependency pins against Cargo.lock (see `pin_check.rs`).
//! - `fetch-contracts`: fetch pinned estate contracts into `.contracts/` (see `fetch_contracts.rs`).
//! - `tools`, `fmt`, `fmt-check`, `clippy`, `test`, `doc-test`, `deny`, `typos`, `machete`,
//!   `ci` and `install`: the gates of spec 0002, XT-1 to XT-8 (see `gates.rs`).

mod fetch_contracts;
mod gates;
mod pin_check;
mod spec_check;
mod toolchain;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// The repo root: the parent of the `xtask` directory.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

fn main() -> ExitCode {
    let arg = std::env::args().nth(1);
    let root = repo_root();
    match arg.as_deref() {
        Some("spec-check") => spec_check::run(&root),
        Some("toolchain-check") => toolchain::run(&root),
        Some("pin-check") => pin_check::run(&root),
        Some("fetch-contracts") => {
            let rest: Vec<String> = std::env::args().skip(2).collect();
            fetch_contracts::run(&root, &rest)
        }
        other => match other.and_then(gates::Gate::parse) {
            Some(gate) => gates::run(&root, gate),
            None => {
                if let Some(name) = other {
                    eprintln!("xtask: unknown command {name:?}");
                }
                eprintln!("{}", usage());
                ExitCode::from(2)
            }
        },
    }
}

/// The usage line: the repo's own commands, then the gates of spec 0002.
fn usage() -> String {
    let mut names = vec!["spec-check", "pin-check", "fetch-contracts"];
    names.extend(gates::Gate::ALL.iter().map(|g| g.name()));
    format!("usage: cargo xtask <{}>", names.join("|"))
}
