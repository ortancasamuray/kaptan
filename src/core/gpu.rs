//! GPU usage (AMD via sysfs, NVIDIA via nvidia-smi). Intel iGPUs expose no usage counter.

use super::command::is_executable;
use super::fsutil::{list_dir, read_string};
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
}

impl GpuInfo {
    pub fn detect() -> Self {
        for card in list_dir("/sys/class/drm", false) {
            let name = card.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
            if !name.starts_with("card") || name.contains('-') {
                continue;
            }
            let device = card.join("device");
            match read_string(device.join("vendor")).trim() {
                "0x1002" if device.join("gpu_busy_percent").exists() => {
                    return Self { vendor: Vendor::Amd, device };
                }
                "0x10de" if is_executable("nvidia-smi") => {
                    return Self { vendor: Vendor::Nvidia, device };
                }
                _ => {}
            }
        }
        Self { vendor: Vendor::None, device: PathBuf::new() }
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
        }
    }

    fn read_nvidia(&self) -> GpuStatus {
        // On hybrid laptops the dGPU sleeps when idle; querying it would wake it up.
        if read_string(self.device.join("power/runtime_status")).trim() == "suspended" {
            return GpuStatus { available: true, name: "NVIDIA GPU".into(), ..Default::default() };
        }

        let Ok(mut child) = Command::new("nvidia-smi")
            .args(["--query-gpu=name,utilization.gpu,memory.used,memory.total", "--format=csv,noheader,nounits", "--id=0"])
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
        parse_nvidia(&String::from_utf8_lossy(&out.stdout)).unwrap_or_default()
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
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn nvidia_csv() {
        let s = super::parse_nvidia("NVIDIA GeForce RTX 4080 Laptop GPU, 9, 7359, 12282\n").unwrap();
        assert_eq!(s.usage, 9);
        assert_eq!(s.mem_total, 12282 << 20);
        assert!(super::parse_nvidia("[N/A]").is_none());
    }
}
