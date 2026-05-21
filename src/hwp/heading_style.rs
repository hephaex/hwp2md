/// Parse a heading level (1–6) from an HWP/HWPX style name.
///
/// Accepts (case-insensitive):
/// - "Outline N", "outline N", "Outline1", "outline1"
/// - "Heading N", "heading N", "Heading1"
/// - "개요 N", "개요N"  (Korean outline)
/// - "제목 N", "제목N"  (Korean heading/title)
///
/// After stripping a matching prefix and an optional space, the remainder
/// must parse as a decimal digit in the range 1–6.
///
/// Returns `Some(N)` on match, `None` otherwise.
pub(crate) fn parse_heading_style(style_name: &str) -> Option<u8> {
    let lower = style_name.to_lowercase();

    // Ordered from most-specific to least-specific so that the first matching
    // prefix wins.  All comparisons are case-insensitive (via `lower`).
    const PREFIXES: &[&str] = &["outline", "heading", "개요", "제목"];

    for prefix in PREFIXES {
        if let Some(rest) = lower.strip_prefix(prefix) {
            // Allow optional whitespace (spaces, tabs, etc.) between the keyword and the digit.
            let rest = rest.trim();
            // Ensure the remainder contains only ASCII digits to reject inputs like "+1".
            if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
                if let Ok(n) = rest.parse::<u8>() {
                    if (1..=6).contains(&n) {
                        return Some(n);
                    }
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_heading_style_outline_english_with_space() {
        assert_eq!(parse_heading_style("Outline 1"), Some(1));
        assert_eq!(parse_heading_style("Outline 6"), Some(6));
    }

    #[test]
    fn parse_heading_style_outline_english_no_space() {
        assert_eq!(parse_heading_style("Outline1"), Some(1));
        assert_eq!(parse_heading_style("outline4"), Some(4));
    }

    #[test]
    fn parse_heading_style_heading_variants() {
        assert_eq!(parse_heading_style("heading 1"), Some(1));
        assert_eq!(parse_heading_style("Heading 3"), Some(3));
        assert_eq!(parse_heading_style("Heading2"), Some(2));
    }

    #[test]
    fn parse_heading_style_korean_outline() {
        assert_eq!(parse_heading_style("개요 2"), Some(2));
        assert_eq!(parse_heading_style("개요5"), Some(5));
    }

    #[test]
    fn parse_heading_style_korean_title() {
        assert_eq!(parse_heading_style("제목 1"), Some(1));
        assert_eq!(parse_heading_style("제목3"), Some(3));
    }

    #[test]
    fn parse_heading_style_out_of_range_returns_none() {
        assert_eq!(parse_heading_style("Outline 0"), None);
        assert_eq!(parse_heading_style("Outline 7"), None);
        assert_eq!(parse_heading_style("heading 9"), None);
    }

    #[test]
    fn parse_heading_style_unrecognised_returns_none() {
        assert_eq!(parse_heading_style("Normal"), None);
        assert_eq!(parse_heading_style("Body Text"), None);
        assert_eq!(parse_heading_style(""), None);
    }

    #[test]
    fn outline_double_space_returns_level() {
        // After trim(), double spaces are handled correctly
        assert_eq!(parse_heading_style("Outline  1"), Some(1));
    }

    #[test]
    fn outline_plus_sign_is_none() {
        // Non-ASCII-digit characters are rejected
        assert_eq!(parse_heading_style("Outline +1"), None);
    }

    #[test]
    fn heading_tab_separator_returns_level() {
        // Tab characters are handled by trim()
        assert_eq!(parse_heading_style("Heading\t2"), Some(2));
    }
}
