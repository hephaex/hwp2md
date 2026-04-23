use crate::hwp::record::*;

/// Parse the URL from a `hyln` (CTRL_HYPERLINK) CTRL_HEADER record.
///
/// Layout (observed): bytes 0-3 = ctrl_id (`hyln`), bytes 4-5 = UTF-16LE
/// char count, followed by that many UTF-16LE code units.  The exact
/// field layout may vary across HWP versions; we apply a plausibility
/// check on the decoded URL and return empty on garbage.
pub(crate) fn parse_hyperlink_url(rec: &Record) -> String {
    if rec.data.len() < 6 {
        return String::new();
    }
    let (url, _) = read_utf16le_str(&rec.data, 4);
    if url.is_empty() || url.contains('\0') {
        return String::new();
    }
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
        data.extend_from_slice(&(url_chars.len() as u16).to_le_bytes());
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
}
