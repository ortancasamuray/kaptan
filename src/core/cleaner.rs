//! System cleaner: scan and remove caches, logs, crash reports, trash (System Cleaner page in Stacer).

use super::command::sudo_exec;
use super::fsutil::{home, list_dir, size_of};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Category {
    PackageCache,
    CrashReports,
    AppLogs,
    AppCaches,
    Trash,
}

impl Category {
    pub const ALL: [Category; 5] = [Self::PackageCache, Self::CrashReports, Self::AppLogs, Self::AppCaches, Self::Trash];

    pub fn title(self) -> &'static str {
        match self {
            Self::PackageCache => "Package Caches",
            Self::CrashReports => "Crash Reports",
            Self::AppLogs => "Application Logs",
            Self::AppCaches => "Application Caches",
            Self::Trash => "Trash",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Item {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
}

#[derive(Clone, Debug)]
pub struct ScanGroup {
    pub category: Category,
    pub items: Vec<Item>,
}

impl ScanGroup {
    pub fn total(&self) -> u64 {
        self.items.iter().map(|i| i.size).sum()
    }
}

fn trash_dir() -> PathBuf {
    home().join(".local/share/Trash")
}

fn paths_for(cat: Category) -> Vec<PathBuf> {
    match cat {
        Category::PackageCache => super::package::cache_files(),
        Category::CrashReports => list_dir("/var/crash", true),
        Category::AppLogs => list_dir("/var/log", true),
        Category::AppCaches => app_caches(),
        Category::Trash => {
            let t = trash_dir();
            ["files", "info", "expunged"].iter().map(|d| t.join(d)).filter(|p| p.exists()).collect()
        }
    }
}

fn app_caches() -> Vec<PathBuf> {
    let h = home();
    let exclusions = [h.join(".cache/p10k")];
    let mut v = list_dir(h.join(".cache"), false);

    for rel in [".npm/_cacache", ".bun/install/cache", ".m2/repository", ".gradle/caches", ".cargo/registry", ".expo/versions-cache", ".expo/native-modules-cache"] {
        let p = h.join(rel);
        if p.is_dir() {
            v.push(p);
        }
    }
    // Electron / Chromium style caches: ~/.config/<app>/{Cache,GPUCache,Code Cache,DawnCache}
    for app in list_dir(h.join(".config"), false) {
        for sub in ["Cache", "GPUCache", "Code Cache", "DawnCache"] {
            let p = app.join(sub);
            if p.is_dir() {
                v.push(p);
            }
        }
    }
    v.retain(|p| !exclusions.iter().any(|e| p.starts_with(e)));
    v
}

/// Blocking: walks directories to compute sizes.
pub fn scan(categories: &[Category]) -> Vec<ScanGroup> {
    categories
        .iter()
        .map(|&category| {
            let mut items: Vec<Item> = paths_for(category)
                .into_iter()
                .map(|path| {
                    let name = if category == Category::AppCaches || category == Category::Trash {
                        path.strip_prefix(home()).map(|p| format!("~/{}", p.display())).unwrap_or_else(|_| path.display().to_string())
                    } else {
                        path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
                    };
                    Item { size: size_of(&path), name, path }
                })
                .collect();
            items.sort_by(|a, b| b.size.cmp(&a.size));
            ScanGroup { category, items }
        })
        .collect()
}

/// Remove the given paths. Tries without privileges first, then one pkexec call for the rest.
/// Returns the number of bytes freed.
pub fn clean(paths: &[PathBuf]) -> Result<u64, String> {
    let trash = trash_dir();
    let mut freed = 0;
    let mut need_root: Vec<&Path> = Vec::new();

    for p in paths {
        let size = size_of(p);
        // Trash sub-folders and cache dirs: empty them, keep the folder itself.
        let keep_dir = p.starts_with(&trash) || p.starts_with(home());
        let res = if p.is_dir() && keep_dir {
            empty_dir(p)
        } else if p.is_dir() {
            std::fs::remove_dir_all(p)
        } else {
            std::fs::remove_file(p)
        };
        match res {
            Ok(()) => freed += size,
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => need_root.push(p),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => freed += size.saturating_sub(size_of(p)),
        }
    }

    if !need_root.is_empty() {
        let before: u64 = need_root.iter().map(size_of).sum();
        let mut args: Vec<String> = vec!["-rf".into(), "--".into()];
        args.extend(need_root.iter().map(|p| p.to_string_lossy().into_owned()));
        let a: Vec<&str> = args.iter().map(String::as_str).collect();
        sudo_exec("rm", &a, None)?;
        freed += before;
    }
    Ok(freed)
}

fn empty_dir(dir: &Path) -> std::io::Result<()> {
    let mut first_err = None;
    for e in std::fs::read_dir(dir)?.flatten() {
        let p = e.path();
        let r = if e.file_type().map(|t| t.is_dir()).unwrap_or(false) { std::fs::remove_dir_all(&p) } else { std::fs::remove_file(&p) };
        if let Err(err) = r {
            first_err.get_or_insert(err);
        }
    }
    first_err.map_or(Ok(()), Err)
}
