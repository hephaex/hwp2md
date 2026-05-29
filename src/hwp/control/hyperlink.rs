use crate::hwp::record::{read_utf16le_str, Record};

/// Parse the URL from a `hyln` (`CTRL_HYPERLINK`) `CTRL_HEADER` record.
///
/// Layout (observed): bytes 0-3 = `ctrl_id` (`hyln`), bytes 4-5 = UTF-16LE
/// char count, followed by that many UTF-16LE code units.  The exact
/// field layout may vary across HWP versions; we apply a plausibility
/// check on the decoded URL and return empty on garbage.
pub(crate) fn parse_hyperlink_url(rec: &Record) -> String {
    if rec.data.len() < 6 {
        return String::new();
    }
    let (raw, _) = read_utf16le_str(&rec.data, 4);
    if raw.is_empty() {
        return String::new();
    }
    // Truncate at null terminator if present (some HWP files include a trailing null).
    let before_null = raw.split('\0').next().unwrap_or("");
    // Strip control characters: U+0000–U+001F, DEL (U+007F), and C1 (U+0080–U+009F).
    let url: String = before_null.chars().filter(|c| !c.is_control()).collect();
    // Minimum scheme plausibility: a URL must contain ':' (e.g. "https:", "file:").
    if !url.contains(':') {
        return String::new();
    }
    url
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hwp::record::{CTRL_HYPERLINK, HWPTAG_CTRL_HEADER};

    /// Build a CTRL_HEADER record whose payload is the given raw string encoded as UTF-16LE.
    fn make_url_record(raw: &str) -> Record {
        let chars: Vec<u16> = raw.encode_utf16().collect();
        let mut data = CTRL_HYPERLINK.to_le_bytes().to_vec();
        data.extend_from_slice(&(u16::try_from(chars.len()).unwrap()).to_le_bytes());
        for ch in &chars {
            data.extend_from_slice(&ch.to_le_bytes());
        }
        Record { tag_id: HWPTAG_CTRL_HEADER, level: 0, data }
    }

    #[test]
    fn parse_hyperlink_url_valid() {
        let rec = make_url_record("https://example.com");
        assert_eq!(parse_hyperlink_url(&rec), "https://example.com");
    }

    #[test]
    fn parse_hyperlink_url_record_too_short() {
        // Record with only ctrl_id bytes (< 6 bytes total) — returns empty.
        let data = CTRL_HYPERLINK.to_le_bytes().to_vec();
        let rec = Record { tag_id: HWPTAG_CTRL_HEADER, level: 0, data };
        assert_eq!(parse_hyperlink_url(&rec), "");
    }

    #[test]
    fn parse_hyperlink_url_null_terminated_truncates_at_null() {
        // URL ends with null terminator — common in some HWP files.
        // Must return the part before the null, not reject the whole URL.
        let rec = make_url_record("https://example.com\0garbage");
        assert_eq!(parse_hyperlink_url(&rec), "https://example.com");
    }

    #[test]
    fn parse_hyperlink_url_c0_control_chars_stripped() {
        // C0 control chars (U+0001–U+001F) around the URL are stripped.
        let rec = make_url_record("\x01https://example.com\x02");
        assert_eq!(parse_hyperlink_url(&rec), "https://example.com");
    }

    #[test]
    fn parse_hyperlink_url_del_stripped() {
        // DEL (U+007F) is stripped via is_control(); `>= ' '` alone would not catch it.
        let rec = make_url_record("https://example.com\x7f");
        assert_eq!(parse_hyperlink_url(&rec), "https://example.com");
    }

    #[test]
    fn parse_hyperlink_url_c1_control_chars_stripped() {
        // C1 controls (U+0080–U+009F) are stripped via is_control().
        let rec = make_url_record("https://example.com\u{0085}");
        assert_eq!(parse_hyperlink_url(&rec), "https://example.com");
    }

    #[test]
    fn parse_hyperlink_url_no_scheme_returns_empty() {
        // A string without ':' is not a plausible URL — must return empty.
        let rec = make_url_record("example.com");
        assert_eq!(parse_hyperlink_url(&rec), "");
    }

    #[test]
    fn parse_hyperlink_url_only_null_returns_empty() {
        // URL consists only of null terminator — empty after truncation.
        let rec = make_url_record("\0");
        assert_eq!(parse_hyperlink_url(&rec), "");
    }

    #[test]
    fn parse_hyperlink_url_only_control_chars_returns_empty() {
        // URL consists only of control chars — empty after strip.
        let rec = make_url_record("\x01\x02\x03");
        assert_eq!(parse_hyperlink_url(&rec), "");
    }
}
