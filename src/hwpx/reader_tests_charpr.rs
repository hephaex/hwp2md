use super::*;

// -----------------------------------------------------------------------
// Helper: unwrap the section and panic with a descriptive message on error.
// -----------------------------------------------------------------------

fn section(xml: &str) -> ir::Section {
    parse_section_xml(xml).expect("parse_section_xml must not fail")
}

fn section_with_faces(xml: &str, faces: &[&str]) -> ir::Section {
    let face_names: Vec<String> = faces.iter().map(|s| s.to_string()).collect();
    parse_section_xml_with_face_names(xml, &face_names)
        .expect("parse_section_xml_with_face_names must not fail")
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

    let mut section = crate::ir::Section {
        blocks: Vec::new(),
        page_layout: None,
    };
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

    let mut section = crate::ir::Section {
        blocks: Vec::new(),
        page_layout: None,
    };
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

// -----------------------------------------------------------------------
// font_name resolution via faceNameIDRef + face_names table
// -----------------------------------------------------------------------

#[test]
fn font_name_resolved_from_face_names_table() {
    // charPr carries faceNameIDRef="1" which maps to the second face name.
    let xml = r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
                          xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
        <hp:p>
            <hp:run>
                <hp:charPr faceNameIDRef="1"/>
                <hp:t>폰트 테스트</hp:t>
            </hp:run>
        </hp:p>
    </hs:sec>"#;
    let faces = &["바탕", "맑은 고딕"];
    let sec = section_with_faces(xml, faces);
    let inlines = match &sec.blocks[0] {
        ir::Block::Paragraph { inlines } => inlines,
        other => panic!("expected Paragraph, got {other:?}"),
    };
    assert_eq!(
        inlines[0].font_name.as_deref(),
        Some("맑은 고딕"),
        "faceNameIDRef=1 must resolve to the second face name"
    );
}

#[test]
fn font_name_index_zero_resolves_to_first_face() {
    let xml = r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
                          xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
        <hp:p>
            <hp:run>
                <hp:charPr faceNameIDRef="0"/>
                <hp:t>첫 번째 폰트</hp:t>
            </hp:run>
        </hp:p>
    </hs:sec>"#;
    let faces = &["바탕", "맑은 고딕"];
    let sec = section_with_faces(xml, faces);
    let inlines = match &sec.blocks[0] {
        ir::Block::Paragraph { inlines } => inlines,
        other => panic!("expected Paragraph, got {other:?}"),
    };
    assert_eq!(
        inlines[0].font_name.as_deref(),
        Some("바탕"),
        "faceNameIDRef=0 must resolve to the first face name"
    );
}

#[test]
fn font_name_out_of_range_index_yields_none() {
    // Index 99 exceeds the table size — must not panic, font_name stays None.
    let xml = r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
                          xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
        <hp:p>
            <hp:run>
                <hp:charPr faceNameIDRef="99"/>
                <hp:t>폰트 없음</hp:t>
            </hp:run>
        </hp:p>
    </hs:sec>"#;
    let faces = &["바탕"];
    let sec = section_with_faces(xml, faces);
    let inlines = match &sec.blocks[0] {
        ir::Block::Paragraph { inlines } => inlines,
        other => panic!("expected Paragraph, got {other:?}"),
    };
    assert!(
        inlines[0].font_name.is_none(),
        "out-of-range faceNameIDRef must produce None font_name"
    );
}

#[test]
fn font_name_empty_face_table_yields_none() {
    // No face_names provided — font_name should remain None regardless of the
    // faceNameIDRef value.
    let xml = r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
                          xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
        <hp:p>
            <hp:run>
                <hp:charPr faceNameIDRef="0"/>
                <hp:t>폰트 테이블 없음</hp:t>
            </hp:run>
        </hp:p>
    </hs:sec>"#;
    let sec = section(xml);
    let inlines = match &sec.blocks[0] {
        ir::Block::Paragraph { inlines } => inlines,
        other => panic!("expected Paragraph, got {other:?}"),
    };
    assert!(
        inlines[0].font_name.is_none(),
        "empty face table must leave font_name as None"
    );
}

#[test]
fn font_name_reset_between_runs() {
    // First run carries a font; second run has no charPr — must not inherit.
    let xml = r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
                          xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
        <hp:p>
            <hp:run>
                <hp:charPr faceNameIDRef="0"/>
                <hp:t>첫 번째</hp:t>
            </hp:run>
            <hp:run>
                <hp:t>두 번째</hp:t>
            </hp:run>
        </hp:p>
    </hs:sec>"#;
    let faces = &["나눔고딕"];
    let sec = section_with_faces(xml, faces);
    let inlines = match &sec.blocks[0] {
        ir::Block::Paragraph { inlines } => inlines,
        other => panic!("expected Paragraph, got {other:?}"),
    };
    // First inline should have a font name; second must not inherit it.
    let first = inlines
        .iter()
        .find(|i| i.text == "첫 번째")
        .expect("first inline must exist");
    let second = inlines
        .iter()
        .find(|i| i.text == "두 번째")
        .expect("second inline must exist");
    assert_eq!(first.font_name.as_deref(), Some("나눔고딕"));
    assert!(
        second.font_name.is_none(),
        "font_name must not leak from one run into the next"
    );
}

#[test]
fn hangul_id_ref_attr_also_accepted() {
    // Some HWPX encodings use `hangulIDRef` instead of `faceNameIDRef`.
    let xml = r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
                          xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
        <hp:p>
            <hp:run>
                <hp:charPr hangulIDRef="0"/>
                <hp:t>한글IDRef</hp:t>
            </hp:run>
        </hp:p>
    </hs:sec>"#;
    let faces = &["돋움"];
    let sec = section_with_faces(xml, faces);
    let inlines = match &sec.blocks[0] {
        ir::Block::Paragraph { inlines } => inlines,
        other => panic!("expected Paragraph, got {other:?}"),
    };
    assert_eq!(
        inlines[0].font_name.as_deref(),
        Some("돋움"),
        "hangulIDRef must be treated identically to faceNameIDRef"
    );
}

// -----------------------------------------------------------------------
// parse_face_names — header.xml parser
// -----------------------------------------------------------------------

#[test]
fn parse_face_names_extracts_names_from_first_fontface() {
    let header_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<hh:head xmlns:hh="http://www.hancom.co.kr/hwpml/2011/head">
  <hh:refList>
    <hh:fontfaces itemCnt="2">
      <hh:fontface lang="HANGUL" fontCnt="3">
        <hh:font id="0" face="바탕" type="REP"/>
        <hh:font id="1" face="맑은 고딕" type="REP"/>
        <hh:font id="2" face="나눔고딕" type="REP"/>
      </hh:fontface>
      <hh:fontface lang="LATIN" fontCnt="2">
        <hh:font id="0" face="Arial" type="REP"/>
        <hh:font id="1" face="Times New Roman" type="REP"/>
      </hh:fontface>
    </hh:fontfaces>
  </hh:refList>
</hh:head>"#;
    // Indirectly test parse_face_names by calling parse_section_xml_with_face_names
    // after parsing the names from the header.
    let face_names = super::parse_face_names(header_xml);
    assert_eq!(face_names, vec!["바탕", "맑은 고딕", "나눔고딕"]);
}

#[test]
fn parse_face_names_empty_header_yields_empty_vec() {
    let header_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<head xmlns="http://www.hancom.co.kr/hwpml/2011/head">
  <title>제목</title>
</head>"#;
    let face_names = super::parse_face_names(header_xml);
    assert!(
        face_names.is_empty(),
        "header without fontface elements must produce empty vec"
    );
}

#[test]
fn parse_face_names_empty_string_yields_empty_vec() {
    let face_names = super::parse_face_names("");
    assert!(face_names.is_empty());
}
