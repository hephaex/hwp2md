use super::*;

// -----------------------------------------------------------------------
// parse_heading_style tests — shared implementation from hwp::heading_style
// -----------------------------------------------------------------------

#[test]
fn parse_heading_style_heading1() {
    assert_eq!(parse_heading_style("Heading1"), Some(1));
}

#[test]
fn parse_heading_style_heading6() {
    assert_eq!(parse_heading_style("Heading6"), Some(6));
}

#[test]
fn parse_heading_style_korean_title() {
    // "제목1" -> 1
    assert_eq!(parse_heading_style("제목1"), Some(1));
}

#[test]
fn parse_heading_style_korean_outline_3() {
    // "개요3" -> 3
    assert_eq!(parse_heading_style("개요3"), Some(3));
}

#[test]
fn parse_heading_style_normal_is_none() {
    assert_eq!(parse_heading_style("Normal"), None);
}

#[test]
fn parse_heading_style_body_text_is_none() {
    assert_eq!(parse_heading_style("BodyText"), None);
}

// "Heading" without a trailing digit: the shared implementation requires a
// digit in range 1–6 after the prefix — no digit means no match → None.
#[test]
fn parse_heading_style_heading_no_digit_returns_none() {
    assert_eq!(parse_heading_style("Heading"), None);
}

#[test]
fn parse_heading_style_case_insensitive() {
    assert_eq!(parse_heading_style("HEADING2"), Some(2));
}

// Bare numeric strings are not style names and return None.
#[test]
fn parse_heading_style_numeric_1_is_none() {
    assert_eq!(parse_heading_style("1"), None);
}

#[test]
fn parse_heading_style_numeric_6_is_none() {
    assert_eq!(parse_heading_style("6"), None);
}

#[test]
fn parse_heading_style_numeric_7_is_none() {
    assert_eq!(parse_heading_style("7"), None);
}

#[test]
fn parse_heading_style_numeric_0_is_none() {
    assert_eq!(parse_heading_style("0"), None);
}
