//! Physical disks and their partitions from `lsblk` (mounted or not).

use super::command::exec;
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct Partition {
    pub name: String,
    pub size: u64,
    pub fstype: String,
    pub label: String,
    /// Mount points; "[SWAP]" for active swap
    pub mounts: Vec<String>,
    /// Used / total bytes of the mounted file system. "Used" counts the space reserved for root
    /// as used (size - available), like statvfs and the dashboard gauge.
    pub used: Option<u64>,
    pub fssize: Option<u64>,
    /// Nesting level (partition 0, LVM/LUKS inside a partition 1, ...)
    pub depth: usize,
}

#[derive(Clone, Debug)]
pub struct BlockDisk {
    pub name: String,
    pub model: String,
    pub size: u64,
    /// "NVMe SSD", "SSD", "HDD", "USB"
    pub kind: String,
    pub parts: Vec<Partition>,
}

const COLUMNS: &str = "NAME,MODEL,SIZE,TYPE,FSTYPE,LABEL,MOUNTPOINTS,FSUSED,FSSIZE,FSAVAIL,TRAN,ROTA,RM";

pub fn list() -> Vec<BlockDisk> {
    let out = exec("lsblk", &["-J", "-b", "-o", COLUMNS]).unwrap_or_default();
    parse(&out)
}

fn num(v: &Value) -> Option<u64> {
    v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

fn text(v: &Value) -> String {
    v.as_str().unwrap_or_default().trim().to_string()
}

fn flag(v: &Value) -> bool {
    v.as_bool().unwrap_or_else(|| matches!(v.as_str(), Some("1") | Some("true")))
}

fn parse(json: &str) -> Vec<BlockDisk> {
    let Ok(root) = serde_json::from_str::<Value>(json) else { return vec![] };
    let Some(devs) = root["blockdevices"].as_array() else { return vec![] };

    devs.iter()
        .filter(|d| d["type"] == "disk") // skips loop (snaps), rom, zram is type disk but has no model
        .filter(|d| !text(&d["name"]).starts_with("zram"))
        .map(|d| {
            let tran = text(&d["tran"]);
            let kind = if flag(&d["rm"]) || tran == "usb" {
                "USB"
            } else if tran == "nvme" {
                "NVMe SSD"
            } else if flag(&d["rota"]) {
                "HDD"
            } else {
                "SSD"
            };
            let mut parts = Vec::new();
            collect(d, 0, &mut parts);
            BlockDisk {
                name: text(&d["name"]),
                model: text(&d["model"]),
                size: num(&d["size"]).unwrap_or(0),
                kind: kind.into(),
                parts,
            }
        })
        .collect()
}

fn collect(dev: &Value, depth: usize, out: &mut Vec<Partition>) {
    for c in dev["children"].as_array().into_iter().flatten() {
        out.push(Partition {
            name: text(&c["name"]),
            size: num(&c["size"]).unwrap_or(0),
            fstype: text(&c["fstype"]),
            label: text(&c["label"]),
            mounts: c["mountpoints"].as_array().into_iter().flatten().filter_map(|m| m.as_str()).map(str::to_string).collect(),
            used: match (num(&c["fssize"]), num(&c["fsavail"])) {
                (Some(size), Some(avail)) => Some(size.saturating_sub(avail)),
                _ => num(&c["fsused"]),
            },
            fssize: num(&c["fssize"]),
            depth,
        });
        collect(c, depth + 1, out);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_lsblk() {
        let j = r#"{"blockdevices":[
          {"name":"loop0","model":null,"size":1000,"type":"loop","tran":null,"rota":false,"rm":false,"mountpoints":["/snap/x"]},
          {"name":"nvme1n1","model":"HFS001TEJ9X101N","size":1024209543168,"type":"disk","tran":"nvme","rota":false,"rm":false,"mountpoints":[null],
           "children":[
             {"name":"nvme1n1p2","size":987311669760,"type":"part","fstype":"ext4","label":null,"mountpoints":["/"],"fsused":551752851456,"fssize":970660192256},
             {"name":"nvme1n1p3","size":36581169664,"type":"part","fstype":"swap","label":"swap","mountpoints":["[SWAP]"],"fsused":null,"fssize":null}
           ]},
          {"name":"sdb","model":"Flash","size":"16000000000","type":"disk","tran":"usb","rota":"1","rm":"1","mountpoints":[null],
           "children":[{"name":"sdb1","size":"16000000000","type":"part","fstype":"crypto_LUKS","mountpoints":[null],
             "children":[{"name":"luks-x","size":"15990000000","type":"crypt","fstype":"ext4","mountpoints":["/media/u/x"],"fsused":"5","fssize":"10"}]}]}
        ]}"#;
        let d = super::parse(j);
        assert_eq!(d.len(), 2, "loop devices are skipped");
        assert_eq!((d[0].kind.as_str(), d[0].parts.len()), ("NVMe SSD", 2));
        assert_eq!(d[0].parts[0].mounts, vec!["/"]);
        assert_eq!(d[0].parts[0].used, Some(551752851456));
        assert_eq!(d[1].kind, "USB");
        assert_eq!(d[1].parts.len(), 2);
        assert_eq!((d[1].parts[1].depth, d[1].parts[1].fssize), (1, Some(10)));
        assert!(d[1].parts[0].mounts.is_empty(), "null mount points are dropped");
    }
}
