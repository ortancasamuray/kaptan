//! APT repositories: one-line `.list` and deb822 `.sources` files (`AptSourceTool` in Stacer).

use super::command::{is_apt_rpm, is_executable, sudo_exec};
use super::fsutil::read_string;
use std::path::{Path, PathBuf};

pub const SOURCES_LIST: &str = "/etc/apt/sources.list";
pub const SOURCES_DIR: &str = "/etc/apt/sources.list.d";

#[derive(Clone, Debug, PartialEq)]
pub struct AptSource {
    pub file: PathBuf,
    /// Line index (`.list`) or entry index (`.sources`) inside `file`.
    pub index: usize,
    pub types: String,
    pub uri: String,
    pub suites: String,
    pub components: String,
    pub options: String,
    pub active: bool,
}

impl AptSource {
    pub fn is_deb822(&self) -> bool {
        self.file.extension().is_some_and(|e| e == "sources")
    }

    pub fn display(&self) -> String {
        let opts = if self.options.is_empty() { String::new() } else { format!(" {}", self.options) };
        format!("{}{} {} {} {}", self.types, opts, self.uri, self.suites, self.components).trim().to_string()
    }
}

fn binary_type() -> &'static str {
    if is_apt_rpm() { "rpm" } else { "deb" }
}

pub fn supported() -> bool {
    Path::new(SOURCES_DIR).is_dir()
}

pub fn list() -> Vec<AptSource> {
    let mut files: Vec<PathBuf> = vec![SOURCES_LIST.into()];
    let mut dir = super::fsutil::list_dir(SOURCES_DIR, true);
    dir.retain(|p| p.extension().is_some_and(|e| e == "list" || e == "sources"));
    files.extend(dir);

    let mut out = Vec::new();
    for file in files {
        let content = read_string(&file);
        if file.extension().is_some_and(|e| e == "sources") {
            out.extend(parse_deb822(&file, &content));
        } else {
            out.extend(parse_list(&file, &content));
        }
    }
    out
}

fn parse_list(file: &Path, content: &str) -> Vec<AptSource> {
    let bt = binary_type();
    content
        .lines()
        .enumerate()
        .filter_map(|(i, line)| {
            let t = line.trim();
            let active = !t.starts_with('#');
            let mut t = t.trim_start_matches('#').trim().to_string();
            let mut options = String::new();
            if let (Some(s), Some(e)) = (t.find('['), t.find(']')) {
                if s < e {
                    options = t[s..=e].to_string();
                    t.replace_range(s..=e, "");
                }
            }
            let cols: Vec<&str> = t.split_whitespace().collect();
            if cols.len() < 3 || (cols[0] != bt && cols[0] != format!("{bt}-src")) {
                return None;
            }
            Some(AptSource {
                file: file.into(),
                index: i,
                types: cols[0].into(),
                uri: cols[1].into(),
                suites: cols[2].into(),
                components: cols[3..].join(" "),
                options,
                active,
            })
        })
        .collect()
}

/// Split a deb822 file into entries: (start_line, end_line_exclusive).
fn deb822_entries(lines: &[&str]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, l) in lines.iter().enumerate() {
        if l.trim().is_empty() {
            if let Some(s) = start.take() {
                out.push((s, i));
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(s) = start {
        out.push((s, lines.len()));
    }
    out
}

fn deb822_field(entry: &[&str], key: &str) -> Option<String> {
    entry.iter().filter(|l| !l.trim_start().starts_with('#')).find_map(|l| {
        let (k, v) = l.split_once(':')?;
        (k.trim().eq_ignore_ascii_case(key) && !l.starts_with(' ')).then(|| v.trim().to_string())
    })
}

fn parse_deb822(file: &Path, content: &str) -> Vec<AptSource> {
    let lines: Vec<&str> = content.lines().collect();
    deb822_entries(&lines)
        .into_iter()
        .enumerate()
        .filter_map(|(idx, (s, e))| {
            let entry = &lines[s..e];
            let types = deb822_field(entry, "Types")?;
            if !types.split_whitespace().any(|t| t == binary_type() || t == format!("{}-src", binary_type())) {
                return None;
            }
            Some(AptSource {
                file: file.into(),
                index: idx,
                types,
                uri: deb822_field(entry, "URIs").unwrap_or_default(),
                suites: deb822_field(entry, "Suites").unwrap_or_default(),
                components: deb822_field(entry, "Components").unwrap_or_default(),
                options: String::new(),
                active: deb822_field(entry, "Enabled").map(|v| v.to_lowercase() != "no").unwrap_or(true),
            })
        })
        .collect()
}

/// Rewrite `src` in its file: `None` removes it, `Some(new)` replaces it.
pub fn change(src: &AptSource, new: Option<&AptSource>) -> Result<(), String> {
    let content = read_string(&src.file);
    let lines: Vec<&str> = content.lines().collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());

    if src.is_deb822() {
        let entries = deb822_entries(&lines);
        let &(s, e) = entries.get(src.index).ok_or("source entry not found")?;
        out.extend(lines[..s].iter().map(|l| l.to_string()));
        if let Some(n) = new {
            out.extend(rewrite_deb822_entry(&lines[s..e], n));
        } else {
            // drop the following blank line too
            let mut e2 = e;
            while e2 < lines.len() && lines[e2].trim().is_empty() {
                e2 += 1;
            }
            out.extend(lines[e2..].iter().map(|l| l.to_string()));
            return write(&src.file, &out);
        }
        out.extend(lines[e..].iter().map(|l| l.to_string()));
    } else {
        for (i, l) in lines.iter().enumerate() {
            if i != src.index {
                out.push(l.to_string());
            } else if let Some(n) = new {
                let mut line = n.display();
                if !n.active {
                    line = format!("# {line}");
                }
                out.push(line);
            }
        }
    }
    write(&src.file, &out)
}

fn rewrite_deb822_entry(entry: &[&str], n: &AptSource) -> Vec<String> {
    let mut wanted: Vec<(&str, String)> = vec![
        ("Types", n.types.clone()),
        ("URIs", n.uri.clone()),
        ("Suites", n.suites.clone()),
        ("Components", n.components.clone()),
    ];
    if !n.active {
        wanted.push(("Enabled", "no".into()));
    }
    let mut out = Vec::new();
    for l in entry {
        let key = (!l.starts_with([' ', '\t']) && !l.trim_start().starts_with('#'))
            .then(|| l.split_once(':').map(|(k, _)| k.trim()))
            .flatten();
        match key {
            Some(k) if k.eq_ignore_ascii_case("Enabled") && n.active => continue,
            Some(k) => {
                if let Some(pos) = wanted.iter().position(|(w, _)| w.eq_ignore_ascii_case(k)) {
                    let (w, v) = wanted.remove(pos);
                    out.push(format!("{w}: {v}"));
                } else {
                    out.push(l.to_string());
                }
            }
            None => out.push(l.to_string()), // comments, continuation lines (Signed-By keys)
        }
    }
    out.extend(wanted.into_iter().map(|(k, v)| format!("{k}: {v}")));
    out
}

fn write(file: &Path, lines: &[String]) -> Result<(), String> {
    let path = file.to_string_lossy();
    let mut content = lines.join("\n");
    while content.ends_with("\n\n") {
        content.pop();
    }
    if content.trim().is_empty() && file != Path::new(SOURCES_LIST) {
        return sudo_exec("rm", &["--", &path], None).map(|_| ());
    }
    content.push('\n');
    sudo_exec("tee", &[&path], Some(content.as_bytes())).map(|_| ())
}

pub fn set_active(src: &AptSource, active: bool) -> Result<(), String> {
    if src.is_deb822() {
        let mut n = src.clone();
        n.active = active;
        return change(src, Some(&n));
    }
    // One-line format: only add/remove the comment mark so options and spacing stay as written.
    let content = read_string(&src.file);
    let mut lines: Vec<String> = content.lines().map(str::to_string).collect();
    let line = lines.get_mut(src.index).ok_or("source entry not found")?;
    *line = toggle_list_line(line, active);
    write(&src.file, &lines)
}

fn toggle_list_line(line: &str, active: bool) -> String {
    let t = line.trim_start();
    let is_active = !t.starts_with('#');
    match (is_active, active) {
        (true, false) => format!("# {line}"),
        (false, true) => t.trim_start_matches('#').trim_start().to_string(),
        _ => line.to_string(),
    }
}

pub fn remove(src: &AptSource) -> Result<(), String> {
    change(src, None)
}

/// `add-apt-repository` (Debian/Ubuntu) or `apt-repo` (ALT Linux).
pub fn add_repository(repo: &str, with_source: bool) -> Result<(), String> {
    let repo = repo.trim();
    if repo.is_empty() {
        return Err("Repository is empty".into());
    }
    if is_apt_rpm() && is_executable("apt-repo") {
        sudo_exec("apt-repo", &["add", repo], None).map(|_| ())
    } else if is_executable("add-apt-repository") {
        let mut args = vec!["-y", repo];
        if with_source {
            args.push("-s");
        }
        sudo_exec("add-apt-repository", &args, None).map(|_| ())
    } else {
        Err("add-apt-repository is not installed (package: software-properties-common)".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_format() {
        let s = parse_list(Path::new("/x.list"), "# comment\ndeb [arch=amd64] http://a b main contrib\n# deb-src http://a b main\n");
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].options, "[arch=amd64]");
        assert_eq!(s[0].components, "main contrib");
        assert!(!s[1].active);
        assert_eq!(s[1].index, 2);
    }

    #[test]
    fn list_toggle_keeps_line() {
        let l = "deb [arch=amd64  signed-by=/k.gpg] http://a b main";
        let off = toggle_list_line(l, false);
        assert_eq!(off, format!("# {l}"));
        assert_eq!(toggle_list_line(&off, true), l);
        assert_eq!(toggle_list_line("#deb-src http://a b main", true), "deb-src http://a b main");
        assert_eq!(toggle_list_line(l, true), l);
    }

    #[test]
    fn deb822_format() {
        let c = "Types: deb\nURIs: http://a\nSuites: trixie\nComponents: main\nSigned-By: /k.gpg\n\n# x\nTypes: deb-src\nURIs: http://b\nSuites: s\nComponents: main\nEnabled: no\n";
        let s = parse_deb822(Path::new("/x.sources"), c);
        assert_eq!(s.len(), 2);
        assert!(s[0].active && !s[1].active);
        let lines: Vec<&str> = c.lines().collect();
        let mut n = s[0].clone();
        n.active = false;
        let r = rewrite_deb822_entry(&lines[0..5], &n);
        assert!(r.contains(&"Enabled: no".to_string()));
        assert!(r.contains(&"Signed-By: /k.gpg".to_string()));
    }
}
