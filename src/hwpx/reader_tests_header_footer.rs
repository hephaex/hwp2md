use super::*;
use crate::ir;

fn section(xml: &str) -> ir::Section {
    parse_section_xml(xml).expect("parse_section_xml must not fail")
}

// -----------------------------------------------------------------------
// B-4: header/footer parsing from <hp:headerFooter>
// -----------------------------------------------------------------------

#[test]
fn header_footer_parsed() {
    let xml = r#"<root>
        <hp:p><hp:run><hp:t>body text</hp:t></hp:run></hp:p>
        <hp:headerFooter>
            <hp:header>
                <hp:p><hp:run><hp:t>Header text</hp:t></hp:run></hp:p>
            </hp:header>
            <hp:footer>
                <hp:p><hp:run><hp:t>Footer text</hp:t></hp:run></hp:p>
            </hp:footer>
        </hp:headerFooter>
    </root>"#;
    let s = section(xml);

    // Main body block still present.
    assert_eq!(s.blocks.len(), 1, "main body must have one paragraph");

    // Header is parsed.
    let header = s.header.as_ref().expect("section.header must be Some");
    assert_eq!(header.len(), 1, "header must have one block");
    match &header[0] {
        ir::Block::Paragraph { inlines } => {
            let text: String = inlines.iter().map(|i| i.text.as_str()).collect();
            assert_eq!(text, "Header text");
        }
        other => panic!("expected Paragraph in header, got {other:?}"),
    }

    // Footer is parsed.
    let footer = s.footer.as_ref().expect("section.footer must be Some");
    assert_eq!(footer.len(), 1, "footer must have one block");
    match &footer[0] {
        ir::Block::Paragraph { inlines } => {
            let text: String = inlines.iter().map(|i| i.text.as_str()).collect();
            assert_eq!(text, "Footer text");
        }
        other => panic!("expected Paragraph in footer, got {other:?}"),
    }
}

#[test]
fn header_only_no_footer() {
    let xml = r#"<root>
        <hp:headerFooter>
            <hp:header>
                <hp:p><hp:run><hp:t>Only header</hp:t></hp:run></hp:p>
            </hp:header>
        </hp:headerFooter>
    </root>"#;
    let s = section(xml);

    let header = s.header.as_ref().expect("section.header must be Some");
    assert_eq!(header.len(), 1);
    match &header[0] {
        ir::Block::Paragraph { inlines } => {
            let text: String = inlines.iter().map(|i| i.text.as_str()).collect();
            assert_eq!(text, "Only header");
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }

    assert!(
        s.footer.is_none(),
        "section.footer must be None when no footer element is present"
    );
}

#[test]
fn no_header_footer_remains_none() {
    // A section without <hp:headerFooter> must leave header/footer as None.
    let xml = r#"<root>
        <hp:p><hp:run><hp:t>plain body</hp:t></hp:run></hp:p>
    </root>"#;
    let s = section(xml);

    assert!(
        s.header.is_none(),
        "header must be None when no headerFooter element present"
    );
    assert!(
        s.footer.is_none(),
        "footer must be None when no headerFooter element present"
    );
}

#[test]
fn footer_only_no_header() {
    let xml = r#"<root>
        <hp:headerFooter>
            <hp:footer>
                <hp:p><hp:run><hp:t>Only footer</hp:t></hp:run></hp:p>
            </hp:footer>
        </hp:headerFooter>
    </root>"#;
    let s = section(xml);

    assert!(
        s.header.is_none(),
        "section.header must be None when only footer present"
    );

    let footer = s.footer.as_ref().expect("section.footer must be Some");
    assert_eq!(footer.len(), 1);
    match &footer[0] {
        ir::Block::Paragraph { inlines } => {
            let text: String = inlines.iter().map(|i| i.text.as_str()).collect();
            assert_eq!(text, "Only footer");
        }
        other => panic!("expected Paragraph in footer, got {other:?}"),
    }
}

#[test]
fn header_footer_body_text_not_mixed_into_header() {
    // Body paragraphs that appear outside <hp:headerFooter> must NOT end up
    // in section.header / section.footer.
    let xml = r#"<root>
        <hp:p><hp:run><hp:t>body para 1</hp:t></hp:run></hp:p>
        <hp:headerFooter>
            <hp:header>
                <hp:p><hp:run><hp:t>hdr</hp:t></hp:run></hp:p>
            </hp:header>
        </hp:headerFooter>
        <hp:p><hp:run><hp:t>body para 2</hp:t></hp:run></hp:p>
    </root>"#;
    let s = section(xml);

    // Two body blocks.
    assert_eq!(s.blocks.len(), 2, "two body paragraphs expected");

    // Header has exactly one block.
    let header = s.header.as_ref().expect("header must be Some");
    assert_eq!(header.len(), 1, "header must have exactly one block");
    match &header[0] {
        ir::Block::Paragraph { inlines } => {
            let text: String = inlines.iter().map(|i| i.text.as_str()).collect();
            assert_eq!(text, "hdr");
        }
        other => panic!("unexpected header block: {other:?}"),
    }
}

#[test]
fn header_footer_without_hp_prefix_also_parsed() {
    // The parser must accept bare element names (no namespace prefix).
    let xml = r#"<root>
        <headerFooter>
            <header>
                <p><run><t>bare header</t></run></p>
            </header>
        </headerFooter>
    </root>"#;
    let s = section(xml);

    let header = s
        .header
        .as_ref()
        .expect("header must be Some even without hp: prefix");
    assert!(!header.is_empty(), "header blocks must not be empty");
    match &header[0] {
        ir::Block::Paragraph { inlines } => {
            let text: String = inlines.iter().map(|i| i.text.as_str()).collect();
            assert_eq!(text, "bare header");
        }
        other => panic!("expected Paragraph in header (bare prefix), got {other:?}"),
    }
}

#[test]
fn image_in_header_stays_in_header() {
    let xml = r#"<root>
        <hp:p><hp:run><hp:t>body</hp:t></hp:run></hp:p>
        <hp:headerFooter>
            <hp:header>
                <hp:p><hp:run><hp:t>hdr text</hp:t></hp:run></hp:p>
                <hp:p>
                    <hp:run>
                        <hp:img binaryItemIDRef="logo.png" width="100" height="50"/>
                    </hp:run>
                </hp:p>
            </hp:header>
        </hp:headerFooter>
    </root>"#;
    let s = section(xml);

    assert_eq!(s.blocks.len(), 1, "body must have only one paragraph");
    let header = s.header.as_ref().expect("header must be Some");
    assert!(
        header.len() >= 2,
        "header must have at least 2 blocks (text + image), got {}",
        header.len()
    );
    let has_image = header.iter().any(|b| matches!(b, ir::Block::Image { .. }));
    assert!(
        has_image,
        "image block must stay in header, not leak to body"
    );
}

#[test]
fn page_break_in_footer_stays_in_footer() {
    let xml = r#"<root>
        <hp:headerFooter>
            <hp:footer>
                <hp:p><hp:run><hp:t>ftr text</hp:t></hp:run></hp:p>
                <hp:p><hp:run><hp:ctrl id="newPage"/></hp:run></hp:p>
            </hp:footer>
        </hp:headerFooter>
    </root>"#;
    let s = section(xml);

    assert!(s.blocks.is_empty(), "body must be empty");
    let footer = s.footer.as_ref().expect("footer must be Some");
    let has_pb = footer.iter().any(|b| matches!(b, ir::Block::PageBreak));
    assert!(has_pb, "page break must stay in footer, not leak to body");
}
