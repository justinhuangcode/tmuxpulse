//! Parsing utilities for tmux format output.
//! Kept as a separate module for testability.

/// Parse a tab-separated line into a fixed-size array of fields.
/// Returns None if the line has fewer fields than expected.
pub fn parse_tsv_line(line: &str, expected_fields: usize) -> Option<Vec<String>> {
    let fields: Vec<String> = line.split('\t').map(|s| s.to_string()).collect();
    if fields.len() >= expected_fields {
        Some(fields)
    } else {
        None
    }
}

/// Parse a tmux timestamp string (seconds since epoch) into i64
pub fn parse_timestamp(s: &str) -> i64 {
    s.parse::<i64>().unwrap_or(0)
}

/// Parse a boolean-like tmux field ("0" = false, anything else = true)
pub fn parse_bool(s: &str) -> bool {
    s != "0"
}

/// Parse a u32 field with a default fallback
pub fn parse_u32(s: &str, default: u32) -> u32 {
    s.parse().unwrap_or(default)
}

/// Parse a u16 field with a default fallback
pub fn parse_u16(s: &str, default: u16) -> u16 {
    s.parse().unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tsv_valid() {
        let fields = parse_tsv_line("$1\tdev\t0\t1700000000\t1700001000", 5).unwrap();
        assert_eq!(fields.len(), 5);
        assert_eq!(fields[0], "$1");
        assert_eq!(fields[1], "dev");
    }

    #[test]
    fn parse_tsv_too_few_fields() {
        assert!(parse_tsv_line("$1\tdev", 5).is_none());
    }

    #[test]
    fn parse_tsv_extra_fields() {
        let fields = parse_tsv_line("a\tb\tc\td\te\tf", 5).unwrap();
        assert_eq!(fields.len(), 6);
    }

    #[test]
    fn parse_bool_values() {
        assert!(!parse_bool("0"));
        assert!(parse_bool("1"));
        assert!(parse_bool("2"));
    }

    #[test]
    fn parse_timestamp_values() {
        assert_eq!(parse_timestamp("1700000000"), 1700000000);
        assert_eq!(parse_timestamp("invalid"), 0);
        assert_eq!(parse_timestamp(""), 0);
    }

    #[test]
    fn parse_u32_values() {
        assert_eq!(parse_u32("42", 0), 42);
        assert_eq!(parse_u32("bad", 99), 99);
    }
}
