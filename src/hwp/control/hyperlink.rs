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
    // Minimum scheme plausibility (RFC 3986 §3.1): the part before the first ':'
    // must be non-empty and start with a letter (scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )).
    // This rejects bare hostnames ("example.com"), digit-led strings ("123:foo"),
    // and empty-prefix strings (":foo").
    //
    // NOTE: single-letter schemes such as "C:" also satisfy RFC 3986 §3.1 and
    // therefore pass this check.  A Windows drive path ("C:\path\file") is not
    // rejected here — it is rejected downstream by `url_util::is_safe_url_scheme`
    // whose explicit allowlist (http/https/ftp/mailto/…) excludes "c:".
    // Do not tighten this pre-filter beyond the RFC without updating tests; the
    // two-layer design is intentional.
    // NOTE: this is a loose pre-filter only. The real security gate is
    // `url_util::is_safe_url_scheme`, applied at the convert layer (convert.rs ~line 486),
    // which enforces an explicit allowlist and rejects javascript:/data: etc.
    let has_valid_scheme = url.find(':').is_some_and(|i| {
        let prefix = &url[..i];
        !prefix.is_empty()
            && prefix.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
            && prefix
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    });
    if !has_valid_scheme {
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
        // Bare hostname — no ':' at all.
        let rec = make_url_record("example.com");
        assert_eq!(parse_hyperlink_url(&rec), "");
    }

    #[test]
    fn parse_hyperlink_url_digit_led_scheme_rejected() {
        // "123:foo" has a colon but the prefix starts with a digit, not a letter.
        // RFC 3986 §3.1: scheme must start with ALPHA.
        let rec = make_url_record("123:foo");
        assert_eq!(parse_hyperlink_url(&rec), "");
    }

    #[test]
    fn parse_hyperlink_url_empty_scheme_prefix_rejected() {
        // ":foo" — colon with empty prefix (no scheme name at all).
        let rec = make_url_record(":foo");
        assert_eq!(parse_hyperlink_url(&rec), "");
    }

    #[test]
    fn parse_hyperlink_url_scheme_starting_with_plus_rejected() {
        // "+foo:bar" — scheme must start with ALPHA, not '+'.
        let rec = make_url_record("+foo:bar");
        assert_eq!(parse_hyperlink_url(&rec), "");
    }

    #[test]
    fn parse_hyperlink_url_ftp_accepted() {
        // ftp: is a valid RFC 3986 scheme.
        let rec = make_url_record("ftp://example.com/file");
        assert_eq!(parse_hyperlink_url(&rec), "ftp://example.com/file");
    }

    #[test]
    fn parse_hyperlink_url_scheme_with_plus_dash_dot_accepted() {
        // RFC 3986 §3.1 tail chars: ALPHA / DIGIT / "+" / "-" / ".".
        // Pins the matches!(c, '+' | '-' | '.') branch with a positive test.
        // "coap+tcp" uses '+'; "view-source" uses '-'; schemes ending in
        // digits (ws2) use DIGIT.  We test one representative each.
        for url in &["coap+tcp://example.com", "view-source:http://x.com"] {
            let rec = make_url_record(url);
            assert_ne!(
                parse_hyperlink_url(&rec),
                "",
                "expected non-empty for valid scheme in {url}"
            );
        }
    }

    #[test]
    fn parse_hyperlink_url_mailto_accepted() {
        // mailto: is a valid non-hierarchical scheme.
        let rec = make_url_record("mailto:user@example.com");
        assert_eq!(parse_hyperlink_url(&rec), "mailto:user@example.com");
    }

    #[test]
    fn parse_hyperlink_url_tab_cr_lf_stripped() {
        // TAB/CR/LF are is_control() chars; the old `>= ' '` filter let them through,
        // creating injection vectors in the Markdown output. Regression pin.
        let rec = make_url_record("https://ex\t\r\nample.com");
        assert_eq!(
            parse_hyperlink_url(&rec),
            "https://example.com",
            "TAB/CR/LF must be stripped by is_control()"
        );
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
