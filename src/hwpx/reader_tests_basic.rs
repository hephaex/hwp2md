use super::*;

// -----------------------------------------------------------------------
// Helper: unwrap the section and panic with a descriptive message on error.
// -----------------------------------------------------------------------

fn section(xml: &str) -> ir::Section {
    parse_section_xml(xml).expect("parse_section_xml must not fail")
}

// -----------------------------------------------------------------------
// parse_section_xml -- empty / minimal documents
// -----------------------------------------------------------------------

#[test]
fn empty_document_produces_no_blocks() {
    let s = section("<root/>");
    assert!(s.blocks.is_empty(), "expected no blocks for empty XML");
}

#[test]
fn empty_paragraph_produces_no_blocks() {
    // A paragraph element with no run content must be silently dropped.
    let xml = r#"<root><hp:p></hp:p></root>"#;
    let s = section(xml);
    assert!(
        s.blocks.is_empty(),
        "empty paragraph must not produce a block"
    );
}

// -----------------------------------------------------------------------
// parse_section_xml -- simple paragraph
// -----------------------------------------------------------------------

#[test]
fn simple_paragraph_text() {
    // Compact XML -- no whitespace text nodes between tags (matches real HWPX).
    let xml = r#"<root><hp:p><hp:run><hp:t>Hello World</hp:t></hp:run></hp:p></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            assert_eq!(inlines.len(), 1);
            assert_eq!(inlines[0].text, "Hello World");
            assert!(!inlines[0].bold);
            assert!(!inlines[0].italic);
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

#[test]
fn paragraph_without_hp_prefix() {
    // Bare element names (no namespace prefix) must also parse correctly.
    let xml = r#"<root><p><run><t>bare prefix</t></run></p></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            assert_eq!(inlines[0].text, "bare prefix");
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

#[test]
fn multiple_runs_in_one_paragraph_produce_multiple_inlines() {
    let xml = r#"<root><hp:p><hp:run><hp:t>first</hp:t></hp:run><hp:run><hp:t>second</hp:t></hp:run></hp:p></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            assert_eq!(inlines.len(), 2);
            assert_eq!(inlines[0].text, "first");
            assert_eq!(inlines[1].text, "second");
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

// -----------------------------------------------------------------------
// parse_section_xml -- heading via styleIDRef
// -----------------------------------------------------------------------

#[test]
fn heading_level2_via_style_id_ref() {
    let xml = r#"<root><hp:p styleIDRef="Heading2"><hp:run><hp:t>Chapter title</hp:t></hp:run></hp:p></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::Heading { level, inlines } => {
            assert_eq!(*level, 2);
            assert_eq!(inlines[0].text, "Chapter title");
        }
        other => panic!("expected Heading, got {other:?}"),
    }
}

#[test]
fn heading_level3_korean_style() {
    let xml =
        r#"<root><hp:p styleIDRef="제목3"><hp:run><hp:t>소제목</hp:t></hp:run></hp:p></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::Heading { level, inlines } => {
            assert_eq!(*level, 3);
            assert_eq!(inlines[0].text, "소제목");
        }
        other => panic!("expected Heading, got {other:?}"),
    }
}

// -----------------------------------------------------------------------
// parse_section_xml -- bold / italic via Start-element charPr
// -----------------------------------------------------------------------

#[test]
fn bold_text_via_charpr_start_element() {
    // Start-element charPr (non-self-closing) -- handled by handle_start_element.
    let xml = r#"<root><hp:p><hp:run><hp:charPr bold="true" italic="false"></hp:charPr><hp:t>bold text</hp:t></hp:run></hp:p></root>"#;
    let s = section(xml);
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            assert!(inlines[0].bold, "inline must be bold");
            assert!(!inlines[0].italic, "inline must not be italic");
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

#[test]
fn italic_text_via_charpr_empty_element() {
    // Self-closing charPr -- handled by handle_empty_element.
    let xml = r#"<root><hp:p><hp:run><hp:charPr bold="false" italic="true"/><hp:t>italic text</hp:t></hp:run></hp:p></root>"#;
    let s = section(xml);
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            assert!(!inlines[0].bold, "inline must not be bold");
            assert!(inlines[0].italic, "inline must be italic");
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

#[test]
fn bold_and_italic_via_empty_charpr() {
    let xml = r#"<root><hp:p><hp:run><hp:charPr bold="true" italic="true"/><hp:t>strong em</hp:t></hp:run></hp:p></root>"#;
    let s = section(xml);
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            assert!(inlines[0].bold);
            assert!(inlines[0].italic);
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

#[test]
fn charpr_numeric_1_means_bold() {
    // The parser accepts "1" as well as "true" for boolean attributes.
    let xml = r#"<root><hp:p><hp:run><hp:charPr bold="1"/><hp:t>numeric bold</hp:t></hp:run></hp:p></root>"#;
    let s = section(xml);
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            assert!(inlines[0].bold, "bold=\"1\" must be treated as true");
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

#[test]
fn charpr_resets_between_runs() {
    // Second run has no charPr, so bold must revert to false.
    // Compact XML avoids spurious whitespace-only text nodes.
    let xml = r#"<root><hp:p><hp:run><hp:charPr bold="true"/><hp:t>bold</hp:t></hp:run><hp:run><hp:t>plain</hp:t></hp:run></hp:p></root>"#;
    let s = section(xml);
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            assert_eq!(inlines.len(), 2);
            assert!(inlines[0].bold, "first inline must be bold");
            // The run end event resets bold to false.
            assert!(!inlines[1].bold, "second inline must not be bold");
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

#[test]
fn underline_via_empty_charpr() {
    let xml = r#"<root><hp:p><hp:run><hp:charPr underline="solid"/><hp:t>underlined</hp:t></hp:run></hp:p></root>"#;
    let s = section(xml);
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            assert!(inlines[0].underline, "inline must be underlined");
            assert!(!inlines[0].bold);
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

#[test]
fn strikeout_via_empty_charpr() {
    let xml = r#"<root><hp:p><hp:run><hp:charPr strikeout="true"/><hp:t>struck</hp:t></hp:run></hp:p></root>"#;
    let s = section(xml);
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            assert!(inlines[0].strikethrough, "inline must be strikethrough");
            assert!(!inlines[0].bold);
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

// -----------------------------------------------------------------------
// parse_section_xml -- lineBreak (empty element)
// -----------------------------------------------------------------------

#[test]
fn line_break_appends_newline_to_inline_text() {
    // lineBreak is an empty element that appends \n to current_text.
    // It lives outside a <t> so flush_paragraph picks it up at paragraph end.
    let xml = r#"<root><hp:p><hp:run><hp:t>line one</hp:t><hp:lineBreak/></hp:run></hp:p></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            let full: String = inlines.iter().map(|i| i.text.as_str()).collect();
            assert!(
                full.contains('\n'),
                "paragraph inlines must contain a newline from lineBreak; got: {full:?}"
            );
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

// -----------------------------------------------------------------------
// parse_section_xml -- image (empty element)
// -----------------------------------------------------------------------

#[test]
fn image_element_produces_image_block() {
    let xml = r#"<root>
        <hp:img src="image1.png" alt="photo"/>
    </root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::Image { src, alt } => {
            assert_eq!(src, "image1.png");
            assert_eq!(alt, "photo");
        }
        other => panic!("expected Image, got {other:?}"),
    }
}

#[test]
fn image_with_empty_src_is_ignored() {
    // An img element with no src attribute must not produce a block.
    let xml = r#"<root><hp:img alt="no src"/></root>"#;
    let s = section(xml);
    assert!(s.blocks.is_empty(), "img without src must be dropped");
}

// -----------------------------------------------------------------------
// parse_section_xml -- equation
// -----------------------------------------------------------------------

#[test]
fn equation_element_produces_math_block() {
    let xml = r#"<root><hp:equation>x^2 + y^2</hp:equation></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::Math { display, tex } => {
            assert!(*display, "equation must be display mode");
            assert_eq!(tex, "x^2 + y^2");
        }
        other => panic!("expected Math, got {other:?}"),
    }
}

#[test]
fn empty_equation_produces_no_block() {
    // An equation element with no text content must be silently dropped.
    let xml = r#"<root><hp:equation></hp:equation></root>"#;
    let s = section(xml);
    assert!(
        s.blocks.is_empty(),
        "empty equation must not produce a block"
    );
}

#[test]
fn eqedit_alias_also_produces_math_block() {
    // The parser accepts both <hp:equation> and <hp:eqEdit>.
    let xml = r#"<root><hp:eqEdit>a + b = c</hp:eqEdit></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::Math { tex, .. } => assert_eq!(tex, "a + b = c"),
        other => panic!("expected Math, got {other:?}"),
    }
}
