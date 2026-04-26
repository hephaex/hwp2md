use super::*;

// ── Phase A-4 tests: footnote block <hp:fn> wrapping ────────────────────

#[test]
fn section_xml_footnote_block_wrapped_in_hp_fn() {
    let xml = section_xml(vec![Block::Footnote {
        id: "1".into(),
        content: vec![Block::Paragraph {
            inlines: vec![Inline::plain("footnote body")],
        }],
    }]);

    assert!(
        xml.contains("<hp:fn"),
        "hp:fn element must be present: {xml}"
    );
    assert!(
        xml.contains(r#"noteId="1""#),
        "noteId attribute must be on hp:fn: {xml}"
    );
    assert!(
        xml.contains("</hp:fn>"),
        "hp:fn closing tag must be present: {xml}"
    );
}

#[test]
fn section_xml_footnote_content_inside_hp_fn() {
    let xml = section_xml(vec![Block::Footnote {
        id: "2".into(),
        content: vec![Block::Paragraph {
            inlines: vec![Inline::plain("inside footnote")],
        }],
    }]);

    // The paragraph content must appear between <hp:fn> and </hp:fn>.
    let fn_open = xml.find("<hp:fn").expect("hp:fn open must exist");
    let text_pos = xml.find("inside footnote").expect("footnote text must exist");
    let fn_close = xml.find("</hp:fn>").expect("hp:fn close must exist");

    assert!(
        fn_open < text_pos,
        "hp:fn must precede footnote text: {xml}"
    );
    assert!(
        text_pos < fn_close,
        "footnote text must precede hp:fn close: {xml}"
    );
}

#[test]
fn section_xml_footnote_paragraph_wrapped_in_hp_p() {
    let xml = section_xml(vec![Block::Footnote {
        id: "3".into(),
        content: vec![Block::Paragraph {
            inlines: vec![Inline::plain("fn paragraph")],
        }],
    }]);

    // Footnote body paragraphs must be proper <hp:p> inside <hp:fn>.
    let fn_open = xml.find("<hp:fn").expect("hp:fn open");
    let p_open = xml[fn_open..].find("<hp:p").expect("hp:p inside hp:fn");
    let p_close = xml[fn_open..].find("</hp:p>").expect("hp:p close inside hp:fn");
    let fn_close = xml[fn_open..].find("</hp:fn>").expect("hp:fn close");

    assert!(
        p_open < fn_close,
        "hp:p must be inside hp:fn: {xml}"
    );
    assert!(
        p_close < fn_close,
        "hp:p close must be inside hp:fn: {xml}"
    );
}

#[test]
fn section_xml_footnote_multiple_paragraphs() {
    let xml = section_xml(vec![Block::Footnote {
        id: "4".into(),
        content: vec![
            Block::Paragraph {
                inlines: vec![Inline::plain("first paragraph")],
            },
            Block::Paragraph {
                inlines: vec![Inline::plain("second paragraph")],
            },
        ],
    }]);

    assert!(
        xml.contains("first paragraph"),
        "first paragraph must exist: {xml}"
    );
    assert!(
        xml.contains("second paragraph"),
        "second paragraph must exist: {xml}"
    );

    // Both must be inside the single <hp:fn> block.
    let fn_open = xml.find("<hp:fn").expect("hp:fn open");
    let fn_close = xml.find("</hp:fn>").expect("hp:fn close");
    let first_pos = xml.find("first paragraph").expect("first");
    let second_pos = xml.find("second paragraph").expect("second");

    assert!(fn_open < first_pos && first_pos < fn_close, "first para inside hp:fn: {xml}");
    assert!(fn_open < second_pos && second_pos < fn_close, "second para inside hp:fn: {xml}");
}

#[test]
fn section_xml_footnote_empty_content_still_emits_hp_fn() {
    let xml = section_xml(vec![Block::Footnote {
        id: "5".into(),
        content: vec![],
    }]);

    // Even with no content blocks the <hp:fn> wrapper must appear.
    assert!(
        xml.contains("<hp:fn"),
        "hp:fn must appear even for empty footnote: {xml}"
    );
    assert!(
        xml.contains("</hp:fn>"),
        "hp:fn close must appear: {xml}"
    );
}

#[test]
fn roundtrip_footnote_block_preserved() {
    // Write a document with noteRef inline + Footnote block, read it back,
    // and verify both survive the roundtrip.
    let blocks = vec![
        Block::Paragraph {
            inlines: vec![
                Inline::plain("See note"),
                Inline {
                    footnote_ref: Some("1".into()),
                    ..Inline::default()
                },
            ],
        },
        Block::Footnote {
            id: "1".into(),
            content: vec![Block::Paragraph {
                inlines: vec![Inline::plain("This is the footnote.")],
            }],
        },
    ];

    let doc = doc_with_section(blocks);
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");
    let read_back = read_hwpx(tmp.path()).expect("read_hwpx");

    let section = &read_back.sections[0];

    // Find the footnote_ref inline.
    let has_noteref = section.blocks.iter().any(|b| {
        if let Block::Paragraph { inlines } = b {
            inlines.iter().any(|i| i.footnote_ref.is_some())
        } else {
            false
        }
    });
    assert!(has_noteref, "footnote_ref must survive roundtrip: {section:?}");

    // Find the Footnote block.
    let has_footnote = section.blocks.iter().any(|b| {
        matches!(b, Block::Footnote { id, content } if id == "1" && !content.is_empty())
    });
    assert!(has_footnote, "Footnote block must survive roundtrip: {section:?}");
}

#[test]
fn roundtrip_footnote_body_text_preserved() {
    let blocks = vec![Block::Footnote {
        id: "7".into(),
        content: vec![Block::Paragraph {
            inlines: vec![Inline::plain("roundtrip footnote text")],
        }],
    }];

    let doc = doc_with_section(blocks);
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");
    let read_back = read_hwpx(tmp.path()).expect("read_hwpx");

    let section = &read_back.sections[0];
    let footnote_text = section.blocks.iter().find_map(|b| {
        if let Block::Footnote { content, .. } = b {
            content.iter().find_map(|inner| {
                if let Block::Paragraph { inlines } = inner {
                    Some(
                        inlines
                            .iter()
                            .map(|i| i.text.as_str())
                            .collect::<String>(),
                    )
                } else {
                    None
                }
            })
        } else {
            None
        }
    });
    assert_eq!(
        footnote_text.as_deref(),
        Some("roundtrip footnote text"),
        "footnote body text must survive roundtrip: {section:?}"
    );
}

// ── Phase 10 tests: footnote_ref writer ─────────────────────────────────

#[test]
fn section_xml_footnote_ref_produces_note_ref_element() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![Inline {
            footnote_ref: Some("1".into()),
            ..Inline::default()
        }],
    }]);

    assert!(
        xml.contains("<hp:noteRef"),
        "noteRef element must be present: {xml}"
    );
    assert!(
        xml.contains(r#"noteId="1""#),
        "noteId attribute must be present: {xml}"
    );
    assert!(
        xml.contains(r#"type="FOOTNOTE""#),
        "type attribute must be FOOTNOTE: {xml}"
    );
}

#[test]
fn section_xml_footnote_ref_is_self_closing() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![Inline {
            footnote_ref: Some("42".into()),
            ..Inline::default()
        }],
    }]);

    assert!(xml.contains("hp:noteRef"), "noteRef element: {xml}");
    assert!(xml.contains(r#"noteId="42""#), "noteId=42: {xml}");
    // Must NOT contain <hp:t> for a pure footnote_ref with empty text.
    assert!(
        !xml.contains("<hp:t>"),
        "footnote_ref with empty text must not emit <hp:t>: {xml}"
    );
}

#[test]
fn section_xml_footnote_ref_with_text_emits_text_not_noteref() {
    // When footnote_ref is set but text is non-empty, the text should be
    // emitted as normal (the footnote_ref is secondary metadata in that case).
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![Inline {
            text: "visible".into(),
            footnote_ref: Some("5".into()),
            ..Inline::default()
        }],
    }]);

    assert!(
        xml.contains("visible"),
        "text content must be present: {xml}"
    );
    assert!(
        xml.contains("<hp:t>"),
        "text element for non-empty footnote_ref: {xml}"
    );
    // noteRef should NOT appear when there is visible text.
    assert!(
        !xml.contains("<hp:noteRef"),
        "noteRef must not appear when text is non-empty: {xml}"
    );
}

#[test]
fn section_xml_footnote_ref_wrapped_in_run() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![Inline {
            footnote_ref: Some("3".into()),
            ..Inline::default()
        }],
    }]);

    // noteRef must be inside an <hp:run> element.
    let run_pos = xml.find("<hp:run").expect("run element must exist");
    let noteref_pos = xml.find("hp:noteRef").expect("noteRef must exist");
    let run_end_pos = xml.find("</hp:run>").expect("run close must exist");

    assert!(run_pos < noteref_pos, "run must precede noteRef: {xml}");
    assert!(
        noteref_pos < run_end_pos,
        "noteRef must precede run close: {xml}"
    );
}
