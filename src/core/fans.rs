//! Fan speeds from hwmon (/sys/class/hwmon/*/fanN_input, in RPM).

use super::fsutil::{list_dir, read_string};

#[derive(Clone, Debug, PartialEq)]
pub struct Fan {
    /// Driver label ("cpu_fan", "gpu_fan", ...) or "" when the driver gives none
    pub label: String,
    /// 1-based fan number within its hwmon device
    pub index: u32,
    pub rpm: u32,
    /// Maximum speed when the driver reports it
    pub max: Option<u32>,
}

pub fn read() -> Vec<Fan> {
    let mut fans = Vec::new();
    for hw in list_dir("/sys/class/hwmon", false) {
        let mut inputs: Vec<(u32, std::path::PathBuf)> = list_dir(&hw, true)
            .into_iter()
            .filter_map(|p| {
                let name = p.file_name()?.to_str()?.to_string();
                let n = name.strip_prefix("fan")?.strip_suffix("_input")?.parse().ok()?;
                Some((n, p))
            })
            .collect();
        inputs.sort();
        for (n, input) in inputs {
            let Ok(rpm) = read_string(&input).trim().parse::<u32>() else { continue };
            let attr = |a: &str| read_string(hw.join(format!("fan{n}_{a}"))).trim().to_string();
            fans.push(Fan {
                label: attr("label"),
                index: n,
                rpm,
                max: attr("max").parse().ok().filter(|m| *m > 0),
            });
        }
    }
    fans
}

/// English display name for a driver label; translated by the caller.
pub fn display_name(f: &Fan) -> String {
    let l = f.label.to_lowercase();
    let known = [
        ("cpu", "CPU fan"),
        ("processor", "CPU fan"),
        ("gpu", "GPU fan"),
        ("video", "GPU fan"),
        ("mid", "Middle fan"),
        ("sys", "System fan"),
        ("chassis", "Chassis fan"),
        ("case", "Chassis fan"),
        ("pump", "Pump"),
    ];
    known
        .iter()
        .find(|(k, _)| l.contains(k))
        .map(|(_, v)| v.to_string())
        .unwrap_or_else(|| if f.label.is_empty() { format!("Fan {}", f.index) } else { f.label.clone() })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names() {
        let f = |label: &str, index| Fan { label: label.into(), index, rpm: 0, max: None };
        assert_eq!(display_name(&f("cpu_fan", 1)), "CPU fan");
        assert_eq!(display_name(&f("mid_fan", 3)), "Middle fan");
        assert_eq!(display_name(&f("", 2)), "Fan 2");
        assert_eq!(display_name(&f("Exhaust", 1)), "Exhaust");
    }
}
