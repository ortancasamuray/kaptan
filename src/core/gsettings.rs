//! GNOME desktop tweaks through the `gsettings` CLI.
//! Stacer's Unity/Compiz pages are obsolete; these are the modern GNOME equivalents.

use super::command::{exec, is_executable};
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub enum Kind {
    Bool,
    Int { min: i32, max: i32 },
    /// (label, value) pairs
    Enum(Vec<(&'static str, &'static str)>),
}

#[derive(Clone, Debug)]
pub struct Setting {
    pub group: &'static str,
    pub label: &'static str,
    pub schema: &'static str,
    pub key: &'static str,
    pub kind: Kind,
}

fn titlebar_actions() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Maximize", "toggle-maximize"),
        ("Maximize Horizontally", "toggle-maximize-horizontally"),
        ("Maximize Vertically", "toggle-maximize-vertically"),
        ("Minimize", "minimize"),
        ("Lower", "lower"),
        ("Menu", "menu"),
        ("None", "none"),
    ]
}

fn picture_options() -> Vec<(&'static str, &'static str)> {
    vec![
        ("None", "none"),
        ("Wallpaper", "wallpaper"),
        ("Centered", "centered"),
        ("Scaled", "scaled"),
        ("Stretched", "stretched"),
        ("Zoom", "zoom"),
        ("Spanned", "spanned"),
    ]
}

pub fn all_settings() -> Vec<Setting> {
    use Kind::*;
    let s = |group, label, schema, key, kind| Setting { group, label, schema, key, kind };
    vec![
        s("Appearance", "Desktop background mode", "org.gnome.desktop.background", "picture-options", Enum(picture_options())),
        s("Appearance", "Lock screen background mode", "org.gnome.desktop.screensaver", "picture-options", Enum(picture_options())),
        s("Appearance", "Show home folder on desktop", "org.gnome.shell.extensions.ding", "show-home", Bool),
        s("Appearance", "Show trash on desktop", "org.gnome.shell.extensions.ding", "show-trash", Bool),
        s("Appearance", "Show mounted volumes on desktop", "org.gnome.shell.extensions.ding", "show-volumes", Bool),
        s("Appearance", "Show battery percentage", "org.gnome.desktop.interface", "show-battery-percentage", Bool),
        s("Appearance", "Show seconds in clock", "org.gnome.desktop.interface", "clock-show-seconds", Bool),
        s("Appearance", "Show weekday in clock", "org.gnome.desktop.interface", "clock-show-weekday", Bool),
        s("Appearance", "Enable animations", "org.gnome.desktop.interface", "enable-animations", Bool),
        s("Accessibility", "On-screen keyboard", "org.gnome.desktop.a11y.applications", "screen-keyboard-enabled", Bool),
        s("Accessibility", "Screen reader", "org.gnome.desktop.a11y.applications", "screen-reader-enabled", Bool),
        s("Window Manager", "Focus mode", "org.gnome.desktop.wm.preferences", "focus-mode", Enum(vec![("Click", "click"), ("Sloppy", "sloppy"), ("Mouse", "mouse")])),
        s("Window Manager", "Raise windows on click", "org.gnome.desktop.wm.preferences", "raise-on-click", Bool),
        s("Window Manager", "Title bar double click", "org.gnome.desktop.wm.preferences", "action-double-click-titlebar", Enum(titlebar_actions())),
        s("Window Manager", "Title bar middle click", "org.gnome.desktop.wm.preferences", "action-middle-click-titlebar", Enum(titlebar_actions())),
        s("Window Manager", "Title bar right click", "org.gnome.desktop.wm.preferences", "action-right-click-titlebar", Enum(titlebar_actions())),
        s("Window Manager", "Dynamic workspaces", "org.gnome.mutter", "dynamic-workspaces", Bool),
        s("Window Manager", "Number of workspaces", "org.gnome.desktop.wm.preferences", "num-workspaces", Int { min: 1, max: 36 }),
        s("Window Manager", "Center new windows", "org.gnome.mutter", "center-new-windows", Bool),
    ]
}

pub fn available() -> bool {
    is_executable("gsettings")
}

/// Settings whose schema and key exist on this system.
pub fn supported_settings() -> Vec<Setting> {
    if !available() {
        return vec![];
    }
    let schemas: HashSet<String> = exec("gsettings", &["list-schemas"]).unwrap_or_default().lines().map(str::to_string).collect();
    all_settings()
        .into_iter()
        .filter(|s| schemas.contains(s.schema))
        .filter(|s| exec("gsettings", &["writable", s.schema, s.key]).map(|v| v == "true").unwrap_or(false))
        .collect()
}

/// Raw value with quotes stripped (`'click'` -> `click`, `uint32 4` -> `4`).
pub fn get(schema: &str, key: &str) -> String {
    let v = exec("gsettings", &["get", schema, key]).unwrap_or_default();
    let v = v.rsplit(' ').next().unwrap_or_default();
    v.trim_matches('\'').to_string()
}

pub fn set(schema: &str, key: &str, value: &str) -> Result<(), String> {
    super::command::exec_checked("gsettings", &["set", schema, key, value]).map(|_| ())
}
