use super::*;

// ── Advanced / ID tests ───────────────────────────────────────────────────

#[test]
fn section_xml_multiple_blocks_ordering() {
    let xml = section_xml(vec![
        Block::Heading {
            level: 2,
            inlines: vec![inline("Section")],
        },
        Block::Paragraph {
            inlines: vec![inline("Body text")],
        },
    ]);
    // The heading must come before the paragraph in document order.
    let heading_pos = xml.find("Section").expect("heading text");
    let para_pos = xml.find("Body text").expect("para text");
    assert!(heading_pos < para_pos, "heading before paragraph: {xml}");
}

// ── Phase 4 tests: styles, numeric styleIDRef, numeric charPrIDRef ────────

#[test]
fn section_xml_heading_has_numeric_style_id_ref() {
    // Verify that heading styleIDRef is a pure decimal digit, not a name
    // like "Heading1".
    for level in 1u8..=6 {
        let xml = section_xml(vec![Block::Heading {
            level,
            inlines: vec![inline("h")],
        }]);
        let expected = format!(r#"hp:styleIDRef="{level}""#);
        assert!(
            xml.contains(&expected),
            "level {level}: expected numeric styleIDRef={level}, got: {xml}"
        );
        // Must NOT contain the old string form.
        let old_form = format!(r#"hp:styleIDRef="Heading{level}""#);
        assert!(
            !xml.contains(&old_form),
            "level {level}: string styleIDRef must not appear: {xml}"
        );
    }
}

#[test]
fn section_xml_code_block_has_numeric_char_pr_id_ref() {
    let xml = section_xml(vec![Block::CodeBlock {
        language: Some("rust".into()),
        code: "let x = 1;".into(),
    }]);
    // The charPrIDRef value on the run element must be a decimal number.
    // Extract it and confirm it parses as u32.
    let marker = "charPrIDRef=\"";
    let start = xml
        .find(marker)
        .expect("charPrIDRef attribute must be present");
    let rest = &xml[start + marker.len()..];
    let end = rest.find('"').expect("closing quote");
    let value = &rest[..end];
    assert!(
        value.parse::<u32>().is_ok(),
        "charPrIDRef value '{value}' must be a numeric u32, not a string like 'code': {xml}"
    );
    // Sanity: "let x = 1;" must appear in the output.
    assert!(
        xml.contains("let x = 1;"),
        "code content must appear: {xml}"
    );
}

// ── Phase 5 tests: paragraph IDs + table wrapping ────────────────────────

#[test]
fn section_xml_paragraphs_have_sequential_ids() {
    // Multiple blocks must receive sequential id="0", id="1", … on their <hp:p>.
    let xml = section_xml(vec![
        Block::Heading {
            level: 1,
            inlines: vec![inline("First")],
        },
        Block::Paragraph {
            inlines: vec![inline("Second")],
        },
        Block::CodeBlock {
            language: None,
            code: "third".into(),
        },
    ]);
    assert!(xml.contains(r#"id="0""#), "first block id=0: {xml}");
    assert!(xml.contains(r#"id="1""#), "second block id=1: {xml}");
    assert!(xml.contains(r#"id="2""#), "third block id=2: {xml}");
    // Verify the first paragraph element is the heading (id=0 + styleIDRef).
    let id0_pos = xml.find(r#"id="0""#).expect("id=0 position");
    let style_pos = xml
        .find(r#"hp:styleIDRef="1""#)
        .expect("styleIDRef=1 position");
    // id="0" must appear on the same opening tag as hp:styleIDRef="1".
    // Since the heading opens before the content, id0_pos < style_pos for a
    // forward-ordered attribute list (writer emits id first).
    assert!(
        id0_pos < style_pos,
        "id=0 must precede styleIDRef on the heading element: {xml}"
    );
}

#[test]
fn section_xml_table_wrapped_in_paragraph() {
    // A table block must be emitted as:
    //   <hp:p id="N" paraPrIDRef="0">
    //     <hp:run charPrIDRef="0">
    //       <hp:tbl rowCnt="…" colCnt="…"> … </hp:tbl>
    //     </hp:run>
    //   </hp:p>
    let cell = |text: &str| TableCell {
        blocks: vec![Block::Paragraph {
            inlines: vec![inline(text)],
        }],
        colspan: 1,
        rowspan: 1,
    };
    let xml = section_xml(vec![Block::Table {
        col_count: 2,
        rows: vec![TableRow {
            cells: vec![cell("X"), cell("Y")],
            is_header: false,
        }],
    }]);
    // The outer paragraph wrapper must carry an id.
    assert!(
        xml.contains(r#"id="0""#),
        "table wrapper p must have id=0: {xml}"
    );
    // The run wrapper must be present with charPrIDRef="0".
    assert!(
        xml.contains(r#"<hp:run charPrIDRef="0">"#),
        "run wrapper: {xml}"
    );
    // The table element must be present with rowCnt and colCnt.
    assert!(
        xml.contains(r#"<hp:tbl rowCnt="1" colCnt="2">"#),
        "tbl attrs: {xml}"
    );
    // Structural nesting: p opens before run, run opens before tbl.
    let p_pos = xml.find(r#"id="0""#).expect("p wrapper position");
    let run_pos = xml
        .find(r#"<hp:run charPrIDRef="0">"#)
        .expect("run position");
    let tbl_pos = xml
        .find(r#"<hp:tbl rowCnt="1" colCnt="2">"#)
        .expect("tbl position");
    assert!(p_pos < run_pos, "p must come before run: {xml}");
    assert!(run_pos < tbl_pos, "run must come before tbl: {xml}");
    // Cell paragraph IDs (inside the table) continue from the outer counter.
    // The outer table p is id=0, so the first cell paragraph is id=1.
    assert!(xml.contains(r#"id="1""#), "cell paragraph id=1: {xml}");
}

#[test]
fn section_xml_table_rowcnt_colcnt_attributes() {
    // Verify rowCnt and colCnt reflect actual row/column counts.
    let cell = || TableCell {
        blocks: vec![],
        colspan: 1,
        rowspan: 1,
    };
    let xml = section_xml(vec![Block::Table {
        col_count: 3,
        rows: vec![
            TableRow {
                cells: vec![cell(), cell(), cell()],
                is_header: false,
            },
            TableRow {
                cells: vec![cell(), cell(), cell()],
                is_header: false,
            },
            TableRow {
                cells: vec![cell(), cell(), cell()],
                is_header: false,
            },
        ],
    }]);
    assert!(xml.contains(r#"rowCnt="3""#), "rowCnt must be 3: {xml}");
    assert!(xml.contains(r#"colCnt="3""#), "colCnt must be 3: {xml}");
}

// ── Phase 8 tests: inline code charPr ───────────────────────────────────

#[test]
fn section_xml_inline_code_has_monospace_charpr_id_ref() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![
            inline("text "),
            Inline {
                text: "code_val".into(),
                code: true,
                ..Inline::default()
            },
        ],
    }]);

    // There must be at least two <hp:run> elements with different charPrIDRef values.
    let run_count = xml.matches("<hp:run ").count();
    assert!(
        run_count >= 2,
        "must have at least 2 runs (plain + code): {xml}"
    );

    // The code text must be present.
    assert!(xml.contains("code_val"), "code text missing: {xml}");

    // Extract all charPrIDRef values and verify they are not all the same.
    let marker = "charPrIDRef=\"";
    let ids: Vec<&str> = xml
        .match_indices(marker)
        .map(|(pos, _)| {
            let rest = &xml[pos + marker.len()..];
            let end = rest.find('"').unwrap();
            &rest[..end]
        })
        .collect();
    assert!(
        ids.len() >= 2,
        "must have at least 2 charPrIDRef values: {ids:?}"
    );
    // At least one ID must differ from the plain ID (which is "0").
    let has_nonzero = ids.iter().any(|&id| id != "0");
    assert!(
        has_nonzero,
        "inline code must produce a non-zero charPrIDRef: {ids:?}"
    );
}

// ── Phase A-3 tests: BlockQuote paraPrIDRef indentation ─────────────────

#[test]
fn section_xml_blockquote_uses_para_pr_id_ref_1() {
    // A paragraph inside a BlockQuote must use paraPrIDRef="1" (indented).
    let xml = section_xml(vec![Block::BlockQuote {
        blocks: vec![Block::Paragraph {
            inlines: vec![inline("indented text")],
        }],
    }]);
    assert!(
        xml.contains(r#"paraPrIDRef="1""#),
        "blockquote paragraph must use paraPrIDRef=\"1\": {xml}"
    );
    assert!(
        xml.contains("indented text"),
        "blockquote content must be preserved: {xml}"
    );
}

#[test]
fn section_xml_normal_paragraph_uses_para_pr_id_ref_0() {
    // A normal paragraph (not inside BlockQuote) must use paraPrIDRef="0".
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![inline("normal")],
    }]);
    assert!(
        xml.contains(r#"paraPrIDRef="0""#),
        "normal paragraph must use paraPrIDRef=\"0\": {xml}"
    );
}

#[test]
fn section_xml_blockquote_and_normal_mixed() {
    // When a normal paragraph precedes a BlockQuote, both must use the correct
    // paraPrIDRef values.
    let xml = section_xml(vec![
        Block::Paragraph {
            inlines: vec![inline("before")],
        },
        Block::BlockQuote {
            blocks: vec![Block::Paragraph {
                inlines: vec![inline("quoted")],
            }],
        },
        Block::Paragraph {
            inlines: vec![inline("after")],
        },
    ]);
    assert!(
        xml.contains(r#"paraPrIDRef="0""#),
        "normal paragraphs must use paraPrIDRef=\"0\": {xml}"
    );
    assert!(
        xml.contains(r#"paraPrIDRef="1""#),
        "blockquote paragraph must use paraPrIDRef=\"1\": {xml}"
    );
    // Verify ordering: "before" appears first, then "quoted", then "after".
    let before_pos = xml.find("before").expect("before text");
    let quoted_pos = xml.find("quoted").expect("quoted text");
    let after_pos = xml.find("after").expect("after text");
    assert!(before_pos < quoted_pos, "before must precede quoted: {xml}");
    assert!(quoted_pos < after_pos, "quoted must precede after: {xml}");
}

#[test]
fn section_xml_blockquote_multiple_children() {
    // All child paragraphs inside a BlockQuote must get paraPrIDRef="1".
    let xml = section_xml(vec![Block::BlockQuote {
        blocks: vec![
            Block::Paragraph {
                inlines: vec![inline("first quoted")],
            },
            Block::Paragraph {
                inlines: vec![inline("second quoted")],
            },
        ],
    }]);
    // Count occurrences of paraPrIDRef="1" -- should be exactly 2.
    let count = xml.matches(r#"paraPrIDRef="1""#).count();
    assert_eq!(
        count, 2,
        "both paragraphs in blockquote must use paraPrIDRef=\"1\", found {count}: {xml}"
    );
    assert!(
        !xml.contains(r#"paraPrIDRef="0""#),
        "no paragraph should use paraPrIDRef=\"0\" when all are quoted: {xml}"
    );
}

#[test]
fn section_xml_blockquote_heading_uses_para_pr_id_ref_1() {
    // A heading inside a BlockQuote must also use paraPrIDRef="1".
    let xml = section_xml(vec![Block::BlockQuote {
        blocks: vec![Block::Heading {
            level: 2,
            inlines: vec![inline("Quoted Heading")],
        }],
    }]);
    assert!(
        xml.contains(r#"paraPrIDRef="1""#),
        "heading inside blockquote must use paraPrIDRef=\"1\": {xml}"
    );
    assert!(
        xml.contains(r#"hp:styleIDRef="2""#),
        "heading must still have its styleIDRef: {xml}"
    );
}

#[test]
fn section_xml_nested_blockquote_still_uses_para_pr_id_ref_1() {
    // Nested BlockQuotes still produce paraPrIDRef="1" (we only have two
    // paraPr entries: 0=normal, 1=indented).
    let xml = section_xml(vec![Block::BlockQuote {
        blocks: vec![Block::BlockQuote {
            blocks: vec![Block::Paragraph {
                inlines: vec![inline("deeply quoted")],
            }],
        }],
    }]);
    assert!(
        xml.contains(r#"paraPrIDRef="1""#),
        "nested blockquote must use paraPrIDRef=\"1\": {xml}"
    );
    assert!(
        xml.contains("deeply quoted"),
        "nested blockquote content must be preserved: {xml}"
    );
}
