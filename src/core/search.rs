//! File search built on `find` (Search page in Stacer).

use super::command::{exec, sudo_exec};
use std::collections::HashMap;
use std::path::PathBuf;

pub const MAX_RESULTS: usize = 2000;

#[derive(Clone, Debug, Default)]
pub struct Query {
    pub directory: String,
    pub pattern: String,
    pub case_insensitive: bool,
    pub regex: bool,
    pub invert: bool,
    pub empty: bool,
    /// "", "f", "d", "l"
    pub file_type: String,
    /// "", "-amin", "-mmin", "-cmin"
    pub time_type: String,
    /// "-", "", "+"
    pub time_cmp: String,
    pub time_minutes: i32,
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
    /// "" disables size filter; "-", "=", "+"
    pub size_cmp: String,
    pub size: i32,
    /// "c", "k", "M", "G"
    pub size_unit: String,
    pub user: String,
    pub group: String,
    pub as_root: bool,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Found {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub user: String,
    pub group: String,
    pub modified: i64,
    pub accessed: i64,
    pub changed: i64,
}

pub fn build_args(q: &Query) -> Vec<String> {
    let mut a: Vec<String> = vec![q.directory.clone(), "-mindepth".into(), "1".into()];
    if !q.pattern.is_empty() {
        if q.invert {
            a.push("!".into());
        }
        a.push(match (q.case_insensitive, q.regex) {
            (true, true) => "-iregex",
            (true, false) => "-iname",
            (false, true) => "-regex",
            (false, false) => "-name",
        }.into());
        a.push(q.pattern.clone());
    }
    if q.empty {
        a.push("-empty".into());
    }
    if !q.file_type.is_empty() {
        a.extend(["-type".into(), q.file_type.clone()]);
    }
    if !q.time_type.is_empty() {
        a.extend([q.time_type.clone(), format!("{}{}", q.time_cmp, q.time_minutes)]);
    }
    for (on, flag) in [(q.readable, "-readable"), (q.writable, "-writable"), (q.executable, "-executable")] {
        if on {
            a.push(flag.into());
        }
    }
    if !q.size_cmp.is_empty() {
        let cmp = if q.size_cmp == "=" { "" } else { &q.size_cmp };
        a.extend(["-size".into(), format!("{cmp}{}{}", q.size, q.size_unit)]);
    }
    if !q.user.is_empty() {
        a.extend(["-user".into(), q.user.clone()]);
    }
    if !q.group.is_empty() {
        a.extend(["-group".into(), q.group.clone()]);
    }
    a
}

/// Blocking. Returns (total found, first MAX_RESULTS entries).
/// Metadata comes from `find -printf` so it is correct for searches run as root, too.
pub fn run(q: &Query) -> Result<(usize, Vec<Found>), String> {
    if q.directory.is_empty() || !std::path::Path::new(&q.directory).is_dir() {
        return Err("Select the search directory.".into());
    }
    let mut args = build_args(q);
    // size uid gid mtime atime ctime path, NUL-terminated (names may contain newlines)
    args.extend(["-printf".into(), "%s %U %G %T@ %A@ %C@ %p\\0".into()]);
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    // find exits non-zero on unreadable dirs; keep the partial output
    let out = if q.as_root { sudo_exec("find", &a, None)? } else { exec("find", &a)? };
    let records: Vec<&str> = out.split('\0').filter(|r| !r.trim().is_empty()).collect();

    let users = id_map("/etc/passwd");
    let groups = id_map("/etc/group");
    let found = records.iter().take(MAX_RESULTS).filter_map(|r| parse_record(r, &users, &groups)).collect();
    Ok((records.len(), found))
}

fn parse_record(r: &str, users: &HashMap<u32, String>, groups: &HashMap<u32, String>) -> Option<Found> {
    let mut f = r.trim_start_matches('\n').splitn(7, ' ');
    let size = f.next()?.parse().unwrap_or(0);
    let uid: u32 = f.next()?.parse().unwrap_or(u32::MAX);
    let gid: u32 = f.next()?.parse().unwrap_or(u32::MAX);
    let time = |s: Option<&str>| s.and_then(|v| v.split('.').next()?.parse::<i64>().ok()).unwrap_or(0);
    let (modified, accessed, changed) = (time(f.next()), time(f.next()), time(f.next()));
    let path = PathBuf::from(f.next()?);
    let name_of = |m: &HashMap<u32, String>, id: u32| m.get(&id).cloned().unwrap_or_else(|| id.to_string());
    Some(Found {
        name: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        size,
        user: name_of(users, uid),
        group: name_of(groups, gid),
        modified,
        accessed,
        changed,
        path,
    })
}

fn id_map(path: &str) -> HashMap<u32, String> {
    super::fsutil::read_lines(path)
        .into_iter()
        .filter_map(|l| {
            let f: Vec<&str> = l.split(':').collect();
            Some((f.get(2)?.parse().ok()?, f[0].to_string()))
        })
        .collect()
}

/// Local time "dd.MM.yyyy hh:mm:ss".
pub fn format_time(ts: i64) -> String {
    if ts == 0 {
        return String::new();
    }
    let t = ts as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    unsafe { libc::localtime_r(&t, &mut tm) };
    format!("{:02}.{:02}.{} {:02}:{:02}:{:02}", tm.tm_mday, tm.tm_mon + 1, tm.tm_year + 1900, tm.tm_hour, tm.tm_min, tm.tm_sec)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn args() {
        let q = Query { directory: "/tmp".into(), pattern: "*.log".into(), case_insensitive: true, invert: true, file_type: "f".into(), size_cmp: "+".into(), size: 5, size_unit: "M".into(), ..Default::default() };
        assert_eq!(build_args(&q).join(" "), "/tmp -mindepth 1 ! -iname *.log -type f -size +5M");
    }

    #[test]
    fn printf_record() {
        let users = HashMap::from([(0, "root".to_string())]);
        let f = parse_record("42 0 0 1700000000.5 1700000001.0 1700000002.0 /root/a b.txt", &users, &HashMap::new()).unwrap();
        assert_eq!((f.size, f.user.as_str(), f.group.as_str(), f.modified), (42, "root", "0", 1700000000));
        assert_eq!(f.name, "a b.txt");
    }

    #[test]
    fn finds_with_metadata() {
        let dir = std::env::temp_dir().join(format!("kaptan-search-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("hello world.log"), b"12345").unwrap();
        let q = Query { directory: dir.to_string_lossy().into(), pattern: "*.log".into(), ..Default::default() };
        let (n, found) = run(&q).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(n, 1);
        assert_eq!((found[0].name.as_str(), found[0].size), ("hello world.log", 5));
        assert_eq!(found[0].user, super::super::system::username());
    }
}
