use super::*;

// -----------------------------------------------------------------------
// Helper: unwrap the section and panic with a descriptive message on error.
// -----------------------------------------------------------------------

fn section(xml: &str) -> ir::Section {
    parse_section_xml(xml).expect("parse_section_xml must not fail")
}

fn section_with_faces(xml: &str, faces: &[&str]) -> ir::Section {
    let face_names: Vec<String> = faces.iter().map(|s| (*s).to_string()).collect();
    parse_section_xml_with_face_names(xml, &face_names)
        .expect("parse_section_xml_with_face_names must not fail")
}

// -----------------------------------------------------------------------
// apply_charpr_attrs — color attribute parsing
// -----------------------------------------------------------------------

fn make_bytes_start_with_attrs(tag: &str, attrs: &[(&str, &str)]) -> Vec<u8> {
    use std::fmt::Write as _;
    let mut xml = format!("<{tag}");
    for (k, v) in attrs {
        let _ = write!(xml, " {k}=\"{v}\"");
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
        .copied()
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
    assert_eq!(ctx.fmt.color.as_deref(), Some("#FF0000"));
}

#[test]
fn apply_charpr_attrs_color_without_hash_normalises() {
    let ctx = apply_attrs_via_xml("charPr", &[("color", "FF0000")]);
    assert_eq!(ctx.fmt.color.as_deref(), Some("#FF0000"));
}

#[test]
fn apply_charpr_attrs_black_color_sets_none() {
    let ctx = apply_attrs_via_xml("charPr", &[("color", "#000000")]);
    assert!(
        ctx.fmt.color.is_none(),
        "black color must not be propagated"
    );
}

#[test]
fn apply_charpr_attrs_black_color_without_hash_sets_none() {
    let ctx = apply_attrs_via_xml("charPr", &[("color", "000000")]);
    assert!(ctx.fmt.color.is_none());
}

#[test]
fn apply_charpr_attrs_empty_color_sets_none() {
    let ctx = apply_attrs_via_xml("charPr", &[("color", "")]);
    assert!(ctx.fmt.color.is_none());
}

#[test]
fn apply_charpr_attrs_hp_color_prefix_accepted() {
    let ctx = apply_attrs_via_xml("hp:charPr", &[("hp:color", "0000FF")]);
    assert_eq!(ctx.fmt.color.as_deref(), Some("#0000FF"));
}

#[test]
fn apply_charpr_attrs_color_lowercase_normalised_to_upper() {
    let ctx = apply_attrs_via_xml("charPr", &[("color", "ff0000")]);
    assert_eq!(ctx.fmt.color.as_deref(), Some("#FF0000"));
}

// -----------------------------------------------------------------------
// flush_paragraph — color propagation into ir::Inline
// -----------------------------------------------------------------------

#[test]
fn flush_paragraph_propagates_color_to_inline() {
    let mut ctx = super::context::ParseContext {
        in_paragraph: true,
        current_text: "colored".to_string(),
        fmt: super::context::FormattingState {
            color: Some("#00FF00".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    let mut section = crate::ir::Section {
        blocks: Vec::new(),
        page_layout: None,
        ..Default::default()
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
        ..Default::default()
    };

    let mut section = crate::ir::Section {
        blocks: Vec::new(),
        page_layout: None,
        ..Default::default()
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

// -----------------------------------------------------------------------
// apply_charpr_attrs — parse_bool_preserve (bold / italic)
// -----------------------------------------------------------------------

#[test]
fn apply_charpr_attrs_bold_garbage_value_preserves_existing() {
    use quick_xml::events::BytesStart;

    // Build a BytesStart with bold="yes" (an unrecognised value).
    let xml_bytes = make_bytes_start_with_attrs("charPr", &[("bold", "yes")]);
    let start_bytes = xml_bytes
        .iter()
        .take_while(|&&b| b != b'>')
        .copied()
        .collect::<Vec<_>>();
    let e = BytesStart::from_content(
        std::str::from_utf8(&start_bytes[1..]).unwrap(),
        "charPr".len(),
    );

    // Pre-set bold = true so we can confirm it is preserved.
    let mut ctx = super::context::ParseContext {
        fmt: super::context::FormattingState {
            bold: true,
            ..Default::default()
        },
        ..Default::default()
    };
    super::context::apply_charpr_attrs(&e, &mut ctx);

    assert!(
        ctx.fmt.bold,
        "garbage bold value must preserve existing true"
    );
}

#[test]
fn apply_charpr_attrs_italic_garbage_value_preserves_existing() {
    use quick_xml::events::BytesStart;

    let xml_bytes = make_bytes_start_with_attrs("charPr", &[("italic", "yes")]);
    let start_bytes = xml_bytes
        .iter()
        .take_while(|&&b| b != b'>')
        .copied()
        .collect::<Vec<_>>();
    let e = BytesStart::from_content(
        std::str::from_utf8(&start_bytes[1..]).unwrap(),
        "charPr".len(),
    );

    let mut ctx = super::context::ParseContext {
        fmt: super::context::FormattingState {
            italic: true,
            ..Default::default()
        },
        ..Default::default()
    };
    super::context::apply_charpr_attrs(&e, &mut ctx);

    assert!(
        ctx.fmt.italic,
        "garbage italic value must preserve existing true"
    );
}

#[test]
fn apply_charpr_attrs_bold_numeric_one_sets_true() {
    // bold starts false; "1" must flip it to true.
    let ctx = apply_attrs_via_xml("charPr", &[("bold", "1")]);
    assert!(
        ctx.fmt.bold,
        "bold=\"1\" must set bold to true"
    );
}

// -----------------------------------------------------------------------
// flush_paragraph_staged — list-staging gate
// -----------------------------------------------------------------------

#[test]
fn flush_paragraph_staged_code_block_with_list_para_pr_id_is_plain() {
    // Regression guard for build_block extraction (Sprint 72): a CodeBlock must never be
    // wrapped in StagedBlock::ListPara even when paraPrIDRef="2" would normally trigger
    // list-staging for a Paragraph.
    let mut ctx = super::context::ParseContext {
        in_paragraph: true,
        current_text: "fn main() {}".to_string(),
        pending_code_lang: Some(None), // outer Some = code block; inner None = no language
        current_para_pr_id: Some("2".to_string()), // would trigger depth-0 list for a Paragraph
        ..Default::default()
    };

    let result = super::context::flush_paragraph_staged(&mut ctx);

    assert!(result.is_some(), "flush_paragraph_staged must return Some for non-empty inlines");
    match result.unwrap() {
        super::context::StagedBlock::Plain(crate::ir::Block::CodeBlock { code, .. }) => {
            assert_eq!(code, "fn main() {}", "code block text mismatch: {:?}", code);
        }
        other => panic!(
            "expected StagedBlock::Plain(CodeBlock), got {:?}; \
             CodeBlock must never be list-staged regardless of paraPrIDRef",
            other
        ),
    }
}

// -----------------------------------------------------------------------
// apply_charpr_attrs — height attribute parsing (tier-3 heading support)
// -----------------------------------------------------------------------

#[test]
fn apply_charpr_attrs_height_sets_font_height() {
    let ctx = apply_attrs_via_xml("charPr", &[("height", "1600")]);
    assert_eq!(
        ctx.fmt.font_height,
        Some(1600),
        "height=\"1600\" must set font_height to Some(1600)"
    );
}

#[test]
fn apply_charpr_attrs_hp_height_prefix_accepted() {
    let ctx = apply_attrs_via_xml("hp:charPr", &[("hp:height", "1400")]);
    assert_eq!(
        ctx.fmt.font_height,
        Some(1400),
        "hp:height must be accepted identically to height"
    );
}

#[test]
fn apply_charpr_attrs_height_invalid_value_leaves_none() {
    let ctx = apply_attrs_via_xml("charPr", &[("height", "auto")]);
    assert!(
        ctx.fmt.font_height.is_none(),
        "invalid height value must not set font_height"
    );
}

#[test]
fn apply_charpr_attrs_height_updates_para_max_when_greater() {
    use quick_xml::events::BytesStart;

    // Pre-seed the tracker with a smaller value (1200), then apply height=1600.
    let xml_bytes = make_bytes_start_with_attrs("charPr", &[("height", "1600"), ("bold", "true")]);
    let start_bytes: Vec<u8> = xml_bytes
        .iter()
        .take_while(|&&b| b != b'>')
        .copied()
        .collect();
    let e = BytesStart::from_content(
        std::str::from_utf8(&start_bytes[1..]).unwrap(),
        "charPr".len(),
    );

    let mut ctx = super::context::ParseContext {
        para_max_font_height: 1200,
        para_max_font_height_bold: false,
        ..Default::default()
    };
    super::context::apply_charpr_attrs(&e, &mut ctx);

    assert_eq!(ctx.para_max_font_height, 1600, "tracker must update to the larger height");
    assert!(ctx.para_max_font_height_bold, "bold flag must reflect the run that set the max");
}

#[test]
fn apply_charpr_attrs_height_does_not_decrease_para_max() {
    use quick_xml::events::BytesStart;

    // Pre-seed with a large value (2000), then apply a smaller height (1400).
    let xml_bytes = make_bytes_start_with_attrs("charPr", &[("height", "1400"), ("bold", "true")]);
    let start_bytes: Vec<u8> = xml_bytes
        .iter()
        .take_while(|&&b| b != b'>')
        .copied()
        .collect();
    let e = BytesStart::from_content(
        std::str::from_utf8(&start_bytes[1..]).unwrap(),
        "charPr".len(),
    );

    let mut ctx = super::context::ParseContext {
        para_max_font_height: 2000,
        para_max_font_height_bold: true,
        ..Default::default()
    };
    super::context::apply_charpr_attrs(&e, &mut ctx);

    assert_eq!(ctx.para_max_font_height, 2000, "tracker must not decrease for a smaller height");
}

// -----------------------------------------------------------------------
// Tier-3 heading detection via flush_paragraph (integration path)
// -----------------------------------------------------------------------

/// Build a section that has a single paragraph whose charPr carries the given
/// height and bold attributes.  Returns the parsed `ir::Section`.
fn tier3_section(text: &str, height: u32, bold: bool) -> ir::Section {
    let bold_str = if bold { "true" } else { "false" };
    let xml = format!(
        r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
                         xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
        <hp:p>
            <hp:run>
                <hp:charPr height="{height}" bold="{bold_str}"/>
                <hp:t>{text}</hp:t>
            </hp:run>
        </hp:p>
    </hs:sec>"#,
    );
    section(&xml)
}

#[test]
fn tier3_bold_16pt_produces_h1() {
    // 1600 (16 pt) + bold, short text → H1
    let sec = tier3_section("제1장 총칙", 1600, true);
    assert_eq!(sec.blocks.len(), 1);
    match &sec.blocks[0] {
        ir::Block::Heading { level, .. } => {
            assert_eq!(*level, 1, "1600pt bold must produce H1; got level {level}");
        }
        other => panic!("expected Heading block, got {other:?}"),
    }
}

#[test]
fn tier3_bold_14pt_produces_h2() {
    // 1400 (14 pt) + bold, short text → H2
    let sec = tier3_section("제1절 적용범위", 1400, true);
    assert_eq!(sec.blocks.len(), 1);
    match &sec.blocks[0] {
        ir::Block::Heading { level, .. } => {
            assert_eq!(*level, 2, "1400pt bold must produce H2; got level {level}");
        }
        other => panic!("expected Heading block, got {other:?}"),
    }
}

#[test]
fn tier3_bold_12pt_produces_h3() {
    // 1200 (12 pt) + bold, short text → H3
    let sec = tier3_section("제1조 목적", 1200, true);
    assert_eq!(sec.blocks.len(), 1);
    match &sec.blocks[0] {
        ir::Block::Heading { level, .. } => {
            assert_eq!(*level, 3, "1200pt bold must produce H3; got level {level}");
        }
        other => panic!("expected Heading block, got {other:?}"),
    }
}

#[test]
fn tier3_not_bold_16pt_no_heading() {
    // 1600 (16 pt) + NOT bold → must not promote to heading
    let sec = tier3_section("큰 글자지만 굵지 않음", 1600, false);
    assert_eq!(sec.blocks.len(), 1);
    match &sec.blocks[0] {
        ir::Block::Paragraph { .. } => {} // correct: no promotion
        other => panic!("expected Paragraph (not bold must not trigger tier-3), got {other:?}"),
    }
}

#[test]
fn tier3_bold_below_12pt_no_heading() {
    // 1199 (11.99 pt) + bold → below threshold, stays Paragraph
    let sec = tier3_section("작은 굵은 글자", 1199, true);
    assert_eq!(sec.blocks.len(), 1);
    match &sec.blocks[0] {
        ir::Block::Paragraph { .. } => {} // correct: below HWPX_H3_MIN_HEIGHT
        other => panic!("expected Paragraph (height below threshold), got {other:?}"),
    }
}

#[test]
fn tier3_100char_guard_prevents_heading() {
    // 1600 pt + bold but text >= 100 characters → body-text guard kicks in
    let long_text = "가".repeat(100); // exactly 100 characters
    let sec = tier3_section(&long_text, 1600, true);
    assert_eq!(sec.blocks.len(), 1);
    match &sec.blocks[0] {
        ir::Block::Paragraph { .. } => {} // correct: 100-char guard
        other => panic!(
            "expected Paragraph (100-char guard must suppress tier-3 heading), got {other:?}"
        ),
    }
}

#[test]
fn tier3_99char_boundary_promotes_to_heading() {
    // 1600 pt + bold + exactly 99 characters → guard is `>= 100`, so 99 must still promote
    let text_99 = "가".repeat(99);
    let sec = tier3_section(&text_99, 1600, true);
    assert_eq!(sec.blocks.len(), 1);
    match &sec.blocks[0] {
        ir::Block::Heading { level, .. } => {
            assert_eq!(
                *level, 1,
                "99-char bold 1600pt must promote to H1 (guard fires at >= 100); got level {level}"
            );
        }
        other => panic!("expected Heading block for 99-char text, got {other:?}"),
    }
}

#[test]
fn tier3_style_level_takes_priority_over_height() {
    // styleIDRef="2" (tier-1/2) + 1600pt bold → must use tier-1 level H2, not H1
    let xml = r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
                         xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
        <hp:p styleIDRef="2">
            <hp:run>
                <hp:charPr height="1600" bold="true"/>
                <hp:t>스타일 우선 테스트</hp:t>
            </hp:run>
        </hp:p>
    </hs:sec>"#;
    let sec = section(xml);
    assert_eq!(sec.blocks.len(), 1);
    match &sec.blocks[0] {
        ir::Block::Heading { level, .. } => {
            assert_eq!(
                *level, 2,
                "styleIDRef=2 must win over tier-3 height (1600pt would produce H1); got level {level}"
            );
        }
        other => panic!("expected Heading block, got {other:?}"),
    }
}

// -----------------------------------------------------------------------
// flush_nested_scope — pending_code_lang clearing
// -----------------------------------------------------------------------

#[test]
fn pending_code_lang_cleared_after_nested_scope_flush() {
    use super::context::flush_nested_scope;

    // Helper macro: verify flush_nested_scope returns true and clears pending_code_lang.
    // The $label parameter appears in failure messages so the failing branch is identifiable.
    macro_rules! check_scope {
        ($ctx:ident, $label:expr) => {{
            $ctx.pending_code_lang = Some(Some("python".to_string()));
            let result = flush_nested_scope(&mut $ctx);
            assert!(result, "[{}] flush_nested_scope must return true in nested scope", $label);
            assert!(
                $ctx.pending_code_lang.is_none(),
                "[{}] pending_code_lang must be cleared after nested scope flush", $label
            );
        }};
    }

    // in_header
    {
        let mut ctx = super::context::ParseContext::default();
        ctx.header_footer.in_header = true;
        check_scope!(ctx, "in_header");
    }

    // in_footer
    {
        let mut ctx = super::context::ParseContext::default();
        ctx.header_footer.in_footer = true;
        check_scope!(ctx, "in_footer");
    }

    // footnote.active
    {
        let mut ctx = super::context::ParseContext::default();
        ctx.footnote.active = true;
        check_scope!(ctx, "footnote.active");
    }

    // table.in_cell
    {
        let mut ctx = super::context::ParseContext::default();
        ctx.table.in_cell = true;
        check_scope!(ctx, "table.in_cell");
    }

    // list.in_item
    {
        let mut ctx = super::context::ParseContext::default();
        ctx.list.in_item = true;
        check_scope!(ctx, "list.in_item");
    }
}

#[test]
fn pending_code_lang_preserved_at_top_level_no_nested_scope() {
    use super::context::flush_nested_scope;

    let mut ctx = super::context::ParseContext {
        pending_code_lang: Some(Some("rust".to_string())),
        ..Default::default()
    };

    // No nested scope active (all false by default).
    let result = flush_nested_scope(&mut ctx);

    assert!(
        !result,
        "flush_nested_scope must return false at top level"
    );
    assert_eq!(
        ctx.pending_code_lang,
        Some(Some("rust".to_string())),
        "pending_code_lang must remain intact when no nested scope is active"
    );
}
