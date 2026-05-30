/// Integration tests for HWPX hyperlink (fieldBegin/fieldEnd) handling.
///
/// Covers: basic link creation, fieldEnd clearing, multiple links in one
/// paragraph, unsafe URL filtering, and inline formatting inside a hyperlink.
///
/// Extracted from integration.rs (Sprints 82, 90, 92) to keep each test
/// file focused.  New Sprint 92 tests pin the writer rendering order when
/// bold, italic, or color is combined with a hyperlink.
#[path = "fixtures/mod.rs"]
#[allow(dead_code)]
mod fixtures;

use fixtures::{read_fixture, HwpxFixture};
use hwp2md::{ir, md};

// ---------------------------------------------------------------------------
// Sprint 82 P3: HWPX hyperlink field begin/end integration
// ---------------------------------------------------------------------------

/// A `<hp:fieldBegin type="HYPERLINK" command="url"/>` … `<hp:fieldEnd/>` sequence
/// must produce an inline carrying the link URL, rendered as Markdown `[text](url)`.
#[test]
fn hwpx_hyperlink_field_begin_end_produces_link_inline() {
    let hyperlink_xml = r#"<hp:p><hp:run>
        <hp:fieldBegin type="HYPERLINK" command="https://example.com"/>
        <hp:t>Click here</hp:t>
        <hp:fieldEnd/>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(hyperlink_xml));

    let link_inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.link.is_some())
            } else {
                None
            }
        });

    assert!(
        link_inline.is_some(),
        "expected an inline with link; blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    let link_inline = link_inline.unwrap();
    assert_eq!(link_inline.text, "Click here", "link text mismatch");
    assert_eq!(
        link_inline.link.as_deref(),
        Some("https://example.com"),
        "link URL mismatch"
    );

    // Markdown rendering: [text](url) format.
    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("[Click here](https://example.com)"),
        "markdown must contain [text](url) link; got: {markdown:?}"
    );
}

/// Text after `<hp:fieldEnd/>` must NOT carry the hyperlink.
/// Pins that in_hyperlink is properly cleared on fieldEnd.
#[test]
fn hwpx_hyperlink_text_after_field_end_has_no_link() {
    let hyperlink_xml = r#"<hp:p><hp:run>
        <hp:fieldBegin type="HYPERLINK" command="https://example.com"/>
        <hp:t>linked</hp:t>
        <hp:fieldEnd/>
        <hp:t> plain</hp:t>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(hyperlink_xml));

    let inlines: Vec<_> = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .filter_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                Some(inlines.as_slice())
            } else {
                None
            }
        })
        .flatten()
        .collect();

    // Positive: "linked" must carry the URL (verifies fieldBegin actually worked).
    assert!(
        inlines
            .iter()
            .any(|i| i.text.contains("linked") && i.link.as_deref() == Some("https://example.com")),
        "linked text must carry the URL; inlines: {inlines:?}"
    );
    // Negative: "plain" (after fieldEnd) must have no link.
    assert!(
        inlines
            .iter()
            .any(|i| i.text.contains("plain") && i.link.is_none()),
        "text after fieldEnd must not carry the URL; inlines: {inlines:?}"
    );
}

// ---------------------------------------------------------------------------
// Sprint 90 P2: HWPX hyperlink edge cases
// ---------------------------------------------------------------------------

/// Two hyperlinks in one paragraph each carry their own URL with no bleed
/// from the first link into the second or the plain text between them.
#[test]
fn hwpx_hyperlink_multiple_links_in_same_paragraph() {
    let xml = r#"<hp:p><hp:run>
        <hp:fieldBegin type="HYPERLINK" command="https://first.com"/>
        <hp:t>First</hp:t>
        <hp:fieldEnd/>
    </hp:run><hp:run>
        <hp:t> and </hp:t>
    </hp:run><hp:run>
        <hp:fieldBegin type="HYPERLINK" command="https://second.com"/>
        <hp:t>Second</hp:t>
        <hp:fieldEnd/>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(xml));

    let inlines: Vec<&ir::Inline> = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .filter_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                Some(inlines.as_slice())
            } else {
                None
            }
        })
        .flatten()
        .collect();

    let first = inlines.iter().find(|i| i.text.contains("First"));
    let and_text = inlines.iter().find(|i| i.text.contains("and"));
    let second = inlines.iter().find(|i| i.text.contains("Second"));

    assert!(first.is_some(), "expected inline with 'First'");
    assert!(second.is_some(), "expected inline with 'Second'");

    assert_eq!(
        first.unwrap().link.as_deref(),
        Some("https://first.com"),
        "'First' inline must carry first URL"
    );
    assert_eq!(
        second.unwrap().link.as_deref(),
        Some("https://second.com"),
        "'Second' inline must carry second URL"
    );
    assert!(
        and_text.is_some(),
        "expected plain text inline between the two hyperlinks; inlines: {inlines:?}"
    );
    if let Some(mid) = and_text {
        assert!(
            mid.link.is_none(),
            "plain text between links must have no URL; got: {:?}",
            mid.link
        );
    }

    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("[First](https://first.com)"),
        "markdown must contain first link; got: {markdown:?}"
    );
    assert!(
        markdown.contains("[Second](https://second.com)"),
        "markdown must contain second link; got: {markdown:?}"
    );
}

/// A HYPERLINK with a javascript: URL is stored in the IR (no panic) but the
/// Markdown writer must drop the link syntax, emitting only the label text.
#[test]
fn hwpx_hyperlink_unsafe_url_drops_link_syntax() {
    let xml = r#"<hp:p><hp:run>
        <hp:fieldBegin type="HYPERLINK" command="javascript:alert(1)"/>
        <hp:t>click</hp:t>
        <hp:fieldEnd/>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(xml));

    let link_inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.text.contains("click"))
            } else {
                None
            }
        });

    assert!(link_inline.is_some(), "expected inline with text 'click'");
    // Assert the IR retains the raw URL — the writer gate is what drops it,
    // not the parser.  If the parser rejected it, moving the gate to parse time
    // would not be detected by the Markdown-level assertions alone.
    assert_eq!(
        link_inline.unwrap().link.as_deref(),
        Some("javascript:alert(1)"),
        "IR must carry the raw URL so the md::writer security gate is what filters it"
    );

    let markdown = md::write_markdown(&doc, false);
    assert!(
        !markdown.contains("javascript:"),
        "javascript: URL must not appear in markdown output; got: {markdown:?}"
    );
    assert!(
        !markdown.contains("[click]("),
        "unsafe URL must not produce [text](url) syntax; got: {markdown:?}"
    );
    assert!(
        markdown.contains("click"),
        "label text must still be present; got: {markdown:?}"
    );
}

// ---------------------------------------------------------------------------
// Sprint 92 P3: Inline formatting combined with hyperlink
// ---------------------------------------------------------------------------
// The md::writer applies link wrapping LAST (after all inline decorations),
// so: bold → `**text**` → link wraps → `[**text**](url)`.

/// `bold="true"` charPr inside a hyperlink field produces an inline with both
/// `bold=true` and `link=Some(url)`; Markdown renders as `[**BoldLink**](url)`.
#[test]
fn hwpx_hyperlink_bold_text_renders_as_formatted_link() {
    let xml = r#"<hp:p><hp:run charPrIDRef="0">
        <hp:charPr bold="true"/>
        <hp:fieldBegin type="HYPERLINK" command="https://bold.example.com"/>
        <hp:t>BoldLink</hp:t>
        <hp:fieldEnd/>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(xml));

    let inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.text.contains("BoldLink"))
            } else {
                None
            }
        });

    assert!(inline.is_some(), "expected BoldLink inline");
    let inline = inline.unwrap();
    assert!(inline.bold, "bold must be true");
    assert_eq!(
        inline.link.as_deref(),
        Some("https://bold.example.com"),
        "link URL must be set"
    );

    // Writer order: bold → link wraps → [**BoldLink**](url)
    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("[**BoldLink**](https://bold.example.com)"),
        "bold link must render as [**text**](url); got: {markdown:?}"
    );
}

/// `italic="true"` charPr inside a hyperlink field produces an inline with both
/// `italic=true` and `link=Some(url)`; Markdown renders as `[*ItalicLink*](url)`.
#[test]
fn hwpx_hyperlink_italic_text_renders_as_formatted_link() {
    let xml = r#"<hp:p><hp:run charPrIDRef="0">
        <hp:charPr italic="true"/>
        <hp:fieldBegin type="HYPERLINK" command="https://italic.example.com"/>
        <hp:t>ItalicLink</hp:t>
        <hp:fieldEnd/>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(xml));

    let inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.text.contains("ItalicLink"))
            } else {
                None
            }
        });

    assert!(inline.is_some(), "expected ItalicLink inline");
    let inline = inline.unwrap();
    assert!(inline.italic, "italic must be true");
    assert_eq!(
        inline.link.as_deref(),
        Some("https://italic.example.com"),
        "link URL must be set"
    );

    // Writer order: italic → link wraps → [*ItalicLink*](url)
    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("[*ItalicLink*](https://italic.example.com)"),
        "italic link must render as [*text*](url); got: {markdown:?}"
    );
}

/// `color="FF0000"` charPr inside a hyperlink field produces an inline with
/// both color and link set; Markdown renders as
/// `[<span style="color:#FF0000">ColorLink</span>](url)`.
#[test]
fn hwpx_hyperlink_colored_text_renders_as_span_link() {
    let xml = r#"<hp:p><hp:run charPrIDRef="0">
        <hp:charPr color="FF0000"/>
        <hp:fieldBegin type="HYPERLINK" command="https://color.example.com"/>
        <hp:t>ColorLink</hp:t>
        <hp:fieldEnd/>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(xml));

    let inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.text.contains("ColorLink"))
            } else {
                None
            }
        });

    assert!(inline.is_some(), "expected ColorLink inline");
    let inline = inline.unwrap();
    assert_eq!(
        inline.color.as_deref(),
        Some("#FF0000"),
        "color must be #FF0000"
    );
    assert_eq!(
        inline.link.as_deref(),
        Some("https://color.example.com"),
        "link URL must be set"
    );

    // Writer order: color span → link wraps
    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("[<span style=\"color:#FF0000\">ColorLink</span>](https://color.example.com)"),
        "color link must render as [<span>text</span>](url); got: {markdown:?}"
    );
}

// ---------------------------------------------------------------------------
// Sprint 93 P3: Additional hyperlink edge cases
// ---------------------------------------------------------------------------

/// `bold="true" italic="true"` charPr inside a hyperlink produces
/// `[***BoldItalicLink***](url)` — the bold+italic branch of the writer.
#[test]
fn hwpx_hyperlink_bold_italic_text_renders_as_bold_italic_link() {
    let xml = r#"<hp:p><hp:run charPrIDRef="0">
        <hp:charPr bold="true" italic="true"/>
        <hp:fieldBegin type="HYPERLINK" command="https://bolditalic.example.com"/>
        <hp:t>BoldItalicLink</hp:t>
        <hp:fieldEnd/>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(xml));

    let inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.text.contains("BoldItalicLink"))
            } else {
                None
            }
        });

    assert!(inline.is_some(), "expected BoldItalicLink inline");
    let inline = inline.unwrap();
    assert!(inline.bold, "bold must be true");
    assert!(inline.italic, "italic must be true");
    assert_eq!(
        inline.link.as_deref(),
        Some("https://bolditalic.example.com"),
        "link URL must be set"
    );

    // Writer order: bold+italic → ***text*** → link wraps → [***text***](url)
    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("[***BoldItalicLink***](https://bolditalic.example.com)"),
        "bold+italic link must render as [***text***](url); got: {markdown:?}"
    );
}

/// A hyperlink URL containing `)` must use the angle-bracket form
/// `[text](<url>)` to prevent Markdown parsers from ending the link early.
/// Pins the `url.contains(')')` branch in src/md/writer.rs.
#[test]
fn hwpx_hyperlink_url_with_closing_paren_uses_angle_bracket_form() {
    let url = "https://example.com/path(1)";
    let xml = format!(
        r#"<hp:p><hp:run>
        <hp:fieldBegin type="HYPERLINK" command="{url}"/>
        <hp:t>ParenLink</hp:t>
        <hp:fieldEnd/>
    </hp:run></hp:p>"#
    );

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&xml));

    let inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.link.is_some())
            } else {
                None
            }
        });

    assert!(inline.is_some(), "expected a link inline");
    assert_eq!(
        inline.unwrap().link.as_deref(),
        Some(url),
        "IR must carry the URL with the paren"
    );

    // Writer must use [text](<url>) angle-bracket form when URL contains ')'.
    let markdown = md::write_markdown(&doc, false);
    let expected = format!("[ParenLink](<{url}>)");
    assert!(
        markdown.contains(&expected),
        "URL with ')' must use angle-bracket form; got: {markdown:?}"
    );
    // Standard form [text](url) must NOT be used (it would break parsers).
    let plain_form = format!("[ParenLink]({url})");
    assert!(
        !markdown.contains(&plain_form),
        "plain [text](url) form must not be used for URL with ')'; got: {markdown:?}"
    );
}
