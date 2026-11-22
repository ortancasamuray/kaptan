//! Translations. The same gettext catalog (`lang/<code>/LC_MESSAGES/kaptan.po`) is bundled into the
//! Slint UI by `build.rs` (for `@tr(...)`) and parsed here for strings produced in Rust.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

/// (code, native name) of bundled languages, besides English.
pub const LANGUAGES: &[(&str, &str, &str)] = &[("tr", "Türkçe", include_str!("../lang/tr/LC_MESSAGES/kaptan.po"))];

static CURRENT: RwLock<Option<&'static HashMap<String, String>>> = RwLock::new(None);

fn catalog(code: &str) -> Option<&'static HashMap<String, String>> {
    static CACHE: OnceLock<Vec<(&'static str, HashMap<String, String>)>> = OnceLock::new();
    let all = CACHE.get_or_init(|| LANGUAGES.iter().map(|(c, _, po)| (*c, parse_po(po))).collect());
    all.iter().find(|(c, _)| *c == code).map(|(_, m)| m)
}

/// Language code from the environment ("tr_TR.UTF-8" -> "tr").
pub fn system_language() -> String {
    ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|v| std::env::var(v).ok())
        .find(|v| !v.is_empty() && v != "C" && v != "POSIX")
        .map(|v| v.split([':', '_', '.', '@']).next().unwrap_or_default().to_lowercase())
        .unwrap_or_else(|| "en".into())
}

/// `setting` is "" (follow the system), "en" or a bundled code. Applies to Slint and Rust strings.
/// Must be called after the first Slint component was created.
pub fn set_language(setting: &str) {
    let code = if setting.is_empty() { system_language() } else { setting.to_string() };
    let cat = catalog(&code);
    if let Ok(mut c) = CURRENT.write() {
        *c = cat;
    }
    let _ = slint::select_bundled_translation(if cat.is_some() { &code } else { "en" });
}

/// Translate a string produced in Rust (falls back to the English text).
pub fn tr(s: &str) -> String {
    CURRENT
        .read()
        .ok()
        .and_then(|c| c.and_then(|m| m.get(s)).cloned())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| s.to_string())
}

/// Translate, then fill `{}` placeholders in order.
pub fn trf(s: &str, args: &[&dyn std::fmt::Display]) -> String {
    let mut out = String::new();
    let mut args = args.iter();
    let t = tr(s);
    let mut parts = t.split("{}");
    if let Some(first) = parts.next() {
        out.push_str(first);
    }
    for p in parts {
        if let Some(a) = args.next() {
            out.push_str(&a.to_string());
        }
        out.push_str(p);
    }
    out
}

/// Minimal .po reader: msgid/msgstr with continuation lines, no plurals or contexts.
fn parse_po(po: &str) -> HashMap<String, String> {
    fn unquote(s: &str) -> String {
        let s = s.trim();
        let s = s.strip_prefix('"').and_then(|s| s.strip_suffix('"')).unwrap_or(s);
        let mut out = String::new();
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some(o) => out.push(o),
                    None => {}
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    let mut map = HashMap::new();
    let (mut id, mut msg) = (String::new(), String::new());
    let mut in_str = false;
    for line in po.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("msgid ") {
            if !id.is_empty() {
                map.insert(std::mem::take(&mut id), std::mem::take(&mut msg));
            }
            id = unquote(rest);
            msg.clear();
            in_str = false;
        } else if let Some(rest) = line.strip_prefix("msgstr ") {
            msg = unquote(rest);
            in_str = true;
        } else if line.starts_with('"') {
            if in_str { msg.push_str(&unquote(line)) } else { id.push_str(&unquote(line)) }
        }
    }
    if !id.is_empty() {
        map.insert(id, msg);
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn po_parsing() {
        let m = parse_po("msgid \"\"\nmsgstr \"Content-Type: x\\n\"\n\nmsgid \"Hello {}\"\nmsgstr \"\"\n\"Merhaba {}\"\n\nmsgid \"a \\\"q\\\"\"\nmsgstr \"b\"\n");
        assert_eq!(m.get("Hello {}").unwrap(), "Merhaba {}");
        assert_eq!(m.get("a \"q\"").unwrap(), "b");
        assert!(!m.contains_key(""));
    }

    #[test]
    fn bundled_catalog_has_no_empty_entries() {
        for (code, _, po) in LANGUAGES {
            let m = parse_po(po);
            let missing: Vec<_> = m.iter().filter(|(_, v)| v.is_empty()).map(|(k, _)| k).collect();
            assert!(missing.is_empty(), "{code}: untranslated {missing:?}");
            // placeholders must match
            for (k, v) in &m {
                assert_eq!(k.matches("{}").count(), v.matches("{}").count(), "{code}: placeholder mismatch in {k:?}");
            }
        }
    }
}
