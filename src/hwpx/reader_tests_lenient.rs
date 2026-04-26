use super::*;

// -----------------------------------------------------------------------
// B-3: Lenient XML parsing — error recovery and missing-attribute handling
// -----------------------------------------------------------------------

/// Helper: parse section XML and expect success (no panic, no Err).
fn section(xml: &str) -> ir::Section {
    parse_section_xml(xml).expect("parse_section_xml must not fail")
}

// -----------------------------------------------------------------------
// Regression: valid XML still parses correctly
// -----------------------------------------------------------------------

#[test]
fn valid_xml_parses_normally() {
    let xml = r#"<root><hp:p><hp:run><hp:t>Hello</hp:t></hp:run></hp:p></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            assert_eq!(inlines[0].text, "Hello");
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

// -----------------------------------------------------------------------
// Malformed / truncated XML — partial result rather than error
// -----------------------------------------------------------------------

/// `parse_section_xml_with_face_names` (and `parse_section_xml`) must never
/// return `Err` for malformed XML; instead they stop at the parse error and
/// return whatever was accumulated so far.
#[test]
fn truncated_xml_returns_partial_result() {
    // XML that is truncated mid-stream (missing closing tags).
    let xml = r#"<root><hp:p><hp:run><hp:t>partial content"#;
    // Must not return Err — returns an empty-or-partial section.
    let result = parse_section_xml(xml);
    assert!(
        result.is_ok(),
        "truncated XML must return Ok (partial result), got: {result:?}"
    );
    // The accumulated text "partial content" was not flushed because the
    // paragraph end tag never arrived, so zero blocks is acceptable.
    let s = result.expect("Ok variant");
    // Either 0 blocks (text never flushed) or 1 block is fine — the key
    // invariant is that the function did not return Err.
    let _ = s.blocks.len();
}

#[test]
fn xml_with_unclosed_element_returns_ok() {
    // Only the opening tag, no closing tag.
    let xml = r#"<root><hp:p>"#;
    let result = parse_section_xml(xml);
    assert!(
        result.is_ok(),
        "XML with unclosed element must return Ok; got: {result:?}"
    );
}

#[test]
fn completely_malformed_xml_returns_ok_with_no_blocks() {
    let xml = "<<<not xml at all>>>";
    let result = parse_section_xml(xml);
    assert!(
        result.is_ok(),
        "completely invalid XML must return Ok (empty section); got: {result:?}"
    );
    let s = result.expect("Ok variant");
    assert!(
        s.blocks.is_empty(),
        "completely invalid XML must produce no blocks"
    );
}

// -----------------------------------------------------------------------
// Missing optional attributes — sensible defaults, no panic
// -----------------------------------------------------------------------

/// `colSpan` missing from cellAddr → defaults to 1.
#[test]
fn celladdr_missing_colspan_defaults_to_one() {
    let xml = concat!(
        r#"<root><hp:tbl colCnt="1"><hp:tr>"#,
        r#"<hp:tc><hp:cellAddr rowSpan="2"/>"#,
        r#"<hp:p><hp:run><hp:t>cell</hp:t></hp:run></hp:p></hp:tc>"#,
        r#"</hp:tr></hp:tbl></root>"#,
    );
    let s = section(xml);
    match &s.blocks[0] {
        ir::Block::Table { rows, .. } => {
            assert_eq!(
                rows[0].cells[0].colspan, 1,
                "missing colSpan must default to 1"
            );
            assert_eq!(
                rows[0].cells[0].rowspan, 2,
                "rowSpan must be read correctly"
            );
        }
        other => panic!("expected Table, got {other:?}"),
    }
}

/// `rowSpan` missing from cellAddr → defaults to 1.
#[test]
fn celladdr_missing_rowspan_defaults_to_one() {
    let xml = concat!(
        r#"<root><hp:tbl colCnt="1"><hp:tr>"#,
        r#"<hp:tc><hp:cellAddr colSpan="3"/>"#,
        r#"<hp:p><hp:run><hp:t>wide</hp:t></hp:run></hp:p></hp:tc>"#,
        r#"</hp:tr></hp:tbl></root>"#,
    );
    let s = section(xml);
    match &s.blocks[0] {
        ir::Block::Table { rows, .. } => {
            assert_eq!(rows[0].cells[0].colspan, 3, "colSpan must be read");
            assert_eq!(
                rows[0].cells[0].rowspan, 1,
                "missing rowSpan must default to 1"
            );
        }
        other => panic!("expected Table, got {other:?}"),
    }
}

/// `id` missing from footnote element — must default to empty string, no panic.
#[test]
fn footnote_missing_id_attribute_uses_empty_string() {
    let xml = r#"<root><hp:fn><hp:p><hp:run><hp:t>no id</hp:t></hp:run></hp:p></hp:fn></root>"#;
    let s = section(xml);
    // The footnote block must still be emitted with the text preserved.
    let found = s.blocks.iter().any(|b| match b {
        ir::Block::Footnote { id, content } => {
            id.is_empty()
                && content.iter().any(|inner| {
                    matches!(inner, ir::Block::Paragraph { inlines }
                        if inlines.iter().any(|i| i.text == "no id"))
                })
        }
        _ => false,
    });
    assert!(
        found,
        "footnote with missing id must produce a Footnote block with empty id; blocks: {:?}",
        s.blocks
    );
}

/// `styleIDRef` missing from paragraph → no heading, plain paragraph.
#[test]
fn paragraph_missing_style_id_ref_is_plain() {
    let xml = r#"<root><hp:p><hp:run><hp:t>plain para</hp:t></hp:run></hp:p></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            assert_eq!(inlines[0].text, "plain para");
        }
        other => panic!("expected plain Paragraph (no heading), got {other:?}"),
    }
}

/// faceNameIDRef out of range → font_name stays None, no panic.
#[test]
fn charpr_out_of_range_face_name_id_ref_ignored() {
    // faceNameIDRef=9999 — way beyond any populated face table.
    let xml = r#"<root><hp:p><hp:run><hp:charPr faceNameIDRef="9999"/><hp:t>text</hp:t></hp:run></hp:p></root>"#;
    let s = parse_section_xml_with_face_names(xml, &["Font A".to_string()]).expect("Ok");
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            assert_eq!(inlines[0].text, "text");
            assert!(
                inlines[0].font_name.is_none(),
                "out-of-range faceNameIDRef must leave font_name as None; got: {:?}",
                inlines[0].font_name
            );
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

/// Non-numeric faceNameIDRef → font_name stays None, no panic.
#[test]
fn charpr_non_numeric_face_name_id_ref_ignored() {
    let xml = r#"<root><hp:p><hp:run><hp:charPr faceNameIDRef="notanumber"/><hp:t>text</hp:t></hp:run></hp:p></root>"#;
    let s = parse_section_xml_with_face_names(xml, &["Font A".to_string()]).expect("Ok");
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            assert!(
                inlines[0].font_name.is_none(),
                "non-numeric faceNameIDRef must be silently ignored"
            );
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

// -----------------------------------------------------------------------
// Partial document extraction — content before error is preserved
// -----------------------------------------------------------------------

/// When valid content appears before a parse error, that content must
/// be returned in the partial result.
#[test]
fn content_before_parse_error_is_preserved() {
    // Two valid paragraphs followed by malformed XML.
    // quick-xml will stop at the first error; both complete paragraphs
    // should already have been flushed into section.blocks before the error.
    let xml =
        "<root><hp:p><hp:run><hp:t>first</hp:t></hp:run></hp:p><hp:p><hp:run><hp:t>second</hp:t></hp:run></hp:p><<<BAD";
    let result = parse_section_xml(xml);
    assert!(result.is_ok(), "must return Ok despite trailing bad XML");
    let s = result.expect("Ok");
    assert!(
        !s.blocks.is_empty(),
        "at least the first paragraph must be preserved before parse error; blocks: {:?}",
        s.blocks
    );
    let has_first = s.blocks.iter().any(|b| match b {
        ir::Block::Paragraph { inlines } => inlines.iter().any(|i| i.text == "first"),
        _ => false,
    });
    assert!(
        has_first,
        "first paragraph text must survive despite subsequent malformed XML; blocks: {:?}",
        s.blocks
    );
}

// -----------------------------------------------------------------------
// Missing section file — already handled in read_hwpx; document-level test
// -----------------------------------------------------------------------

/// `parse_section_xml` on an empty string should not panic and returns
/// an empty section (the HWPX reader calls this per-section and already
/// wraps each call with a warn+skip on error, so this is the inner path).
#[test]
fn parse_section_xml_on_empty_string_returns_empty_section() {
    let s = section("");
    assert!(
        s.blocks.is_empty(),
        "empty string must produce an empty section, not panic"
    );
}

/// Whitespace-only XML produces an empty section without panicking.
#[test]
fn parse_section_xml_whitespace_only_returns_empty_section() {
    let s = section("   \n  \t  ");
    assert!(
        s.blocks.is_empty(),
        "whitespace-only input must produce an empty section"
    );
}
