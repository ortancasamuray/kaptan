//! Minimal `.desktop` file helpers.

/// Value of the first `key=value` line (unlocalized key only).
pub fn get(lines: &[String], key: &str) -> String {
    let prefix = format!("{key}=");
    lines
        .iter()
        .map(|l| l.trim())
        .find(|l| l.starts_with(&prefix))
        .map(|l| l[prefix.len()..].trim().to_string())
        .unwrap_or_default()
}

/// Replace the first `key=` line or append one.
pub fn set(lines: &mut Vec<String>, key: &str, value: &str) {
    let prefix = format!("{key}=");
    let line = format!("{key}={value}");
    match lines.iter().position(|l| l.trim().starts_with(&prefix)) {
        Some(i) => lines[i] = line,
        None => lines.push(line),
    }
}

pub fn has(lines: &[String], key: &str) -> bool {
    let prefix = format!("{key}=");
    lines.iter().any(|l| l.trim().starts_with(&prefix))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn get_set() {
        let mut l: Vec<String> = ["[Desktop Entry]", "Name[tr]=Uyg", "Name=App", "Exec=app"].map(String::from).to_vec();
        assert_eq!(get(&l, "Name"), "App");
        set(&mut l, "Hidden", "true");
        set(&mut l, "Exec", "app2");
        assert_eq!(get(&l, "Exec"), "app2");
        assert_eq!(l.last().unwrap(), "Hidden=true");
    }
}
