//! Application icon lookup in the user's icon theme (Stacer used QIcon::fromTheme).

use super::command::{exec, is_executable};
use super::fsutil::home;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Current icon theme name (XFCE, then GNOME).
fn theme_name() -> Option<String> {
    let clean = |s: String| Some(s.trim().trim_matches('\'').to_string()).filter(|s| !s.is_empty());
    if is_executable("xfconf-query") {
        if let Some(t) = exec("xfconf-query", &["-c", "xsettings", "-p", "/Net/IconThemeName"]).ok().and_then(clean) {
            return Some(t);
        }
    }
    if is_executable("gsettings") {
        return exec("gsettings", &["get", "org.gnome.desktop.interface", "icon-theme"]).ok().and_then(clean);
    }
    None
}

/// icon name -> file, built once from the theme's app icon folders (first match wins).
fn index() -> &'static HashMap<String, PathBuf> {
    static INDEX: OnceLock<HashMap<String, PathBuf>> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut themes: Vec<String> = theme_name().into_iter().collect();
        themes.push("hicolor".into());
        let bases = [home().join(".local/share/icons"), PathBuf::from("/usr/share/icons")];
        let sizes = ["48x48", "64x64", "scalable", "32x32", "128x128", "256x256"];

        let mut dirs = Vec::new();
        for theme in &themes {
            for base in &bases {
                for size in sizes {
                    dirs.push(base.join(theme).join(size).join("apps"));
                }
            }
        }
        dirs.push(PathBuf::from("/usr/share/pixmaps"));

        let mut map = HashMap::new();
        for dir in dirs {
            let Ok(rd) = std::fs::read_dir(&dir) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                let ext = p.extension().and_then(|e| e.to_str()).unwrap_or_default();
                if ext != "svg" && ext != "png" {
                    continue;
                }
                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    map.entry(stem.to_lowercase()).or_insert(p);
                }
            }
        }
        map
    })
}

/// Icon for an application/package name, e.g. "chromium", "firefox-esr", "Code".
pub fn lookup(name: &str) -> Option<PathBuf> {
    let name = name.trim().trim_start_matches('.').to_lowercase();
    if name.is_empty() {
        return None;
    }
    let idx = index();
    let candidates = [
        name.clone(),
        name.replace(' ', "-"),
        // "org.mozilla.firefox" style and "foo-bin" / "foo-esr" variants
        name.rsplit('.').next().unwrap_or(&name).to_string(),
        name.split(['-', '_']).next().unwrap_or(&name).to_string(),
    ];
    candidates.iter().find_map(|c| idx.get(c).cloned())
}

#[cfg(test)]
mod tests {
    #[test]
    fn empty_names() {
        assert!(super::lookup("").is_none());
        assert!(super::lookup("...").is_none());
    }
}
