use super::*;

// ── Phase 10 tests: ruby annotation writer ──────────────────────────────

#[test]
fn section_xml_ruby_inline_produces_ruby_structure() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![Inline {
            text: "\u{6F22}\u{5B57}".into(),       // 漢字
            ruby: Some("\u{D55C}\u{C790}".into()), // 한자
            ..Inline::default()
        }],
    }]);

    assert!(
        xml.contains("<hp:ruby>"),
        "ruby element must be present: {xml}"
    );
    assert!(
        xml.contains("</hp:ruby>"),
        "ruby closing tag must be present: {xml}"
    );
    assert!(
        xml.contains("<hp:baseText>"),
        "baseText element must be present: {xml}"
    );
    assert!(
        xml.contains("<hp:rubyText>"),
        "rubyText element must be present: {xml}"
    );
    assert!(
        xml.contains("\u{6F22}\u{5B57}"),
        "base text content must be present: {xml}"
    );
    assert!(
        xml.contains("\u{D55C}\u{C790}"),
        "ruby annotation text must be present: {xml}"
    );
}

#[test]
fn section_xml_ruby_nesting_order() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![Inline {
            text: "base".into(),
            ruby: Some("anno".into()),
            ..Inline::default()
        }],
    }]);

    // Verify structural nesting: run > ruby > baseText > t > text
    let run_pos = xml.find("<hp:run").expect("run element");
    let ruby_pos = xml.find("<hp:ruby>").expect("ruby element");
    let base_pos = xml.find("<hp:baseText>").expect("baseText element");
    let ruby_text_pos = xml.find("<hp:rubyText>").expect("rubyText element");

    assert!(run_pos < ruby_pos, "run must precede ruby: {xml}");
    assert!(ruby_pos < base_pos, "ruby must precede baseText: {xml}");
    assert!(
        base_pos < ruby_text_pos,
        "baseText must precede rubyText: {xml}"
    );
}

#[test]
fn section_xml_non_ruby_inline_has_no_ruby_element() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![inline("plain text")],
    }]);

    assert!(
        !xml.contains("<hp:ruby>"),
        "non-ruby inline must not produce ruby element: {xml}"
    );
    assert!(
        !xml.contains("<hp:baseText>"),
        "non-ruby inline must not produce baseText: {xml}"
    );
    assert!(
        !xml.contains("<hp:rubyText>"),
        "non-ruby inline must not produce rubyText: {xml}"
    );
}

#[test]
fn section_xml_ruby_with_formatting_has_charpr_id_ref() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![Inline {
            text: "base".into(),
            ruby: Some("anno".into()),
            bold: true,
            ..Inline::default()
        }],
    }]);

    // Bold ruby should get a non-zero charPrIDRef.
    assert!(xml.contains("charPrIDRef="), "charPrIDRef: {xml}");
    assert!(xml.contains("<hp:ruby>"), "ruby element: {xml}");
}

#[test]
fn section_xml_mixed_ruby_and_plain_inlines() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![
            inline("before "),
            Inline {
                text: "base".into(),
                ruby: Some("anno".into()),
                ..Inline::default()
            },
            inline(" after"),
        ],
    }]);

    assert!(xml.contains("before "), "prefix text: {xml}");
    assert!(xml.contains(" after"), "suffix text: {xml}");
    assert!(xml.contains("<hp:ruby>"), "ruby element: {xml}");

    // "before" must come before <hp:ruby>, "after" must come after </hp:ruby>.
    let before_pos = xml.find("before ").unwrap();
    let ruby_pos = xml.find("<hp:ruby>").unwrap();
    let ruby_end_pos = xml.find("</hp:ruby>").unwrap();
    let after_pos = xml.find(" after").unwrap();
    assert!(before_pos < ruby_pos, "prefix before ruby");
    assert!(ruby_end_pos < after_pos, "ruby end before suffix");
}

// ── Phase 10 tests: ruby reader formatting propagation ──────────────────

#[test]
fn reader_ruby_with_bold_preserves_formatting() {
    // Note: ruby text is accumulated verbatim (including whitespace between
    // XML tags), so the XML must be compact with no extra whitespace inside
    // <hp:baseText> / <hp:rubyText> wrapper elements.
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
        xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
  <hp:p id="0" paraPrIDRef="0">
    <hp:run charPrIDRef="1">
      <hp:charPr bold="true"/>
      <hp:ruby><hp:baseText><hp:t>base</hp:t></hp:baseText><hp:rubyText><hp:t>anno</hp:t></hp:rubyText></hp:ruby>
    </hp:run>
  </hp:p>
</hs:sec>"#;

    let section = read_hwpx_section_xml(xml);
    let inlines = match &section.blocks[0] {
        Block::Paragraph { inlines } => inlines,
        other => panic!("expected Paragraph, got: {other:?}"),
    };

    assert!(!inlines.is_empty(), "inlines should not be empty");
    let ruby_inline = &inlines[0];
    assert_eq!(ruby_inline.text, "base", "base text");
    assert_eq!(ruby_inline.ruby.as_deref(), Some("anno"), "annotation text");
    assert!(
        ruby_inline.bold,
        "bold formatting must be preserved on ruby inline"
    );
}

#[test]
fn reader_ruby_with_italic_and_color_preserves_formatting() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
        xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
  <hp:p id="0" paraPrIDRef="0">
    <hp:run charPrIDRef="1">
      <hp:charPr italic="true" color="FF0000"/>
      <hp:ruby><hp:baseText><hp:t>text</hp:t></hp:baseText><hp:rubyText><hp:t>reading</hp:t></hp:rubyText></hp:ruby>
    </hp:run>
  </hp:p>
</hs:sec>"#;

    let section = read_hwpx_section_xml(xml);
    let inlines = match &section.blocks[0] {
        Block::Paragraph { inlines } => inlines,
        other => panic!("expected Paragraph, got: {other:?}"),
    };

    let ruby_inline = &inlines[0];
    assert!(
        ruby_inline.italic,
        "italic must be preserved on ruby inline"
    );
    assert_eq!(
        ruby_inline.color.as_deref(),
        Some("#FF0000"),
        "color must be preserved on ruby inline"
    );
}

// ── Phase 10 tests: Inline::with_formatting constructor ─────────────────

#[test]
fn inline_with_formatting_sets_all_fields() {
    let i = Inline::with_formatting(
        "hello".into(),
        true,
        false,
        true,
        false,
        true,
        false,
        Some("#FF0000".into()),
    );
    assert_eq!(i.text, "hello");
    assert!(i.bold);
    assert!(!i.italic);
    assert!(i.underline);
    assert!(!i.strikethrough);
    assert!(i.superscript);
    assert!(!i.subscript);
    assert_eq!(i.color.as_deref(), Some("#FF0000"));
    // Unset fields must be at their defaults.
    assert!(!i.code);
    assert!(i.link.is_none());
    assert!(i.footnote_ref.is_none());
    assert!(i.font_name.is_none());
    assert!(i.ruby.is_none());
}

#[test]
fn inline_with_formatting_chained_with_link() {
    let i = Inline::with_formatting(
        "click".into(),
        false,
        false,
        false,
        false,
        false,
        false,
        None,
    )
    .with_link(Some("https://example.com".into()));

    assert_eq!(i.text, "click");
    assert_eq!(i.link.as_deref(), Some("https://example.com"));
}

#[test]
fn inline_with_formatting_chained_with_ruby() {
    let i = Inline::with_formatting("base".into(), true, false, false, false, false, false, None)
        .with_ruby(Some("annotation".into()));

    assert_eq!(i.text, "base");
    assert!(i.bold);
    assert_eq!(i.ruby.as_deref(), Some("annotation"));
}

// ── Phase 12: ruby + link combo test ────────────────────────────────────

#[test]
fn section_xml_ruby_with_link_produces_field_wrapping_around_ruby() {
    // An Inline with both ruby AND link set must produce fieldBegin/fieldEnd
    // wrapping that encloses the ruby structure.
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![Inline {
            text: "base".into(),
            ruby: Some("annotation".into()),
            link: Some("https://example.com".into()),
            ..Inline::default()
        }],
    }]);

    // Both field markers must be present.
    assert!(
        xml.contains("hp:fieldBegin"),
        "fieldBegin must be present for ruby+link inline: {xml}"
    );
    assert!(
        xml.contains("hp:fieldEnd"),
        "fieldEnd must be present for ruby+link inline: {xml}"
    );
    assert!(
        xml.contains(r#"type="HYPERLINK""#),
        "field type must be HYPERLINK: {xml}"
    );
    assert!(
        xml.contains(r#"command="https://example.com""#),
        "field command must contain URL: {xml}"
    );

    // The ruby structure must also be present.
    assert!(
        xml.contains("<hp:ruby>"),
        "ruby element must be present: {xml}"
    );
    assert!(
        xml.contains("<hp:baseText>"),
        "baseText must be present: {xml}"
    );
    assert!(
        xml.contains("<hp:rubyText>"),
        "rubyText must be present: {xml}"
    );
    assert!(xml.contains("base"), "base text must be present: {xml}");
    assert!(
        xml.contains("annotation"),
        "annotation text must be present: {xml}"
    );

    // The fieldBegin must precede the ruby, and fieldEnd must follow it.
    let begin_pos = xml.find("hp:fieldBegin").expect("fieldBegin position");
    let ruby_pos = xml.find("<hp:ruby>").expect("ruby position");
    let ruby_end_pos = xml.find("</hp:ruby>").expect("ruby end position");
    let end_pos = xml.find("hp:fieldEnd").expect("fieldEnd position");

    assert!(
        begin_pos < ruby_pos,
        "fieldBegin must precede ruby element: {xml}"
    );
    assert!(
        ruby_end_pos < end_pos,
        "ruby end must precede fieldEnd: {xml}"
    );
}
