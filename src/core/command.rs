//! Running external commands (`CommandUtil` in Stacer).

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// Run `cmd args...`, return trimmed stdout. Errors only if the process could not be started.
pub fn exec(cmd: &str, args: &[&str]) -> Result<String, String> {
    run(cmd, args, None, false)
}

/// Like [`exec`] but a non-zero exit code is an error.
pub fn exec_checked(cmd: &str, args: &[&str]) -> Result<String, String> {
    run(cmd, args, None, true)
}

/// Run a command as root through `pkexec` (graphical password prompt).
pub fn sudo_exec(cmd: &str, args: &[&str], stdin: Option<&[u8]>) -> Result<String, String> {
    let mut full = Vec::with_capacity(args.len() + 1);
    full.push(cmd);
    full.extend_from_slice(args);
    run("pkexec", &full, stdin, true)
}

fn run(cmd: &str, args: &[&str], stdin: Option<&[u8]>, check: bool) -> Result<String, String> {
    let mut child = Command::new(cmd)
        .args(args)
        .env("LC_ALL", "C") // stable, parseable output
        .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{cmd}: {e}"))?;

    if let (Some(data), Some(mut pipe)) = (stdin, child.stdin.take()) {
        pipe.write_all(data).map_err(|e| format!("{cmd}: {e}"))?;
    }

    let out = child.wait_with_output().map_err(|e| format!("{cmd}: {e}"))?;
    if check && !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() {
            format!("{cmd} exited with {}", out.status)
        } else {
            err
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Is `cmd` found in `$PATH`?
pub fn is_executable(cmd: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| is_exec_file(&dir.join(cmd))))
        .unwrap_or(false)
}

fn is_exec_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// ALT Linux style apt on top of rpm.
pub fn is_apt_rpm() -> bool {
    use std::sync::OnceLock;
    static CACHE: OnceLock<bool> = OnceLock::new();
    *CACHE.get_or_init(|| {
        if !is_executable("apt-get") || !is_executable("rpm") {
            return false;
        }
        if is_executable("apt-repo") {
            return true;
        }
        exec("apt-get", &["--version"])
            .map(|v| v.to_lowercase().contains("rpm interface"))
            .unwrap_or(false)
    })
}
