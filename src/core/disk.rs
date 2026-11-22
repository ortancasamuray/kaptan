//! Mounted volumes and disk I/O (`DiskInfo` in Stacer).

use super::fsutil::{read_lines, read_string};
use std::ffi::CString;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct Disk {
    pub name: String,   // mount point
    pub device: String,
    pub fs_type: String,
    pub size: u64,
    pub used: u64,
    pub free: u64,
}

const PSEUDO_FS: &[&str] = &[
    "proc", "sysfs", "devtmpfs", "tmpfs", "devpts", "securityfs", "cgroup", "cgroup2", "pstore", "bpf",
    "debugfs", "tracefs", "configfs", "fusectl", "mqueue", "hugetlbfs", "autofs", "binfmt_misc",
    "efivarfs", "rpc_pipefs", "nsfs", "ramfs", "fuse.portal", "fuse.gvfsd-fuse", "squashfs", "overlay",
];

/// Real mounted file systems with their usage.
pub fn disks() -> Vec<Disk> {
    let mut out: Vec<Disk> = Vec::new();
    for line in read_lines("/proc/self/mounts") {
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() < 3 || PSEUDO_FS.contains(&f[2]) {
            continue;
        }
        let mount = unescape(f[1]);
        if out.iter().any(|d| d.name == mount) {
            continue;
        }
        let Some((size, free)) = statvfs(&mount) else { continue };
        if size == 0 {
            continue;
        }
        out.push(Disk {
            name: mount,
            device: f[0].to_string(),
            fs_type: f[2].to_string(),
            size,
            used: size.saturating_sub(free),
            free,
        });
    }
    out
}

fn unescape(s: &str) -> String {
    s.replace("\\040", " ").replace("\\011", "\t").replace("\\134", "\\")
}

fn statvfs(path: &str) -> Option<(u64, u64)> {
    let c = CString::new(path).ok()?;
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut st) } != 0 {
        return None;
    }
    let frsize = st.f_frsize as u64;
    Some((st.f_blocks as u64 * frsize, st.f_bavail as u64 * frsize))
}

/// Total bytes (read, written) over all physical block devices.
pub fn io_bytes() -> (u64, u64) {
    let mut read = 0;
    let mut write = 0;
    for dev in super::fsutil::list_dir("/sys/block", false) {
        if !dev.join("device").exists() {
            continue;
        }
        let stat: Vec<u64> = read_string(dev.join("stat")).split_whitespace().filter_map(|v| v.parse().ok()).collect();
        if stat.len() > 6 {
            read += stat[2] * 512;
            write += stat[6] * 512;
        }
    }
    (read, write)
}

/// The disk mounted at `/` or the first one.
pub fn pick<'a>(disks: &'a [Disk], preferred: &str) -> Option<&'a Disk> {
    disks
        .iter()
        .find(|d| !preferred.is_empty() && d.name == preferred)
        .or_else(|| disks.iter().find(|d| Path::new(&d.name) == Path::new("/")))
        .or_else(|| disks.first())
}
