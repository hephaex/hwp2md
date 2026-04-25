use super::*;

// -----------------------------------------------------------------------
// Helper: unwrap the section and panic with a descriptive message on error.
// -----------------------------------------------------------------------

fn section(xml: &str) -> ir::Section {
    parse_section_xml(xml).expect("parse_section_xml must not fail")
}

// -----------------------------------------------------------------------
// apply_charpr_attrs — color attribute parsing
// -----------------------------------------------------------------------

fn make_bytes_start_with_attrs(tag: &str, attrs: &[(&str, &str)]) -> Vec<u8> {
    let mut xml = format!("<{tag}");
    for (k, v) in attrs {
        xml.push_str(&format!(" {k}=\"{v}\""));
    }
    xml.push('>');
    xml.into_bytes()
}

fn apply_attrs_via_xml(tag: &str, attrs: &[(&str, &str)]) -> super::context::ParseContext {
    use quick_xml::events::BytesStart;
    let xml_bytes = make_bytes_start_with_attrs(tag, attrs);
    // Parse just the start tag bytes.
    let start_bytes = xml_bytes
        .iter()
        .take_while(|&&b| b != b'>')
        .cloned()
        .collect::<Vec<_>>();
    let e = BytesStart::from_content(
        std::str::from_utf8(&start_bytes[1..]).unwrap(), // strip leading '<'
        tag.len(),
    );
    let mut ctx = super::context::ParseContext::default();
    super::context::apply_charpr_attrs(&e, &mut ctx);
    ctx
}

#[test]
fn apply_charpr_attrs_color_sets_current_color() {
    let ctx = apply_attrs_via_xml("charPr", &[("color", "#FF0000")]);
    assert_eq!(ctx.current_color.as_deref(), Some("#FF0000"));
}

#[test]
fn apply_charpr_attrs_color_without_hash_normalises() {
    let ctx = apply_attrs_via_xml("charPr", &[("color", "FF0000")]);
    assert_eq!(ctx.current_color.as_deref(), Some("#FF0000"));
}

#[test]
fn apply_charpr_attrs_black_color_sets_none() {
    let ctx = apply_attrs_via_xml("charPr", &[("color", "#000000")]);
    assert!(
        ctx.current_color.is_none(),
        "black color must not be propagated"
    );
}

#[test]
fn apply_charpr_attrs_black_color_without_hash_sets_none() {
    let ctx = apply_attrs_via_xml("charPr", &[("color", "000000")]);
    assert!(ctx.current_color.is_none());
}

#[test]
fn apply_charpr_attrs_empty_color_sets_none() {
    let ctx = apply_attrs_via_xml("charPr", &[("color", "")]);
    assert!(ctx.current_color.is_none());
}

#[test]
fn apply_charpr_attrs_hp_color_prefix_accepted() {
    let ctx = apply_attrs_via_xml("hp:charPr", &[("hp:color", "0000FF")]);
    assert_eq!(ctx.current_color.as_deref(), Some("#0000FF"));
}

#[test]
fn apply_charpr_attrs_color_lowercase_normalised_to_upper() {
    let ctx = apply_attrs_via_xml("charPr", &[("color", "ff0000")]);
    assert_eq!(ctx.current_color.as_deref(), Some("#FF0000"));
}

// -----------------------------------------------------------------------
// flush_paragraph — color propagation into ir::Inline
// -----------------------------------------------------------------------

#[test]
fn flush_paragraph_propagates_color_to_inline() {
    let mut ctx = super::context::ParseContext {
        in_paragraph: true,
        current_text: "colored".to_string(),
        current_color: Some("#00FF00".to_string()),
        ..Default::default()
    };

    let mut section = crate::ir::Section { blocks: Vec::new() };
    super::context::flush_paragraph(&mut ctx, &mut section);

    assert_eq!(section.blocks.len(), 1);
    if let crate::ir::Block::Paragraph { inlines } = &section.blocks[0] {
        assert_eq!(inlines[0].color.as_deref(), Some("#00FF00"));
    } else {
        panic!("Expected Paragraph block");
    }
}

#[test]
fn flush_paragraph_no_color_propagates_none() {
    let mut ctx = super::context::ParseContext {
        in_paragraph: true,
        current_text: "plain".to_string(),
        current_color: None,
        ..Default::default()
    };

    let mut section = crate::ir::Section { blocks: Vec::new() };
    super::context::flush_paragraph(&mut ctx, &mut section);

    if let crate::ir::Block::Paragraph { inlines } = &section.blocks[0] {
        assert!(inlines[0].color.is_none());
    } else {
        panic!("Expected Paragraph block");
    }
}

// -----------------------------------------------------------------------
// Ruby annotation parsing via parse_section_xml
// -----------------------------------------------------------------------

#[test]
fn ruby_element_produces_inline_with_ruby_annotation() {
    let xml = r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
                          xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
        <hp:p><hp:run>
            <hp:ruby>
                <hp:rubyText>한자</hp:rubyText>
                <hp:baseText>漢字</hp:baseText>
            </hp:ruby>
        </hp:run></hp:p>
    </hs:sec>"#;
    let sec = section(xml);
    assert_eq!(sec.blocks.len(), 1, "one paragraph block expected");
    if let ir::Block::Paragraph { inlines } = &sec.blocks[0] {
        let ruby_inline = inlines
            .iter()
            .find(|i| i.ruby.is_some())
            .expect("must have at least one inline with ruby annotation");
        assert_eq!(ruby_inline.text, "漢字");
        assert_eq!(ruby_inline.ruby.as_deref(), Some("한자"));
    } else {
        panic!("expected Paragraph block, got {:?}", sec.blocks[0]);
    }
}

#[test]
fn ruby_element_without_hp_prefix_also_parsed() {
    let xml = r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
                          xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
        <hp:p><hp:run>
            <ruby>
                <rubyText>annotation</rubyText>
                <baseText>base</baseText>
            </ruby>
        </hp:run></hp:p>
    </hs:sec>"#;
    let sec = section(xml);
    assert_eq!(sec.blocks.len(), 1);
    if let ir::Block::Paragraph { inlines } = &sec.blocks[0] {
        let ruby_inline = inlines
            .iter()
            .find(|i| i.ruby.is_some())
            .expect("must have ruby inline");
        assert_eq!(ruby_inline.text, "base");
        assert_eq!(ruby_inline.ruby.as_deref(), Some("annotation"));
    } else {
        panic!("expected Paragraph");
    }
}

#[test]
fn ruby_element_empty_annotation_no_ruby_field() {
    let xml = r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
                          xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
        <hp:p><hp:run>
            <hp:ruby>
                <hp:rubyText></hp:rubyText>
                <hp:baseText>漢字</hp:baseText>
            </hp:ruby>
        </hp:run></hp:p>
    </hs:sec>"#;
    let sec = section(xml);
    if let ir::Block::Paragraph { inlines } = &sec.blocks[0] {
        let inline = inlines
            .iter()
            .find(|i| i.text == "漢字")
            .expect("base text inline must exist");
        assert!(
            inline.ruby.is_none(),
            "empty rubyText must produce None annotation; got {:?}",
            inline.ruby
        );
    }
}

#[test]
fn ruby_inline_renders_to_html_ruby_tags_in_markdown() {
    use crate::md::write_markdown;

    let xml = r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
                          xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
        <hp:p><hp:run>
            <hp:ruby>
                <hp:rubyText>한자</hp:rubyText>
                <hp:baseText>漢字</hp:baseText>
            </hp:ruby>
        </hp:run></hp:p>
    </hs:sec>"#;
    let sec = section(xml);
    let mut doc = crate::ir::Document::new();
    doc.sections.push(sec);
    let md = write_markdown(&doc, false);
    assert!(
        md.contains("<ruby>漢字<rt>한자</rt></ruby>"),
        "markdown output must contain HTML ruby tags; got: {md}"
    );
}
