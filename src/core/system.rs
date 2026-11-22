//! Static system information and user/group lists (`SystemInfo` in Stacer).

use super::command::exec;
use super::fsutil::{read_lines, read_string};

#[derive(Clone, Debug)]
pub struct SystemInfo {
    pub hostname: String,
    pub platform: String,
    pub distribution: String,
    pub kernel: String,
    pub cpu_model: String,
    pub cpu_cores: usize,
    pub cpu_speed: String,
}

pub fn read() -> SystemInfo {
    let lscpu = exec("lscpu", &[]).unwrap_or_default();
    let field = |key: &str| {
        lscpu
            .lines()
            .find(|l| l.starts_with(key))
            .and_then(|l| l.split_once(':'))
            .map(|(_, v)| v.split_whitespace().collect::<Vec<_>>().join(" "))
    };

    let cpu_model = field("Model name")
        .or_else(|| cpuinfo_field("model name"))
        .map(|m| m.split('@').next().unwrap_or_default().trim().to_string())
        .unwrap_or_else(|| "Unknown".into());

    let cpu_speed = field("CPU max MHz")
        .or_else(|| field("CPU MHz"))
        .or_else(|| cpuinfo_field("cpu MHz"))
        .and_then(|v| v.replace(',', ".").parse::<f64>().ok())
        .map(|mhz| format!("{:.2} GHz", mhz / 1000.0))
        .unwrap_or_else(|| "Unknown".into());

    let uname = uname();
    SystemInfo {
        hostname: read_string("/proc/sys/kernel/hostname").trim().to_string(),
        platform: format!("{} {}", uname.0, uname.2),
        distribution: distribution(),
        kernel: uname.1,
        cpu_model,
        cpu_cores: super::cpu::physical_core_count(),
        cpu_speed,
    }
}

fn cpuinfo_field(key: &str) -> Option<String> {
    read_lines("/proc/cpuinfo")
        .into_iter()
        .find(|l| l.starts_with(key))
        .and_then(|l| l.split_once(':').map(|(_, v)| v.trim().to_string()))
}

/// (sysname, release, machine)
fn uname() -> (String, String, String) {
    let mut u: libc::utsname = unsafe { std::mem::zeroed() };
    if unsafe { libc::uname(&mut u) } != 0 {
        return ("Linux".into(), String::new(), String::new());
    }
    let s = |a: &[libc::c_char]| unsafe { std::ffi::CStr::from_ptr(a.as_ptr()) }.to_string_lossy().into_owned();
    (s(&u.sysname), s(&u.release), s(&u.machine))
}

pub fn distribution() -> String {
    for path in ["/etc/os-release", "/usr/lib/os-release"] {
        for line in read_lines(path) {
            if let Some(v) = line.strip_prefix("PRETTY_NAME=") {
                return v.trim_matches('"').to_string();
            }
        }
    }
    "Linux".into()
}

pub fn username() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| exec("whoami", &[]).unwrap_or_default())
}

fn first_column(path: &str) -> Vec<String> {
    read_lines(path)
        .into_iter()
        .filter_map(|l| l.split(':').next().map(str::to_string))
        .filter(|s| !s.is_empty() && !s.starts_with('#'))
        .collect()
}

pub fn users() -> Vec<String> {
    first_column("/etc/passwd")
}

pub fn groups() -> Vec<String> {
    first_column("/etc/group")
}

pub fn is_flatpak() -> bool {
    std::path::Path::new("/.flatpak-info").exists()
}
