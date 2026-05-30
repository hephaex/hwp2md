/// Integration tests for HWPX inline formatting charPr attributes.
///
/// Covers: bold/italic, underline, superscript/subscript, strikethrough,
/// color, and combinations thereof.  All tests construct minimal HWPX
/// fixtures via `HwpxFixture` and verify both the IR layer and the final
/// Markdown output.
///
/// Extracted from integration.rs (Sprints 88-91) to keep each test file
/// focused and within a manageable size.
#[path = "fixtures/mod.rs"]
#[allow(dead_code)]
mod fixtures;

use fixtures::{read_fixture, styled_run_xml, HwpxFixture};
use hwp2md::{ir, md};

// ---------------------------------------------------------------------------
// Sprint 88 P2: HWPX inline formatting (bold/italic/underline) integration
// ---------------------------------------------------------------------------

/// Bold and italic charPr attributes produce the correct IR inline flags
/// and render as `***text***` in GFM Markdown.
#[test]
fn hwpx_charpr_bold_italic_produces_formatted_inline() {
    let body = styled_run_xml("Hello");
    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&body));

    // IR layer: inline must have bold=true, italic=true.
    let bold_italic_inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.bold && i.italic)
            } else {
                None
            }
        });

    assert!(
        bold_italic_inline.is_some(),
        "expected an inline with bold=true, italic=true; blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    assert_eq!(
        bold_italic_inline.unwrap().text,
        "Hello",
        "bold+italic inline text mismatch"
    );

    // Markdown layer: bold+italic → ***Hello***.
    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("***Hello***"),
        "bold+italic must render as ***Hello***; got: {markdown:?}"
    );
}

/// Underline charPr produces `<u>text</u>` in Markdown.
#[test]
fn hwpx_charpr_underline_produces_html_u_tag() {
    let underline_xml = r#"<hp:p paraPrIDRef="0"><hp:run charPrIDRef="0">
        <hp:charPr underline="single"/>
        <hp:t>Underlined</hp:t>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(underline_xml));

    let underline_inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.underline)
            } else {
                None
            }
        });

    assert!(
        underline_inline.is_some(),
        "expected an inline with underline=true; blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );

    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("<u>Underlined</u>"),
        "underline must render as <u>Underlined</u>; got: {markdown:?}"
    );
}

// ---------------------------------------------------------------------------
// Sprint 88 P3: HWPX superscript/subscript charPr integration test
// ---------------------------------------------------------------------------
//
// OWPML uses attribute name "supscript" (not "superscript") with values:
//   supscript="superscript" → superscript mode
//   supscript="subscript"   → subscript mode
// The handler is in context/flush.rs:124-126.

/// `supscript="superscript"` charPr → `ir::Inline.superscript=true`
/// → `<sup>text</sup>` in Markdown.
#[test]
fn hwpx_charpr_superscript_produces_sup_html() {
    let sup_xml = r#"<hp:p paraPrIDRef="0"><hp:run charPrIDRef="0">
        <hp:charPr supscript="superscript"/>
        <hp:t>2</hp:t>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(sup_xml));

    let sup_inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.superscript && !i.subscript)
            } else {
                None
            }
        });

    assert!(
        sup_inline.is_some(),
        "expected an inline with superscript=true; got: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("<sup>2</sup>"),
        "superscript must render as <sup>2</sup>; got: {markdown:?}"
    );
}

/// `supscript="subscript"` charPr → `ir::Inline.subscript=true`
/// → `<sub>text</sub>` in Markdown.
#[test]
fn hwpx_charpr_subscript_produces_sub_html() {
    let sub_xml = r#"<hp:p paraPrIDRef="0"><hp:run charPrIDRef="0">
        <hp:charPr supscript="subscript"/>
        <hp:t>i</hp:t>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(sub_xml));

    let sub_inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.subscript && !i.superscript)
            } else {
                None
            }
        });

    assert!(
        sub_inline.is_some(),
        "expected an inline with subscript=true; got: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("<sub>i</sub>"),
        "subscript must render as <sub>i</sub>; got: {markdown:?}"
    );
}

// ---------------------------------------------------------------------------
// Sprint 89 P2: strikethrough + color charPr integration tests
// ---------------------------------------------------------------------------

/// `strikeout="single"` charPr → `ir::Inline.strikethrough=true` → `~~text~~`.
#[test]
fn hwpx_charpr_strikeout_produces_gfm_strikethrough() {
    let strike_xml = r#"<hp:p paraPrIDRef="0"><hp:run charPrIDRef="0">
        <hp:charPr strikeout="single"/>
        <hp:t>Deleted text</hp:t>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(strike_xml));

    let strike_inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.strikethrough)
            } else {
                None
            }
        });

    assert!(
        strike_inline.is_some(),
        "expected inline with strikethrough=true; blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    assert_eq!(strike_inline.unwrap().text, "Deleted text");

    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("~~Deleted text~~"),
        "strikeout must render as ~~Deleted text~~; got: {markdown:?}"
    );
}

/// Non-black `color` charPr → `<span style="color:#RRGGBB">text</span>` in Markdown.
/// Black (#000000) is treated as "no color" and does not emit a span.
#[test]
fn hwpx_charpr_non_black_color_produces_span_html() {
    let color_xml = r#"<hp:p paraPrIDRef="0"><hp:run charPrIDRef="0">
        <hp:charPr color="FF0000"/>
        <hp:t>Red text</hp:t>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(color_xml));

    let colored_inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.color.is_some())
            } else {
                None
            }
        });

    assert!(
        colored_inline.is_some(),
        "expected inline with color set; blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    // Color stored as #RRGGBB uppercase.
    assert_eq!(
        colored_inline.unwrap().color.as_deref(),
        Some("#FF0000"),
        "color must be stored as #FF0000"
    );

    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("<span style=\"color:#FF0000\">Red text</span>"),
        "non-black color must render as <span>; got: {markdown:?}"
    );
}

/// Black color (#000000) must NOT produce a span in Markdown output.
/// The reader treats black as "no color" (apply_charpr_attrs sets color to None).
#[test]
fn hwpx_charpr_black_color_produces_no_span() {
    let black_xml = r#"<hp:p paraPrIDRef="0"><hp:run charPrIDRef="0">
        <hp:charPr color="000000"/>
        <hp:t>Black text</hp:t>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(black_xml));

    let markdown = md::write_markdown(&doc, false);
    assert!(
        !markdown.contains("<span"),
        "black color must not produce a <span>; got: {markdown:?}"
    );
    assert!(
        markdown.contains("Black text"),
        "text must still appear; got: {markdown:?}"
    );
}

// ---------------------------------------------------------------------------
// Sprint 90 P3: Combined inline formatting integration tests
// ---------------------------------------------------------------------------

/// `bold="true" underline="single"` applied together produces an inline with
/// both flags set; Markdown renders as `<u>**text**</u>` (underline wraps bold).
#[test]
fn hwpx_charpr_bold_underline_combined_produces_wrapped_markdown() {
    let xml = r#"<hp:p paraPrIDRef="0"><hp:run charPrIDRef="0">
        <hp:charPr bold="true" underline="single"/>
        <hp:t>BoldUnderline</hp:t>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(xml));

    let inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.text.contains("BoldUnderline"))
            } else {
                None
            }
        });

    assert!(inline.is_some(), "expected BoldUnderline inline");
    let inline = inline.unwrap();
    assert!(inline.bold, "bold must be true");
    assert!(inline.underline, "underline must be true");

    let markdown = md::write_markdown(&doc, false);
    // Rendering order: bold → then underline wraps → <u>**BoldUnderline**</u>
    assert!(
        markdown.contains("<u>**BoldUnderline**</u>"),
        "bold+underline must render as <u>**text**</u>; got: {markdown:?}"
    );
}

/// `bold="true" italic="true" color="0000FF"` applied together produces bold,
/// italic, and color in the IR; Markdown renders
/// `<span style="color:#0000FF">***text***</span>`.
#[test]
fn hwpx_charpr_bold_italic_color_combined_produces_span_with_markers() {
    let xml = r#"<hp:p paraPrIDRef="0"><hp:run charPrIDRef="0">
        <hp:charPr bold="true" italic="true" color="0000FF"/>
        <hp:t>BlueItalicBold</hp:t>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(xml));

    let inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.text.contains("BlueItalicBold"))
            } else {
                None
            }
        });

    assert!(inline.is_some(), "expected BlueItalicBold inline");
    let inline = inline.unwrap();
    assert!(inline.bold, "bold must be true");
    assert!(inline.italic, "italic must be true");
    assert_eq!(
        inline.color.as_deref(),
        Some("#0000FF"),
        "color must be #0000FF"
    );

    let markdown = md::write_markdown(&doc, false);
    // Rendering order: bold+italic → ***text*** → color wraps → <span>***text***</span>
    assert!(
        markdown.contains("<span style=\"color:#0000FF\">***BlueItalicBold***</span>"),
        "bold+italic+color must render as <span>***text***</span>; got: {markdown:?}"
    );
}

// ---------------------------------------------------------------------------
// Sprint 91 P3: Additional inline formatting combination tests
// ---------------------------------------------------------------------------
// These tests pin the writer rendering order for combinations not yet covered:
//   strikethrough THEN underline → <u>~~text~~</u>
//   bold THEN strikethrough     → ~~**text**~~
//   italic THEN underline       → <u>*text*</u>

/// `strikeout="single" underline="single"` in one charPr: strikethrough is
/// applied first (`~~`), then underline wraps → `<u>~~text~~</u>`.
#[test]
fn hwpx_charpr_strikethrough_underline_combined() {
    let xml = r#"<hp:p paraPrIDRef="0"><hp:run charPrIDRef="0">
        <hp:charPr strikeout="single" underline="single"/>
        <hp:t>StrikeUnderline</hp:t>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(xml));

    let inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.text.contains("StrikeUnderline"))
            } else {
                None
            }
        });

    assert!(inline.is_some(), "expected StrikeUnderline inline");
    let inline = inline.unwrap();
    assert!(inline.strikethrough, "strikethrough must be true");
    assert!(inline.underline, "underline must be true");

    let markdown = md::write_markdown(&doc, false);
    // Rendering order (src/md/writer.rs): bold/italic → strikethrough → underline
    // → <u>~~StrikeUnderline~~</u>
    assert!(
        markdown.contains("<u>~~StrikeUnderline~~</u>"),
        "strikethrough+underline must render as <u>~~text~~</u>; got: {markdown:?}"
    );
}

/// `bold="true" strikeout="single"` in one charPr: bold is applied first
/// (`**`), then strikethrough wraps → `~~**text**~~`.
#[test]
fn hwpx_charpr_bold_strikethrough_combined() {
    let xml = r#"<hp:p paraPrIDRef="0"><hp:run charPrIDRef="0">
        <hp:charPr bold="true" strikeout="single"/>
        <hp:t>BoldStrike</hp:t>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(xml));

    let inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.text.contains("BoldStrike"))
            } else {
                None
            }
        });

    assert!(inline.is_some(), "expected BoldStrike inline");
    let inline = inline.unwrap();
    assert!(inline.bold, "bold must be true");
    assert!(inline.strikethrough, "strikethrough must be true");

    let markdown = md::write_markdown(&doc, false);
    // Rendering order: bold → **BoldStrike** → strikethrough wraps → ~~**BoldStrike**~~
    assert!(
        markdown.contains("~~**BoldStrike**~~"),
        "bold+strikethrough must render as ~~**text**~~; got: {markdown:?}"
    );
}

/// `italic="true" underline="single"` in one charPr: italic is applied first
/// (`*`), then underline wraps → `<u>*text*</u>`.
#[test]
fn hwpx_charpr_italic_underline_combined() {
    let xml = r#"<hp:p paraPrIDRef="0"><hp:run charPrIDRef="0">
        <hp:charPr italic="true" underline="single"/>
        <hp:t>ItalicUnderline</hp:t>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(xml));

    let inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.text.contains("ItalicUnderline"))
            } else {
                None
            }
        });

    assert!(inline.is_some(), "expected ItalicUnderline inline");
    let inline = inline.unwrap();
    assert!(inline.italic, "italic must be true");
    assert!(inline.underline, "underline must be true");

    let markdown = md::write_markdown(&doc, false);
    // Rendering order: italic → *ItalicUnderline* → underline wraps → <u>*ItalicUnderline*</u>
    assert!(
        markdown.contains("<u>*ItalicUnderline*</u>"),
        "italic+underline must render as <u>*text*</u>; got: {markdown:?}"
    );
}
