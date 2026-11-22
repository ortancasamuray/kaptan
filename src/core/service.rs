//! systemd services (`ServiceTool` in Stacer).

use super::command::{exec, sudo_exec};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Service {
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub active: bool,
}

pub fn list() -> Vec<Service> {
    let files = exec("systemctl", &["list-unit-files", "-t", "service", "-a", "--state=enabled,disabled", "--no-legend", "--no-pager"])
        .unwrap_or_default();

    // One `systemctl show` call for all units instead of two calls per unit.
    let units: Vec<(String, bool)> = files
        .lines()
        .filter_map(|l| {
            let mut f = l.split_whitespace();
            let unit = f.next()?.to_string();
            let state = f.next()?;
            (!unit.contains('@')).then(|| (unit, state == "enabled"))
        })
        .collect();

    let mut info: HashMap<String, (String, String)> = HashMap::new();
    if !units.is_empty() {
        let mut args = vec!["show", "--property=Id,Description,ActiveState", "--no-pager"];
        args.extend(units.iter().map(|(u, _)| u.as_str()));
        let out = exec("systemctl", &args).unwrap_or_default();
        for block in out.split("\n\n") {
            let mut id = String::new();
            let mut desc = String::new();
            let mut state = String::new();
            for line in block.lines() {
                match line.split_once('=') {
                    Some(("Id", v)) => id = v.into(),
                    Some(("Description", v)) => desc = v.into(),
                    Some(("ActiveState", v)) => state = v.into(),
                    _ => {}
                }
            }
            info.insert(id, (desc, state));
        }
    }

    units
        .into_iter()
        .map(|(unit, enabled)| {
            let (desc, state) = info.get(&unit).cloned().unwrap_or_default();
            Service {
                name: unit.trim_end_matches(".service").to_string(),
                description: if desc.is_empty() { "Unknown".into() } else { desc },
                enabled,
                active: state == "active",
            }
        })
        .collect()
}

pub fn set_enabled(name: &str, enabled: bool) -> Result<(), String> {
    sudo_exec("systemctl", &[if enabled { "enable" } else { "disable" }, name], None).map(|_| ())
}

pub fn set_running(name: &str, running: bool) -> Result<(), String> {
    sudo_exec("systemctl", &[if running { "start" } else { "stop" }, name], None).map(|_| ())
}

pub fn is_active(name: &str) -> bool {
    exec("systemctl", &["is-active", name]).map(|s| s == "active").unwrap_or(false)
}

pub fn is_enabled(name: &str) -> bool {
    exec("systemctl", &["is-enabled", name]).map(|s| s == "enabled").unwrap_or(false)
}
