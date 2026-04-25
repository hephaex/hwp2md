use super::*;

// ── Phase 9 tests: hyperlink writer ───────────────────────────────────────

#[test]
fn section_xml_link_inline_produces_field_begin_end() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![Inline {
            text: "click here".into(),
            link: Some("https://example.com".into()),
            ..Inline::default()
        }],
    }]);

    assert!(
        xml.contains("hp:fieldBegin"),
        "fieldBegin must be present: {xml}"
    );
    assert!(
        xml.contains("hp:fieldEnd"),
        "fieldEnd must be present: {xml}"
    );
    assert!(
        xml.contains(r#"type="HYPERLINK""#),
        "field type must be HYPERLINK: {xml}"
    );
    assert!(
        xml.contains(r#"command="https://example.com""#),
        "field command must contain the URL: {xml}"
    );
    assert!(
        xml.contains("click here"),
        "link text must be present: {xml}"
    );
}

#[test]
fn section_xml_field_begin_has_correct_attributes() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![Inline {
            text: "link".into(),
            link: Some("https://rust-lang.org".into()),
            ..Inline::default()
        }],
    }]);

    // fieldBegin must have both type and command attributes.
    let fb_pos = xml.find("hp:fieldBegin").expect("fieldBegin must exist");
    let fb_end = xml[fb_pos..].find("/>").expect("fieldBegin must be self-closing");
    let fb_tag = &xml[fb_pos..fb_pos + fb_end];
    assert!(
        fb_tag.contains(r#"type="HYPERLINK""#),
        "fieldBegin type attr: {fb_tag}"
    );
    assert!(
        fb_tag.contains(r#"command="https://rust-lang.org""#),
        "fieldBegin command attr: {fb_tag}"
    );
}

#[test]
fn section_xml_link_text_appears_between_field_markers() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![Inline {
            text: "Visit".into(),
            link: Some("https://example.com".into()),
            ..Inline::default()
        }],
    }]);

    let begin_pos = xml.find("hp:fieldBegin").expect("fieldBegin");
    let text_pos = xml.find("Visit").expect("link text");
    let end_pos = xml.find("hp:fieldEnd").expect("fieldEnd");

    assert!(
        begin_pos < text_pos,
        "fieldBegin must precede link text: {xml}"
    );
    assert!(
        text_pos < end_pos,
        "link text must precede fieldEnd: {xml}"
    );
}

#[test]
fn section_xml_non_link_inlines_remain_unchanged() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![inline("no link here")],
    }]);

    assert!(
        !xml.contains("hp:fieldBegin"),
        "non-link inlines must not produce fieldBegin: {xml}"
    );
    assert!(
        !xml.contains("hp:fieldEnd"),
        "non-link inlines must not produce fieldEnd: {xml}"
    );
    assert!(xml.contains("no link here"), "text must be present: {xml}");
}

#[test]
fn section_xml_mixed_link_and_non_link_inlines() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![
            inline("before "),
            Inline {
                text: "link".into(),
                link: Some("https://example.com".into()),
                ..Inline::default()
            },
            inline(" after"),
        ],
    }]);

    assert!(xml.contains("before "), "prefix text: {xml}");
    assert!(xml.contains("link"), "link text: {xml}");
    assert!(xml.contains(" after"), "suffix text: {xml}");
    assert!(
        xml.contains("hp:fieldBegin"),
        "fieldBegin for link inline: {xml}"
    );
    assert!(
        xml.contains("hp:fieldEnd"),
        "fieldEnd for link inline: {xml}"
    );

    // "before" must come before fieldBegin, "after" must come after fieldEnd.
    let before_pos = xml.find("before ").unwrap();
    let begin_pos = xml.find("hp:fieldBegin").unwrap();
    let end_pos = xml.find("hp:fieldEnd").unwrap();
    let after_pos = xml.find(" after").unwrap();
    assert!(before_pos < begin_pos, "prefix before fieldBegin");
    assert!(end_pos < after_pos, "fieldEnd before suffix");
}

#[test]
fn section_xml_consecutive_link_inlines_same_url_grouped() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![
            Inline {
                text: "part1".into(),
                link: Some("https://example.com".into()),
                ..Inline::default()
            },
            Inline {
                text: "part2".into(),
                link: Some("https://example.com".into()),
                ..Inline::default()
            },
        ],
    }]);

    // Two consecutive inlines with the same URL should be grouped into a
    // single fieldBegin/fieldEnd pair.
    let begin_count = xml.matches("hp:fieldBegin").count();
    let end_count = xml.matches("hp:fieldEnd").count();
    assert_eq!(begin_count, 1, "one fieldBegin for grouped link: {xml}");
    assert_eq!(end_count, 1, "one fieldEnd for grouped link: {xml}");
    assert!(xml.contains("part1"), "first part: {xml}");
    assert!(xml.contains("part2"), "second part: {xml}");
}

// ── Phase 9 tests: hyperlink reader ───────────────────────────────────────

#[test]
fn reader_parses_hyperlink_from_field_begin_end() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
        xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
  <hp:p id="0" paraPrIDRef="0">
    <hp:run charPrIDRef="0">
      <hp:fieldBegin type="HYPERLINK" command="https://example.com"/>
    </hp:run>
    <hp:run charPrIDRef="0">
      <hp:t>click here</hp:t>
    </hp:run>
    <hp:run charPrIDRef="0">
      <hp:fieldEnd type="HYPERLINK"/>
    </hp:run>
  </hp:p>
</hs:sec>"#;

    let section = read_hwpx_section_xml(xml);
    assert_eq!(section.blocks.len(), 1, "one paragraph");

    let inlines = match &section.blocks[0] {
        Block::Paragraph { inlines } => inlines,
        other => panic!("expected Paragraph, got: {other:?}"),
    };

    assert!(!inlines.is_empty(), "inlines should not be empty");
    let link_inline = &inlines[0];
    assert_eq!(link_inline.text, "click here");
    assert_eq!(
        link_inline.link.as_deref(),
        Some("https://example.com"),
        "link URL must be parsed from fieldBegin command"
    );
}

#[test]
fn reader_non_hyperlink_field_does_not_set_link() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
        xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
  <hp:p id="0" paraPrIDRef="0">
    <hp:run charPrIDRef="0">
      <hp:fieldBegin type="OTHER" command="something"/>
    </hp:run>
    <hp:run charPrIDRef="0">
      <hp:t>not a link</hp:t>
    </hp:run>
    <hp:run charPrIDRef="0">
      <hp:fieldEnd type="OTHER"/>
    </hp:run>
  </hp:p>
</hs:sec>"#;

    let section = read_hwpx_section_xml(xml);
    let inlines = match &section.blocks[0] {
        Block::Paragraph { inlines } => inlines,
        other => panic!("expected Paragraph, got: {other:?}"),
    };

    assert!(!inlines.is_empty());
    assert!(
        inlines[0].link.is_none(),
        "non-HYPERLINK field must not set link: {:?}",
        inlines[0]
    );
}

#[test]
fn reader_text_outside_hyperlink_has_no_link() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
        xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
  <hp:p id="0" paraPrIDRef="0">
    <hp:run charPrIDRef="0">
      <hp:t>before</hp:t>
    </hp:run>
    <hp:run charPrIDRef="0">
      <hp:fieldBegin type="HYPERLINK" command="https://example.com"/>
    </hp:run>
    <hp:run charPrIDRef="0">
      <hp:t>linked</hp:t>
    </hp:run>
    <hp:run charPrIDRef="0">
      <hp:fieldEnd type="HYPERLINK"/>
    </hp:run>
    <hp:run charPrIDRef="0">
      <hp:t>after</hp:t>
    </hp:run>
  </hp:p>
</hs:sec>"#;

    let section = read_hwpx_section_xml(xml);
    let inlines = match &section.blocks[0] {
        Block::Paragraph { inlines } => inlines,
        other => panic!("expected Paragraph, got: {other:?}"),
    };

    assert_eq!(inlines.len(), 3, "three inlines: before, linked, after");
    assert!(inlines[0].link.is_none(), "before must have no link");
    assert_eq!(inlines[0].text, "before");
    assert_eq!(
        inlines[1].link.as_deref(),
        Some("https://example.com"),
        "linked inline must have URL"
    );
    assert_eq!(inlines[1].text, "linked");
    assert!(inlines[2].link.is_none(), "after must have no link");
    assert_eq!(inlines[2].text, "after");
}

// ── Phase 9 audit: missing tests ─────────────────────────────────────────

#[test]
fn section_xml_adjacent_link_inlines_different_urls_produce_two_field_groups() {
    // Two consecutive inlines with DIFFERENT URLs must each produce their own
    // fieldBegin/fieldEnd pair.  The grouping logic in write_inlines() only
    // merges consecutive spans that share the same URL.
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![
            Inline {
                text: "A".into(),
                link: Some("https://a.com".into()),
                ..Inline::default()
            },
            Inline {
                text: "B".into(),
                link: Some("https://b.com".into()),
                ..Inline::default()
            },
        ],
    }]);

    let begin_count = xml.matches("hp:fieldBegin").count();
    let end_count = xml.matches("hp:fieldEnd").count();
    assert_eq!(
        begin_count, 2,
        "two different URLs must produce two fieldBegin elements: {xml}"
    );
    assert_eq!(
        end_count, 2,
        "two different URLs must produce two fieldEnd elements: {xml}"
    );
    assert!(xml.contains("https://a.com"), "first URL: {xml}");
    assert!(xml.contains("https://b.com"), "second URL: {xml}");
    assert!(xml.contains("A"), "first link text: {xml}");
    assert!(xml.contains("B"), "second link text: {xml}");
}
