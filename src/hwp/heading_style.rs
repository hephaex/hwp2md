/// Tier-4: Korean regulation document text patterns.
///
/// Returns `Some(level)` when `text` matches a Korean statute heading marker:
/// - 제N편 (part)    → H1
/// - 제N장 (chapter) → H1
/// - 제N절 (section) → H2
/// - 제N조 (article) → H2
///
/// Returns `None` for body paragraphs, paragraphs ≥ 100 chars, or patterns
/// that do not meet the heading-terminator boundary rule.
///
/// This function is shared by the HWP binary reader (`hwp::convert`) and the
/// HWPX reader (`hwpx::context::flush`) so that the same tier-4 detection
/// fires regardless of the source format.
pub(crate) fn detect_korean_regulation_heading(text: &str) -> Option<u8> {
    // Long paragraphs are article bodies, not headings.
    if text.chars().count() >= 100 {
        return None;
    }
    let trimmed = text.trim_start();
    let rest = trimmed.strip_prefix('제')?;
    // Must be followed by one or more ASCII digits.
    let digit_end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    if digit_end == 0 {
        return None;
    }
    let suffix = &rest[digit_end..];
    let (level, after_suffix) = if let Some(r) = suffix.strip_prefix('편') {
        (1u8, r)
    } else if let Some(r) = suffix.strip_prefix('장') {
        (1u8, r)
    } else if let Some(r) = suffix.strip_prefix('절') {
        (2u8, r)
    } else if let Some(r) = suffix.strip_prefix('조') {
        (2u8, r)
    } else {
        return None;
    };
    // The character after 편/장/절/조 must be a heading terminator or end-of-string.
    // Exception: "의" followed by digits is amendment sub-article notation (제N조의M).
    match after_suffix.chars().next() {
        None => Some(level),
        Some(c) if is_heading_terminator(c) => Some(level),
        Some('의') => {
            let after_ui = &after_suffix['의'.len_utf8()..];
            if after_ui.chars().next().is_some_and(|d| d.is_ascii_digit()) {
                Some(level)
            } else {
                None
            }
        }
        Some(_) => None,
    }
}

/// Returns true if `c` is a valid boundary character immediately after 편/장/절/조.
///
/// `is_whitespace()` follows Rust's Unicode White_Space property: U+0020, U+0009
/// (tab), U+00A0 (NBSP), U+3000 (ideographic space), U+202F (narrow NBSP), etc.
/// Note: zero-width chars (U+200B ZWSP, U+FEFF BOM) are NOT White_Space and will
/// NOT match this branch.
///
/// Policy note — ASCII vs fullwidth semicolons:
/// - `'；'` (U+FF1B, fullwidth) is in the allowlist.
/// - `';'` (U+003B, ASCII) is intentionally excluded; it does not appear after
///   편/장/절/조 in Korean statute text, and promoting it could mask typos.
pub(crate) fn is_heading_terminator(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            // ASCII openers / closers
            '(' | ')' | '[' | ']'
            // CJK openers
            | '「' | '『' | '<' | '《' | '〈'
            // CJK closers (symmetric — "제3조」" in citations is not a heading-start)
            | '」' | '』' | '>' | '》' | '〉'
            // Fullwidth parens (common in older Korean OCR documents)
            | '（' | '）'
            // Punctuation
            | ':' | '：' | '.' | '．' | ',' | '，' | '-' | '~' | '·' | 'ㆍ' | '；' | '…'
        )
}

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
