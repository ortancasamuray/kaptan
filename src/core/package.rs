//! Installed packages and package caches (`PackageTool` in Stacer).

use super::command::{exec, is_apt_rpm, is_executable, sudo_exec};
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tool {
    Apt,
    AptRpm,
    Dnf,
    Yum,
    Pacman,
    Zypper,
    Unknown,
}

pub fn current() -> Tool {
    static T: OnceLock<Tool> = OnceLock::new();
    *T.get_or_init(|| {
        if is_apt_rpm() {
            Tool::AptRpm
        } else if is_executable("apt-get") {
            Tool::Apt
        } else if is_executable("dnf") {
            Tool::Dnf
        } else if is_executable("yum") {
            Tool::Yum
        } else if is_executable("pacman") {
            Tool::Pacman
        } else if is_executable("zypper") {
            Tool::Zypper
        } else {
            Tool::Unknown
        }
    })
}

pub fn packages() -> Vec<String> {
    let mut v: Vec<String> = match current() {
        Tool::Apt => exec("dpkg", &["--get-selections"])
            .unwrap_or_default()
            .lines()
            .filter(|l| l.split_whitespace().nth(1) == Some("install"))
            .filter_map(|l| l.split_whitespace().next().map(str::to_string))
            .collect(),
        Tool::AptRpm | Tool::Dnf | Tool::Yum | Tool::Zypper => {
            exec("rpm", &["-qa", "--qf", "%{NAME}\\n"]).unwrap_or_default().lines().map(str::to_string).collect()
        }
        Tool::Pacman => exec("pacman", &["-Qq"]).unwrap_or_default().lines().map(str::to_string).collect(),
        Tool::Unknown => vec![],
    };
    v.retain(|s| !s.is_empty());
    v.sort();
    v
}

pub fn snap_available() -> bool {
    is_executable("snap")
}

pub fn snap_packages() -> Vec<String> {
    if !snap_available() {
        return vec![];
    }
    exec("snap", &["list"])
        .unwrap_or_default()
        .lines()
        .skip(1)
        .filter_map(|l| l.split_whitespace().next().map(str::to_string))
        .collect()
}

pub fn uninstall(pkgs: &[String]) -> Result<(), String> {
    let p: Vec<&str> = pkgs.iter().map(String::as_str).collect();
    let (cmd, mut args): (&str, Vec<&str>) = match current() {
        Tool::Apt | Tool::AptRpm => ("apt-get", vec!["remove", "-y"]),
        Tool::Dnf => ("dnf", vec!["remove", "-y"]),
        Tool::Yum => ("yum", vec!["remove", "-y"]),
        Tool::Zypper => ("zypper", vec!["--non-interactive", "remove"]),
        Tool::Pacman => ("pacman", vec!["--noconfirm", "-R"]),
        Tool::Unknown => return Err("No supported package manager found".into()),
    };
    args.extend(p);
    sudo_exec(cmd, &args, None).map(|_| ())
}

/// Packages that removing `pkgs` would actually remove (dependents included), without changing
/// anything. Falls back to `pkgs` when the package manager has no dry-run we can parse.
pub fn simulate_removal(pkgs: &[String]) -> Result<Vec<String>, String> {
    let p: Vec<&str> = pkgs.iter().map(String::as_str).collect();
    match current() {
        Tool::Apt | Tool::AptRpm => {
            let mut args = vec!["-s", "remove"];
            args.extend(&p);
            let out = super::command::exec_checked("apt-get", &args)?;
            Ok(parse_apt_simulation(&out))
        }
        Tool::Pacman => {
            let mut args = vec!["-Rp", "--print-format", "%n"];
            args.extend(&p);
            let out = super::command::exec_checked("pacman", &args)?;
            Ok(out.lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_string).collect())
        }
        _ => Ok(pkgs.to_vec()),
    }
}

fn parse_apt_simulation(out: &str) -> Vec<String> {
    out.lines()
        .filter_map(|l| l.strip_prefix("Remv "))
        .filter_map(|l| l.split_whitespace().next().map(str::to_string))
        .collect()
}

pub fn uninstall_snaps(pkgs: &[String]) -> Result<(), String> {
    let mut args = vec!["remove"];
    args.extend(pkgs.iter().map(String::as_str));
    sudo_exec("snap", &args, None).map(|_| ())
}

pub fn cache_files() -> Vec<PathBuf> {
    let dir = match current() {
        Tool::Apt | Tool::AptRpm => "/var/cache/apt/archives/",
        Tool::Pacman => "/var/cache/pacman/pkg/",
        Tool::Dnf => "/var/cache/dnf/",
        Tool::Yum => "/var/cache/yum/",
        Tool::Zypper => "/var/cache/zypp/packages/",
        Tool::Unknown => return vec![],
    };
    super::fsutil::list_dir(dir, matches!(current(), Tool::Apt | Tool::AptRpm | Tool::Pacman))
}

#[cfg(test)]
mod tests {
    #[test]
    fn apt_simulation() {
        let out = "Reading package lists...\nThe following packages will be REMOVED:\n  libfoo1 foo\nRemv foo [1.0-1]\nRemv libfoo1 [1.0-1] [foo:amd64 ]\n";
        assert_eq!(super::parse_apt_simulation(out), vec!["foo", "libfoo1"]);
    }
}
