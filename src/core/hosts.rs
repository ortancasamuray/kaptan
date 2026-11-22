//! /etc/hosts editor (Helpers page in Stacer).

use super::command::sudo_exec;
use super::fsutil::read_string;

pub const HOSTS_PATH: &str = "/etc/hosts";

#[derive(Clone, Debug, PartialEq)]
pub struct HostItem {
    pub line: usize,
    pub ip: String,
    pub hostname: String,
    pub aliases: String,
}

/// Raw file lines; edits are made on these and written back.
pub fn load() -> Vec<String> {
    read_string(HOSTS_PATH).lines().map(str::to_string).collect()
}

pub fn items(lines: &[String]) -> Vec<HostItem> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(i, l)| {
            let l = l.split('#').next().unwrap_or_default().trim();
            let f: Vec<&str> = l.split_whitespace().collect();
            (f.len() > 1).then(|| HostItem { line: i, ip: f[0].into(), hostname: f[1].into(), aliases: f[2..].join(" ") })
        })
        .collect()
}

pub fn format_line(ip: &str, hostname: &str, aliases: &str) -> String {
    format!("{}\t{} {}", ip.trim(), hostname.trim(), aliases.trim()).trim_end().to_string()
}

pub fn save(lines: &[String]) -> Result<(), String> {
    let content = lines.join("\n") + "\n";
    sudo_exec("tee", &[HOSTS_PATH], Some(content.as_bytes())).map(|_| ())
}

#[cfg(test)]
mod tests {
    #[test]
    fn parse() {
        let l: Vec<String> = ["# c", "127.0.0.1 localhost", "", "::1 ip6-localhost ip6-loopback # x"].map(String::from).to_vec();
        let it = super::items(&l);
        assert_eq!(it.len(), 2);
        assert_eq!(it[1].line, 3);
        assert_eq!(it[1].aliases, "ip6-loopback");
    }
}
