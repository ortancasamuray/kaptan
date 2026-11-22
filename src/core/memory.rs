//! RAM and swap (`MemoryInfo` in Stacer).

use super::fsutil::read_lines;

#[derive(Default, Clone, Copy)]
pub struct MemoryInfo {
    pub mem_total: u64,
    pub mem_used: u64,
    pub swap_total: u64,
    pub swap_used: u64,
}

pub fn read() -> MemoryInfo {
    let mut v = std::collections::HashMap::new();
    for line in read_lines("/proc/meminfo") {
        let mut it = line.split_whitespace();
        if let (Some(k), Some(n)) = (it.next(), it.next()) {
            v.insert(k.trim_end_matches(':').to_string(), n.parse::<u64>().unwrap_or(0) << 10);
        }
    }
    let g = |k: &str| v.get(k).copied().unwrap_or(0);
    // https://access.redhat.com/solutions/406773
    let cached = (g("Cached") + g("SReclaimable")).saturating_sub(g("Shmem"));
    MemoryInfo {
        mem_total: g("MemTotal"),
        mem_used: g("MemTotal").saturating_sub(g("MemFree") + g("Buffers") + cached),
        swap_total: g("SwapTotal"),
        swap_used: g("SwapTotal").saturating_sub(g("SwapFree")),
    }
}
