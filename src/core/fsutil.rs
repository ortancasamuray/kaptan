//! File helpers (`FileUtil` in Stacer).

use std::fs;
use std::path::{Path, PathBuf};

pub fn read_string(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

pub fn read_lines(path: impl AsRef<Path>) -> Vec<String> {
    read_string(path).trim().lines().map(str::to_string).collect()
}

/// Total size of a file or (recursively) a directory. Symlinks are not followed.
pub fn size_of(path: impl AsRef<Path>) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path.as_ref()) else { return 0 };
    if meta.is_dir() {
        fs::read_dir(path.as_ref())
            .map(|rd| rd.flatten().map(|e| size_of(e.path())).sum())
            .unwrap_or(0)
    } else {
        meta.len()
    }
}

/// Entries of a directory (files only when `files_only`).
pub fn list_dir(path: impl AsRef<Path>, files_only: bool) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(path)
        .map(|rd| {
            rd.flatten()
                .filter(|e| !files_only || e.file_type().map(|t| t.is_file()).unwrap_or(false))
                .map(|e| e.path())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

pub fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| "/".into())
}

pub fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home().join(".config"))
}
