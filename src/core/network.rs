//! Network traffic counters (`NetworkInfo` in Stacer).

use super::fsutil::{list_dir, read_string};

/// Total (rx, tx) bytes of all non-loopback interfaces that are up.
pub fn rx_tx_bytes() -> (u64, u64) {
    let mut rx = 0;
    let mut tx = 0;
    for iface in list_dir("/sys/class/net", false) {
        let name = iface.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if name == "lo" || read_string(iface.join("operstate")).trim() == "down" {
            continue;
        }
        // skip virtual bridges/veths so traffic is not counted twice
        if !iface.join("device").exists() && !name.starts_with("wl") && !name.starts_with("ppp") {
            continue;
        }
        rx += read_string(iface.join("statistics/rx_bytes")).trim().parse::<u64>().unwrap_or(0);
        tx += read_string(iface.join("statistics/tx_bytes")).trim().parse::<u64>().unwrap_or(0);
    }
    (rx, tx)
}
