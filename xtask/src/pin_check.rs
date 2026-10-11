//! `cargo xtask pin-check` (spec 0004).
//!
//! The `| Crate | Version |` tables in `specs/0004-dependency-and-licence-policy.md` bind the
//! lockfiles. A crate that a table pins and a manifest declares directly must be locked at the
//! pinned version. The workspace members are checked against `Cargo.lock`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};

const SPEC: &str = "specs/0004-dependency-and-licence-policy.md";
const CI_YML: &str = ".github/workflows/ci.yml";
const LEFTHOOK_YML: &str = "lefthook.yml";

/// The result of one check.
#[derive(Debug, PartialEq, Eq)]
pub struct Report {
    /// How many pinned crates a manifest declares, and a lock was asked about.
    pub checked: usize,
    /// One line per pinned crate that the lock does not hold at the pinned version.
    pub misses: Vec<String>,
}

pub fn run(root: &Path) -> ExitCode {
    match check_root(root) {
        Ok(report) if report.misses.is_empty() => {
            println!("pin-check: ok ({} pinned crates checked)", report.checked);
            ExitCode::SUCCESS
        }
        Ok(report) => {
            for miss in &report.misses {
                eprintln!("{miss}");
            }
            ExitCode::FAILURE
        }
        Err(err) => {
            eprintln!("pin-check: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))
}

fn check_root(root: &Path) -> Result<Report> {
    let spec = read(&root.join(SPEC))?;
    let root_manifest = read(&root.join("Cargo.toml"))?;
    let mut manifests = vec![root_manifest.clone()];
    for dir in member_dirs(root, &root_manifest)? {
        manifests.push(read(&dir.join("Cargo.toml"))?);
    }
    let lock = read(&root.join("Cargo.lock"))?;
    let mut report = check_repo(&spec, &manifests, &lock);
    if let Ok(ci_yml) = read(&root.join(CI_YML)) {
        if let Ok(lefthook_yml) = read(&root.join(LEFTHOOK_YML)) {
            let gitleaks = check_gitleaks(&spec, &ci_yml, &lefthook_yml);
            if gitleaks.is_empty() {
                if let Some(version) = parse_tool_version(&spec, "gitleaks") {
                    println!("pin-check: gitleaks ok ({version})");
                }
            }
            report.misses.extend(gitleaks);
        }
    }
    Ok(report)
}

/// The first `X.Y.Z` token of the version cell of the `| Tool | Version |` row whose first cell
/// is `tool` in backticks.
pub fn parse_tool_version(spec: &str, tool: &str) -> Option<String> {
    let want = format!("`{tool}`");
    let mut in_table = false;
    for raw in spec.lines() {
        let line = raw.trim();
        if !line.starts_with('|') {
            in_table = false;
            continue;
        }
        let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
        if cells.len() >= 2 && cells[0] == "Tool" && cells[1] == "Version" {
            in_table = true;
            continue;
        }
        if in_table && cells.len() >= 2 && cells[0] == want {
            return first_xyz(cells[1]);
        }
    }
    None
}

/// The first digits-only `X.Y.Z` token of `text`.
fn first_xyz(text: &str) -> Option<String> {
    text.split(|c: char| !c.is_ascii_digit() && c != '.')
        .find(|t| is_exact_version(t))
        .map(str::to_owned)
}

/// The version that follows `marker` in `text`, when it is an `X.Y.Z`.
fn version_after(text: &str, marker: &str) -> Option<String> {
    let rest = text.split_once(marker)?.1.trim_start();
    let end = rest
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(rest.len());
    Some(rest[..end].to_owned()).filter(|v| is_exact_version(v))
}

/// One line per file whose gitleaks version differs from the `gitleaks` row of the CI tools
/// table of spec 0004.
pub fn check_gitleaks(spec: &str, ci_yml: &str, lefthook_yml: &str) -> Vec<String> {
    let pinned = parse_tool_version(spec, "gitleaks");
    if let Some(want) = pinned {
        let files = [
            (CI_YML, version_after(ci_yml, "GITLEAKS_VERSION:")),
            (LEFTHOOK_YML, version_after(lefthook_yml, "install v")),
        ];
        files
            .into_iter()
            .filter(|(_, have)| have.as_ref() != Some(&want))
            .map(|(file, have)| {
                let have = have.as_deref().unwrap_or("none");
                format!(
                    "pin-check: gitleaks pinned {want} (specs/0004 CI tools), {file} has {have}"
                )
            })
            .collect()
    } else {
        Vec::new()
    }
}

/// The member directories of the root workspace: one directory level under each `dir/*` entry.
fn member_dirs(root: &Path, root_manifest: &str) -> Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();
    for member in members(root_manifest) {
        if let Some(parent) = member.strip_suffix("/*") {
            let base = root.join(parent);
            if base.is_dir() {
                let entries = fs::read_dir(&base)
                    .with_context(|| format!("cannot read {}", base.display()))?;
                for entry in entries {
                    let path = entry?.path();
                    if path.join("Cargo.toml").is_file() {
                        dirs.push(path);
                    }
                }
            }
        } else {
            let path = root.join(&member);
            if path.join("Cargo.toml").is_file() {
                dirs.push(path);
            }
        }
    }
    dirs.sort();
    Ok(dirs)
}

/// The quoted strings of the `members = [...]` line of `[workspace]`.
pub fn members(manifest: &str) -> Vec<String> {
    let mut in_workspace = false;
    for raw in manifest.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_workspace = line == "[workspace]";
            continue;
        }
        if !in_workspace {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            if key.trim() == "members" {
                return value
                    .split('"')
                    .skip(1)
                    .step_by(2)
                    .map(str::to_owned)
                    .collect();
            }
        }
    }
    Vec::new()
}

/// Check the workspace manifests against the workspace lock.
pub fn check_repo(spec: &str, manifests: &[String], lock: &str) -> Report {
    let pins = parse_pins(spec);
    let workspace: BTreeSet<String> = manifests.iter().flat_map(|m| direct_deps(m)).collect();
    let mut report = Report {
        checked: 0,
        misses: Vec::new(),
    };
    let locked = parse_lock(lock);
    for (name, version) in &pins {
        if !workspace.contains(name) {
            continue;
        }
        report.checked += 1;
        let have = locked.get(name).map_or(&[][..], Vec::as_slice);
        if !have.contains(version) {
            let list = if have.is_empty() {
                "none".to_owned()
            } else {
                have.join(", ")
            };
            report.misses.push(format!(
                "pin-check: {name} pinned {version} (specs/0004), Cargo.lock has {list}"
            ));
        }
    }
    report
}

/// The `(crate, version)` pairs of every `| Crate | Version |` table. Only `X.Y.Z` or `X.Y.Z+build` versions count.
pub fn parse_pins(spec: &str) -> Vec<(String, String)> {
    let mut pins: Vec<(String, String)> = Vec::new();
    let mut in_table = false;
    for raw in spec.lines() {
        let line = raw.trim();
        if !line.starts_with('|') {
            in_table = false;
            continue;
        }
        let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
        if cells.len() >= 2 && cells[0] == "Crate" && cells[1] == "Version" {
            in_table = true;
            continue;
        }
        if !in_table || cells.len() < 2 || cells[0].starts_with("---") {
            continue;
        }
        let names = cells[0].split(',').map(|n| n.trim().trim_matches('`'));
        let versions = cells[1].split(',').map(str::trim);
        for (name, version) in names.zip(versions) {
            if is_exact_version(version) && !name.is_empty() {
                let pair = (name.to_owned(), version.to_owned());
                if !pins.contains(&pair) {
                    pins.push(pair);
                }
            }
        }
    }
    pins
}

/// True for `X.Y.Z` with digits only, optionally followed by `+` and a build
/// string of ASCII letters, digits, `.` and `-`.
fn is_exact_version(text: &str) -> bool {
    let (core, build) = match text.split_once('+') {
        Some((core, build)) => (core, Some(build)),
        None => (text, None),
    };
    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() != 3
        || !parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
    {
        return false;
    }
    build.is_none_or(|b| {
        !b.is_empty()
            && b.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    })
}

/// The crate names declared directly under `[dependencies]`, `[dev-dependencies]`,
/// `[build-dependencies]` and `[workspace.dependencies]`.
pub fn direct_deps(manifest: &str) -> BTreeSet<String> {
    let mut deps = BTreeSet::new();
    let mut in_deps = false;
    for raw in manifest.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            let section = line.trim_matches(|c| c == '[' || c == ']');
            in_deps = matches!(
                section,
                "dependencies"
                    | "dev-dependencies"
                    | "build-dependencies"
                    | "workspace.dependencies"
            ) || section.starts_with("dependencies.")
                || section.starts_with("dev-dependencies.")
                || section.starts_with("build-dependencies.")
                || section.starts_with("target.");
            if in_deps && section.contains(".dependencies.") {
                if let Some(crate_name) = section.rsplit('.').next() {
                    deps.insert(crate_name.to_owned());
                }
            }
            continue;
        }
        if !in_deps {
            continue;
        }
        if let Some((name, _)) = line.split_once('=') {
            let name = name.trim();
            if !name.is_empty() && !name.starts_with('#') {
                deps.insert(name.to_owned());
            }
        }
    }
    deps
}

/// Map each crate name in `Cargo.lock` to the list of versions locked for it.
pub fn parse_lock(lock: &str) -> BTreeMap<String, Vec<String>> {
    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut current_name: Option<String> = None;
    for raw in lock.lines() {
        let line = raw.trim();
        if line == "[[package]]" {
            current_name = None;
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim().trim_matches('"');
            match key {
                "name" => current_name = Some(value.to_owned()),
                "version" => {
                    if let Some(name) = current_name.take() {
                        map.entry(name).or_default().push(value.to_owned());
                    }
                }
                _ => {}
            }
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_check_parses_pins_from_markdown_table() {
        let spec = r#"
## Dependencies

| Crate | Version | Why |
|---|---|---|
| `anyhow` | 1.0.104 | error context |
| `clap` | 4.6.7 | cli parsing |
"#;
        let pins = parse_pins(spec);
        assert_eq!(
            pins,
            vec![
                ("anyhow".to_string(), "1.0.104".to_string()),
                ("clap".to_string(), "4.6.7".to_string()),
            ]
        );
    }

    #[test]
    fn pin_check_checks_direct_dependencies() {
        let spec = "| Crate | Version |\n| `anyhow` | 1.0.104 |\n";
        let manifest = "[dependencies]\nanyhow = \"1\"\n";
        let lock = "[[package]]\nname = \"anyhow\"\nversion = \"1.0.104\"\n";
        let report = check_repo(spec, &[manifest.to_string()], lock);
        assert_eq!(report.checked, 1);
        assert!(report.misses.is_empty());
    }
}
