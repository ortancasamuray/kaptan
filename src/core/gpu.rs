//! GPU usage (AMD via sysfs, NVIDIA via nvidia-smi). Intel iGPUs expose no usage counter.

use super::command::is_executable;
use super::fsutil::{list_dir, read_string};
use super::temps;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Default)]
pub struct GpuStatus {
    pub available: bool,
    pub name: String,
    pub usage: i32,
    pub mem_used: u64,
    pub mem_total: u64,
    /// °C, when the driver reports it
    pub temp: Option<f64>,
    /// NVIDIA dGPU in runtime suspend (not queried, so it is not woken up)
    pub asleep: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Vendor {
    None,
    Amd,
    Nvidia,
}

pub struct GpuInfo {
    vendor: Vendor,
    device: PathBuf,
    /// hwmon directory with a temperature sensor of a GPU (also when usage is not available)
    temp_hwmon: Option<PathBuf>,
}

impl GpuInfo {
    pub fn detect() -> Self {
        let mut found = Self::detect_usage();
        found.temp_hwmon = if found.vendor == Vendor::None {
            Self::cards().iter().find_map(|d| temp_hwmon(d))
        } else {
            temp_hwmon(&found.device)
        };
        found
    }

    /// `device` directories of the DRM cards.
    fn cards() -> Vec<PathBuf> {
        list_dir("/sys/class/drm", false)
            .into_iter()
            .filter(|c| c.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("card") && !n.contains('-')))
            .map(|c| c.join("device"))
            .collect()
    }

    fn detect_usage() -> Self {
        for card in list_dir("/sys/class/drm", false) {
            let name = card.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
            if !name.starts_with("card") || name.contains('-') {
                continue;
            }
            let device = card.join("device");
            match read_string(device.join("vendor")).trim() {
                "0x1002" if device.join("gpu_busy_percent").exists() => {
                    return Self { vendor: Vendor::Amd, device, temp_hwmon: None };
                }
                "0x10de" if is_executable("nvidia-smi") => {
                    return Self { vendor: Vendor::Nvidia, device, temp_hwmon: None };
                }
                _ => {}
            }
        }
        Self { vendor: Vendor::None, device: PathBuf::new(), temp_hwmon: None }
    }

    /// Temperature from the GPU's hwmon sensor (amdgpu, nouveau, xe, ...), without spawning anything.
    pub fn sysfs_temp(&self) -> Option<f64> {
        let hw = self.temp_hwmon.as_ref()?;
        temps::pick(&temps::hwmon_temps(hw), &["edge", "gpu"])
    }

    pub fn has_gpu(&self) -> bool {
        self.vendor != Vendor::None
    }

    /// Blocking (may spawn nvidia-smi): call from a worker thread.
    pub fn status(&self) -> GpuStatus {
        match self.vendor {
            Vendor::Amd => self.read_amd(),
            Vendor::Nvidia => self.read_nvidia(),
            Vendor::None => GpuStatus::default(),
        }
    }

    fn read_amd(&self) -> GpuStatus {
        let Ok(usage) = read_string(self.device.join("gpu_busy_percent")).trim().parse() else {
            return GpuStatus::default();
        };
        let n = |f: &str| read_string(self.device.join(f)).trim().parse().unwrap_or(0);
        GpuStatus {
            available: true,
            name: "AMD GPU".into(),
            usage,
            mem_used: n("mem_info_vram_used"),
            mem_total: n("mem_info_vram_total"),
            temp: self.sysfs_temp(),
            asleep: false,
        }
    }

    fn read_nvidia(&self) -> GpuStatus {
        // On hybrid laptops the dGPU sleeps when idle; querying it would wake it up.
        if read_string(self.device.join("power/runtime_status")).trim() == "suspended" {
            return GpuStatus { available: true, name: "NVIDIA GPU".into(), asleep: true, ..Default::default() };
        }

        let Ok(mut child) = Command::new("nvidia-smi")
            .args(["--query-gpu=name,utilization.gpu,memory.used,memory.total,temperature.gpu", "--format=csv,noheader,nounits", "--id=0"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        else {
            return GpuStatus::default();
        };

        // 3 s timeout
        let start = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if start.elapsed() < Duration::from_secs(3) => std::thread::sleep(Duration::from_millis(20)),
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return GpuStatus::default();
                }
            }
        }
        let Ok(out) = child.wait_with_output() else { return GpuStatus::default() };
        if !out.status.success() {
            return GpuStatus::default();
        }
        parse_nvidia(&String::from_utf8_lossy(&out.stdout))
            .map(|mut g| {
                g.temp = g.temp.or_else(|| self.sysfs_temp());
                g
            })
            .unwrap_or_default()
    }
}

fn parse_nvidia(s: &str) -> Option<GpuStatus> {
    let f: Vec<&str> = s.trim().split(',').map(str::trim).collect();
    if f.len() < 4 {
        return None;
    }
    Some(GpuStatus {
        available: true,
        name: f[0].to_string(),
        usage: f[1].parse().ok()?,
        mem_used: f[2].parse::<u64>().unwrap_or(0) << 20,
        mem_total: f[3].parse::<u64>().unwrap_or(0) << 20,
        temp: f.get(4).and_then(|t| t.parse().ok()),
        asleep: false,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn nvidia_csv() {
        let s = super::parse_nvidia("NVIDIA GeForce RTX 4080 Laptop GPU, 9, 7359, 12282, 51\n").unwrap();
        assert_eq!(s.usage, 9);
        assert_eq!(s.temp, Some(51.0));
        assert_eq!(super::parse_nvidia("X, 1, 2, 3, [N/A]").unwrap().temp, None);
        assert_eq!(s.mem_total, 12282 << 20);
        assert!(super::parse_nvidia("[N/A]").is_none());
    }
}

/// First hwmon directory under a PCI device that has a temperature input.
fn temp_hwmon(device: &std::path::Path) -> Option<PathBuf> {
    list_dir(device.join("hwmon"), false).into_iter().find(|hw| !temps::hwmon_temps(hw).is_empty())
}
