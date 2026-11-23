//! Temperatures from hwmon (/sys/class/hwmon/*/tempN_input, millidegrees Celsius) and thermal zones.

use super::fsutil::{list_dir, read_string};
use std::path::Path;

/// hwmon drivers that report the CPU package / die temperature.
const CPU_DRIVERS: [&str; 6] = ["coretemp", "k10temp", "zenpower", "cpu_thermal", "cpu-thermal", "soc_thermal"];
/// Preferred sensor labels, best first (Tdie has no offset, unlike Tctl on some Ryzens).
const CPU_LABELS: [&str; 4] = ["Package id 0", "Tdie", "Tctl", "CPU"];
/// Thermal zone types used when no hwmon driver matches.
const CPU_THERMAL_ZONES: [&str; 5] = ["x86_pkg_temp", "cpu-thermal", "cpu_thermal", "TCPU", "k10temp"];

/// (label, °C) of every tempN_input in one hwmon directory, sorted by N.
pub fn hwmon_temps(hw: &Path) -> Vec<(String, f64)> {
    let mut v: Vec<(u32, String, f64)> = list_dir(hw, true)
        .into_iter()
        .filter_map(|p| {
            let n: u32 = p.file_name()?.to_str()?.strip_prefix("temp")?.strip_suffix("_input")?.parse().ok()?;
            let milli: f64 = read_string(&p).trim().parse().ok()?;
            let label = read_string(hw.join(format!("temp{n}_label"))).trim().to_string();
            Some((n, label, milli / 1000.0))
        })
        .collect();
    v.sort_by_key(|t| t.0);
    v.into_iter().map(|(_, l, t)| (l, t)).filter(|(_, t)| *t > -50.0 && *t < 200.0).collect()
}

/// Best sensor from a list: first preferred label, else the first sensor.
pub fn pick(temps: &[(String, f64)], labels: &[&str]) -> Option<f64> {
    labels
        .iter()
        .find_map(|l| temps.iter().find(|(name, _)| name.eq_ignore_ascii_case(l)))
        .or_else(|| temps.first())
        .map(|(_, t)| *t)
}

/// CPU temperature in °C, if the system exposes one.
pub fn cpu() -> Option<f64> {
    let hwmons = list_dir("/sys/class/hwmon", false);
    for driver in CPU_DRIVERS {
        for hw in &hwmons {
            if read_string(hw.join("name")).trim() == driver {
                if let Some(t) = pick(&hwmon_temps(hw), &CPU_LABELS) {
                    return Some(t);
                }
            }
        }
    }
    let zones = list_dir("/sys/class/thermal", false);
    for ty in CPU_THERMAL_ZONES {
        for z in zones.iter().filter(|z| z.file_name().is_some_and(|n| n.to_string_lossy().starts_with("thermal_zone"))) {
            if read_string(z.join("type")).trim() == ty {
                if let Ok(m) = read_string(z.join("temp")).trim().parse::<f64>() {
                    return Some(m / 1000.0);
                }
            }
        }
    }
    None
}

/// Temperature zones: below `warm` cold, below `hot` hot, from `hot` on very hot (risky).
#[derive(Clone, Copy)]
pub struct Zones {
    pub warm: f64,
    pub hot: f64,
}

impl Zones {
    /// 0 cold, 1 hot, 2 very hot
    pub fn level(&self, t: f64) -> i32 {
        if t >= self.hot { 2 } else if t >= self.warm { 1 } else { 0 }
    }
}

/// CPUs throttle around 95-105 °C.
pub const CPU_ZONES: Zones = Zones { warm: 60.0, hot: 85.0 };
/// GPUs throttle around 83-90 °C.
pub const GPU_ZONES: Zones = Zones { warm: 60.0, hot: 80.0 };
/// Range shown by the temperature bars.
pub const SCALE: (f64, f64) = (20.0, 105.0);

/// Position of a temperature on the bar scale, 0-100.
pub fn scale_percent(t: f64) -> f32 {
    ((t - SCALE.0) / (SCALE.1 - SCALE.0) * 100.0).clamp(0.0, 100.0) as f32
}

/// "65 °C"
pub fn format(t: f64) -> String {
    format!("{t:.0} °C")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn picks_label() {
        let t = |l: &str, v| (l.to_string(), v);
        assert_eq!(pick(&[t("Tctl", 70.0), t("Tdie", 60.0)], &CPU_LABELS), Some(60.0));
        assert_eq!(pick(&[t("Core 0", 50.0), t("Package id 0", 55.0)], &CPU_LABELS), Some(55.0));
        assert_eq!(pick(&[t("", 42.0)], &CPU_LABELS), Some(42.0));
        assert_eq!(pick(&[], &CPU_LABELS), None);
        assert_eq!(format(64.6), "65 °C");
        assert_eq!(CPU_ZONES.level(59.9), 0);
        assert_eq!(CPU_ZONES.level(60.0), 1);
        assert_eq!(CPU_ZONES.level(92.0), 2);
        assert_eq!(scale_percent(10.0), 0.0);
        assert_eq!(scale_percent(62.5), 50.0);
        assert_eq!(scale_percent(150.0), 100.0);
    }
}
