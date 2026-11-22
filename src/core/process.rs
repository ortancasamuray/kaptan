//! Process list via `ps` (`ProcessInfo` in Stacer) and process control.

use super::command::exec;

#[derive(Clone, Debug, Default)]
#[allow(dead_code)] // kept for parity with Stacer's columns
pub struct Process {
    pub pid: i32,
    pub rss: u64,
    pub pmem: f64,
    pub vsize: u64,
    pub user: String,
    pub pcpu: f64,
    pub start_time: String,
    pub state: String,
    pub group: String,
    pub nice: i32,
    pub cpu_time: String,
    pub session: String,
    pub cmd: String,
}

const COLUMNS: &str = "pid,rss,pmem,vsize,uname:50,pcpu,start_time,state,group,nice,cputime,session,cmd";

pub fn list() -> Vec<Process> {
    let out = exec("ps", &["ax", "-weo", COLUMNS, "--no-headings"]).unwrap_or_default();
    out.lines().filter_map(parse_line).collect()
}

fn parse_line(line: &str) -> Option<Process> {
    let f: Vec<&str> = line.split_whitespace().collect();
    if f.len() < 13 {
        return None;
    }
    Some(Process {
        pid: f[0].parse().ok()?,
        rss: f[1].parse::<u64>().unwrap_or(0) << 10,
        pmem: f[2].parse().unwrap_or(0.0),
        vsize: f[3].parse::<u64>().unwrap_or(0) << 10,
        user: f[4].to_string(),
        pcpu: f[5].parse().unwrap_or(0.0),
        start_time: f[6].to_string(),
        state: f[7].to_string(),
        group: f[8].to_string(),
        nice: f[9].parse().unwrap_or(0),
        cpu_time: f[10].to_string(),
        session: f[11].to_string(),
        cmd: f[12..].join(" "),
    })
}

/// Send SIGTERM; if not permitted, retry as root via pkexec.
pub fn terminate(pid: i32) -> Result<(), String> {
    if unsafe { libc::kill(pid, libc::SIGTERM) } == 0 {
        return Ok(());
    }
    super::command::sudo_exec("kill", &["-TERM", &pid.to_string()], None).map(|_| ())
}
