//! Persistent user settings (~/.config/kaptan/settings.json).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// "dark" | "light"
    pub theme: String,
    /// "" = follow the system, "en", "tr"
    pub language: String,
    pub start_page: String,
    /// Mount point shown on the dashboard disk bar ("" = /)
    pub disk_name: String,
    /// 0 = alert disabled
    pub cpu_alert_percent: i32,
    pub memory_alert_percent: i32,
    pub disk_alert_percent: i32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            language: String::new(),
            start_page: "dashboard".into(),
            disk_name: String::new(),
            cpu_alert_percent: 0,
            memory_alert_percent: 0,
            disk_alert_percent: 0,
        }
    }
}

fn path() -> PathBuf {
    super::fsutil::config_dir().join("kaptan/settings.json")
}

impl Settings {
    pub fn load() -> Self {
        std::fs::read_to_string(path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    pub fn save(&self) {
        let p = path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(p, s);
        }
    }
}

/// Autostart entry for Kaptan itself.
pub fn self_autostart_path() -> PathBuf {
    super::startup::autostart_dir().join("kaptan.desktop")
}

pub fn is_self_autostart() -> bool {
    self_autostart_path().exists()
}

pub fn set_self_autostart(on: bool) -> std::io::Result<()> {
    let p = self_autostart_path();
    if on {
        std::fs::create_dir_all(super::startup::autostart_dir())?;
        let exe = std::env::current_exe().map(|e| e.display().to_string()).unwrap_or_else(|_| "kaptan".into());
        std::fs::write(
            p,
            format!("[Desktop Entry]\nType=Application\nName=Kaptan\nComment=System optimizer and monitor\nExec={exe}\nIcon=kaptan\nTerminal=false\nX-GNOME-Autostart-enabled=true\n"),
        )
    } else if p.exists() {
        std::fs::remove_file(p)
    } else {
        Ok(())
    }
}

/// Desktop notification through `notify-send` (Stacer used the tray icon).
pub fn notify(title: &str, body: &str) {
    if super::command::is_executable("notify-send") {
        let _ = std::process::Command::new("notify-send")
            .args(["-a", "Kaptan", "-u", "critical", title, body])
            .spawn();
    }
}
