use super::*;

// -----------------------------------------------------------------------
// parse_heading_style tests
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

#[test]
fn parse_heading_style_heading_no_digit_defaults_to_1() {
    // "Heading" without a trailing digit -> defaults to level 1.
    assert_eq!(parse_heading_style("Heading"), Some(1));
}

#[test]
fn parse_heading_style_case_insensitive() {
    assert_eq!(parse_heading_style("HEADING2"), Some(2));
}
