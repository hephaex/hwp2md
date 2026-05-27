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
    let truncated = raw.split('\0').next().unwrap_or("");
    // Strip remaining control characters (U+0000–U+001F).
    let url: String = truncated.chars().filter(|&c| c >= ' ').collect();
    url
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hwp::record::{CTRL_HYPERLINK, HWPTAG_CTRL_HEADER};

    #[test]
    fn parse_hyperlink_url_valid() {
        let url_chars: Vec<u16> = "https://example.com".encode_utf16().collect();
        let mut data = CTRL_HYPERLINK.to_le_bytes().to_vec();
        data.extend_from_slice(&(u16::try_from(url_chars.len()).unwrap()).to_le_bytes());
        for ch in &url_chars {
            data.extend_from_slice(&ch.to_le_bytes());
        }
        let rec = Record {
            tag_id: HWPTAG_CTRL_HEADER,
            level: 0,
            data,
        };
        assert_eq!(parse_hyperlink_url(&rec), "https://example.com");
    }

    #[test]
    fn parse_hyperlink_url_truncated_returns_empty() {
        let data = CTRL_HYPERLINK.to_le_bytes().to_vec();
        let rec = Record {
            tag_id: HWPTAG_CTRL_HEADER,
            level: 0,
            data,
        };
        assert_eq!(parse_hyperlink_url(&rec), "");
    }

    #[test]
    fn parse_hyperlink_url_null_terminated_truncates_at_null() {
        // URL ends with null terminator — common in some HWP files.
        // Must return the part before the null, not reject the whole URL.
        let url = "https://example.com";
        let url_with_null = format!("{url}\0garbage");
        let chars: Vec<u16> = url_with_null.encode_utf16().collect();
        let mut data = CTRL_HYPERLINK.to_le_bytes().to_vec();
        data.extend_from_slice(&(u16::try_from(chars.len()).unwrap()).to_le_bytes());
        for ch in &chars {
            data.extend_from_slice(&ch.to_le_bytes());
        }
        let rec = Record {
            tag_id: HWPTAG_CTRL_HEADER,
            level: 0,
            data,
        };
        assert_eq!(parse_hyperlink_url(&rec), "https://example.com");
    }

    #[test]
    fn parse_hyperlink_url_control_chars_stripped() {
        // URL with embedded control chars (e.g., 0x01 before the URL from encoding glitch).
        let url_with_ctrl = "\x01https://example.com\x02";
        let chars: Vec<u16> = url_with_ctrl.encode_utf16().collect();
        let mut data = CTRL_HYPERLINK.to_le_bytes().to_vec();
        data.extend_from_slice(&(u16::try_from(chars.len()).unwrap()).to_le_bytes());
        for ch in &chars {
            data.extend_from_slice(&ch.to_le_bytes());
        }
        let rec = Record {
            tag_id: HWPTAG_CTRL_HEADER,
            level: 0,
            data,
        };
        assert_eq!(parse_hyperlink_url(&rec), "https://example.com");
    }

    #[test]
    fn parse_hyperlink_url_only_null_returns_empty() {
        // URL consists only of null terminator — must return empty.
        let chars: Vec<u16> = "\0".encode_utf16().collect();
        let mut data = CTRL_HYPERLINK.to_le_bytes().to_vec();
        data.extend_from_slice(&(u16::try_from(chars.len()).unwrap()).to_le_bytes());
        for ch in &chars {
            data.extend_from_slice(&ch.to_le_bytes());
        }
        let rec = Record {
            tag_id: HWPTAG_CTRL_HEADER,
            level: 0,
            data,
        };
        assert_eq!(parse_hyperlink_url(&rec), "");
    }

    #[test]
    fn parse_hyperlink_url_only_control_chars_returns_empty() {
        // URL consists only of control chars — must return empty after strip.
        let chars: Vec<u16> = "\x01\x02\x03".encode_utf16().collect();
        let mut data = CTRL_HYPERLINK.to_le_bytes().to_vec();
        data.extend_from_slice(&(u16::try_from(chars.len()).unwrap()).to_le_bytes());
        for ch in &chars {
            data.extend_from_slice(&ch.to_le_bytes());
        }
        let rec = Record {
            tag_id: HWPTAG_CTRL_HEADER,
            level: 0,
            data,
        };
        assert_eq!(parse_hyperlink_url(&rec), "");
    }
}
