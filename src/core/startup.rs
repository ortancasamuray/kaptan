//! Autostart entries in ~/.config/autostart (Startup Apps page in Stacer).

use super::desktop_file as df;
use super::fsutil::{config_dir, read_lines};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct StartupApp {
    pub name: String,
    pub enabled: bool,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Default)]
pub struct StartupAppFields {
    pub name: String,
    pub generic_name: String,
    pub comment: String,
    pub icon: String,
    pub command: String,
}

pub fn autostart_dir() -> PathBuf {
    config_dir().join("autostart")
}

pub fn list() -> Vec<StartupApp> {
    let dir = autostart_dir();
    let _ = fs::create_dir_all(&dir);
    let mut apps: Vec<StartupApp> = super::fsutil::list_dir(&dir, true)
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "desktop"))
        .filter_map(|path| {
            let lines = read_lines(&path);
            let name = df::get(&lines, "Name");
            if name.is_empty() {
                return None;
            }
            let hidden = df::get(&lines, "Hidden").to_lowercase();
            let enabled = if !hidden.is_empty() {
                hidden != "true"
            } else {
                df::get(&lines, "X-GNOME-Autostart-enabled").to_lowercase() != "false"
            };
            Some(StartupApp { name, enabled, path })
        })
        .collect();
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

pub fn set_enabled(path: &Path, enabled: bool) -> std::io::Result<()> {
    let mut lines = read_lines(path);
    if df::has(&lines, "X-GNOME-Autostart-enabled") {
        df::set(&mut lines, "X-GNOME-Autostart-enabled", if enabled { "true" } else { "false" });
    }
    df::set(&mut lines, "Hidden", if enabled { "false" } else { "true" });
    fs::write(path, lines.join("\n") + "\n")
}

pub fn remove(path: &Path) -> std::io::Result<()> {
    fs::remove_file(path)
}

pub fn read_fields(path: &Path) -> StartupAppFields {
    let l = read_lines(path);
    StartupAppFields {
        name: df::get(&l, "Name"),
        generic_name: df::get(&l, "GenericName"),
        comment: df::get(&l, "Comment"),
        icon: df::get(&l, "Icon"),
        command: df::get(&l, "Exec"),
    }
}

/// Create a new entry (`path == None`) or update an existing one.
pub fn save(path: Option<&Path>, f: &StartupAppFields) -> Result<(), String> {
    if f.name.trim().is_empty() || f.command.trim().is_empty() {
        return Err("Name and command fields are required.".into());
    }
    match path {
        Some(p) => {
            let mut lines = read_lines(p);
            df::set(&mut lines, "Name", &f.name);
            df::set(&mut lines, "GenericName", &f.generic_name);
            df::set(&mut lines, "Comment", &f.comment);
            df::set(&mut lines, "Icon", &f.icon);
            df::set(&mut lines, "Exec", &f.command);
            fs::write(p, lines.join("\n") + "\n").map_err(|e| e.to_string())
        }
        None => {
            let content = format!(
                "[Desktop Entry]\nType=Application\nName={}\nGenericName={}\nComment={}\nIcon={}\nHidden=false\nExec={}\nTerminal=false\nX-GNOME-Autostart-enabled=true\nX-GNOME-Autostart-Delay=5\n",
                f.name, f.generic_name, f.comment, f.icon, f.command
            );
            let base: String = f
                .name
                .split_whitespace()
                .collect::<Vec<_>>()
                .join("-")
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                .collect();
            let dir = autostart_dir();
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let mut p = dir.join(format!("{base}.desktop"));
            let mut n = 1;
            while p.exists() {
                p = dir.join(format!("{base}-{n}.desktop"));
                n += 1;
            }
            fs::write(p, content).map_err(|e| e.to_string())
        }
    }
}
