use super::*;

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

    assert!(
        xml.contains("hp:noteRef"),
        "noteRef element: {xml}"
    );
    assert!(
        xml.contains(r#"noteId="42""#),
        "noteId=42: {xml}"
    );
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

    assert!(
        run_pos < noteref_pos,
        "run must precede noteRef: {xml}"
    );
    assert!(
        noteref_pos < run_end_pos,
        "noteRef must precede run close: {xml}"
    );
}
