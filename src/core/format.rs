//! Human readable sizes (`FormatUtil` in Stacer).

const KIBI: u64 = 1 << 10;
const MEBI: u64 = 1 << 20;
const GIBI: u64 = 1 << 30;
const TEBI: u64 = 1 << 40;

pub fn bytes(b: u64) -> String {
    let unit = |u: u64, t: &str| format!("{:.1} {t}", b as f64 / u as f64);
    match b {
        1 => "1 byte".into(),
        _ if b < KIBI => format!("{b} bytes"),
        _ if b < MEBI => unit(KIBI, "KiB"),
        _ if b < GIBI => unit(MEBI, "MiB"),
        _ if b < TEBI => unit(GIBI, "GiB"),
        _ => unit(TEBI, "TiB"),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn formats() {
        assert_eq!(super::bytes(0), "0 bytes");
        assert_eq!(super::bytes(1), "1 byte");
        assert_eq!(super::bytes(1536), "1.5 KiB");
        assert_eq!(super::bytes(3 << 30), "3.0 GiB");
    }
}
