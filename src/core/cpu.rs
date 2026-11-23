//! CPU usage, clocks and load averages (`CpuInfo` in Stacer).

use super::fsutil::{read_lines, read_string};
use std::collections::HashSet;
use std::time::{Duration, Instant};

/// Shorter sampling windows hold only a few clock ticks (10 ms each) per core: too noisy.
const MIN_INTERVAL: Duration = Duration::from_millis(250);

/// Keeps the previous /proc/stat sample so percentages can be computed from deltas.
#[derive(Default)]
pub struct CpuInfo {
    last: Vec<(f64, f64)>, // (idle, total) per line: [all, cpu0, cpu1, ...]
    sampled: Option<Instant>,
    result: Vec<i32>,
}

impl CpuInfo {
    /// Index 0 is the overall usage, then one entry per logical core.
    /// Called sooner than `MIN_INTERVAL` after the previous sample, it returns that result again.
    pub fn percents(&mut self) -> Vec<i32> {
        if self.sampled.is_some_and(|t| t.elapsed() < MIN_INTERVAL) {
            return self.result.clone();
        }
        self.sampled = Some(Instant::now());
        let mut out = Vec::new();
        for (i, line) in read_lines("/proc/stat").iter().filter(|l| l.starts_with("cpu")).enumerate() {
            let times: Vec<f64> = line.split_whitespace().skip(1).filter_map(|t| t.parse().ok()).collect();
            if times.len() < 5 {
                continue;
            }
            // user nice system idle iowait irq softirq steal [guest guest_nice]: guest time is
            // already counted in user/nice, so only the first 8 fields make up the total.
            let idle = times[3] + times[4];
            let total: f64 = times.iter().take(8).sum();
            if self.last.len() <= i {
                self.last.push((0.0, 0.0));
            }
            let (l_idle, l_total) = self.last[i];
            let d_total = total - l_total;
            let d_idle = idle - l_idle;
            let p = if d_total > 0.0 { 100.0 * (d_total - d_idle) / d_total } else { 0.0 };
            self.last[i] = (idle, total);
            out.push(p.clamp(0.0, 100.0).round() as i32);
        }
        self.result = out.clone();
        out
    }
}

pub fn core_count() -> usize {
    read_lines("/proc/cpuinfo").iter().filter(|l| l.starts_with("processor")).count().max(1)
}

pub fn physical_core_count() -> usize {
    let mut set = HashSet::new();
    let mut physical = 0;
    for line in read_lines("/proc/cpuinfo") {
        let val = || line.split(':').nth(1).and_then(|v| v.trim().parse::<i32>().ok()).unwrap_or(0);
        if line.starts_with("physical id") {
            physical = val();
        } else if line.starts_with("core id") {
            set.insert((physical, val()));
        }
    }
    if set.is_empty() { core_count() } else { set.len() }
}

/// Average current clock in MHz.
pub fn avg_clock_mhz() -> f64 {
    let clocks: Vec<f64> = read_lines("/proc/cpuinfo")
        .iter()
        .filter(|l| l.starts_with("cpu MHz"))
        .filter_map(|l| l.split(':').nth(1)?.trim().parse().ok())
        .collect();
    if !clocks.is_empty() {
        return clocks.iter().sum::<f64>() / clocks.len() as f64;
    }
    // Some ARM kernels don't report MHz in cpuinfo
    let khz: Vec<f64> = (0..core_count())
        .filter_map(|i| read_string(format!("/sys/devices/system/cpu/cpu{i}/cpufreq/scaling_cur_freq")).trim().parse().ok())
        .collect();
    if khz.is_empty() { 0.0 } else { khz.iter().sum::<f64>() / khz.len() as f64 / 1000.0 }
}

pub fn load_avgs() -> [f64; 3] {
    let s = read_string("/proc/loadavg");
    let mut it = s.split_whitespace().map(|v| v.parse().unwrap_or(0.0));
    [it.next().unwrap_or(0.0), it.next().unwrap_or(0.0), it.next().unwrap_or(0.0)]
}
