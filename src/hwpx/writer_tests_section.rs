use super::*;

// ── generate_section_xml: structural tests ────────────────────────────

#[test]
fn section_xml_empty_section_produces_valid_wrapper() {
    let xml = section_xml(vec![]);
    assert!(xml.contains("<hs:sec"), "root element missing: {xml}");
    assert!(xml.contains("</hs:sec>"), "closing tag missing: {xml}");
    assert!(xml.contains(r#"xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section""#));
}

#[test]
fn section_xml_paragraph_plain_text() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![inline("hello world")],
    }]);
    assert!(xml.contains("<hp:p "), "paragraph open: {xml}");
    assert!(xml.contains("</hp:p>"), "paragraph close: {xml}");
    assert!(xml.contains("<hp:t>"), "text run open: {xml}");
    assert!(xml.contains("hello world"), "text content: {xml}");
}

#[test]
fn section_xml_empty_paragraph() {
    let xml = section_xml(vec![Block::Paragraph { inlines: vec![] }]);
    assert!(xml.contains("<hp:p "));
    assert!(xml.contains("</hp:p>"));
}

#[test]
fn section_xml_heading_level_1() {
    let xml = section_xml(vec![Block::Heading {
        level: 1,
        inlines: vec![inline("Title")],
    }]);
    // Heading 1 → numeric styleIDRef="1" (matches hh:styles table id=1)
    assert!(
        xml.contains(r#"hp:styleIDRef="1""#),
        "h1 style ref must be numeric 1: {xml}"
    );
    assert!(xml.contains("Title"));
}

#[test]
fn section_xml_heading_level_6() {
    let xml = section_xml(vec![Block::Heading {
        level: 6,
        inlines: vec![inline("Deep")],
    }]);
    // Heading 6 → numeric styleIDRef="6"
    assert!(xml.contains(r#"hp:styleIDRef="6""#), "{xml}");
    assert!(xml.contains("Deep"));
}

#[test]
fn section_xml_bold_inline_has_charpr_id_ref() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![bold_inline("strong")],
    }]);
    // Bold formatting is emitted as an inline <hp:charPr bold="true"/> inside
    // the <hp:run> (section-level charPr), in addition to the header charPr
    // table entry referenced by charPrIDRef.
    assert!(
        xml.contains("<hp:charPr "),
        "inline charPr must be emitted for bold: {xml}"
    );
    assert!(
        xml.contains(r#"bold="true""#),
        "bold attribute must appear on inline charPr: {xml}"
    );
    assert!(xml.contains("charPrIDRef="), "charPrIDRef: {xml}");
    assert!(xml.contains("strong"));
}

#[test]
fn section_xml_italic_inline_has_charpr_id_ref() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![italic_inline("em")],
    }]);
    assert!(xml.contains("charPrIDRef="), "{xml}");
}

#[test]
fn section_xml_underline_inline_has_charpr_id_ref() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![underline_inline("ul")],
    }]);
    assert!(xml.contains("charPrIDRef="), "{xml}");
}

#[test]
fn section_xml_strikethrough_inline_has_charpr_id_ref() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![Inline {
            text: "del".into(),
            strikethrough: true,
            ..Inline::default()
        }],
    }]);
    assert!(xml.contains("charPrIDRef="), "{xml}");
}

#[test]
fn section_xml_plain_inline_has_no_charpr() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![inline("plain")],
    }]);
    // Plain text → no inline hp:charPr element (formatting is only in header table)
    assert!(
        !xml.contains("<hp:charPr"),
        "unexpected inline charPr: {xml}"
    );
}

#[test]
fn section_xml_plain_inline_has_charpr_id_ref() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![inline("plain")],
    }]);
    // Plain text → charPrIDRef="0" on the run element
    assert!(
        xml.contains(r#"charPrIDRef="0""#),
        "charPrIDRef missing: {xml}"
    );
}

#[test]
fn section_xml_bold_inline_has_nonzero_charpr_id_ref() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![bold_inline("bold")],
    }]);
    // Bold → charPrIDRef pointing to id=1 (first non-default entry)
    assert!(xml.contains("charPrIDRef="), "charPrIDRef missing: {xml}");
}

#[test]
fn section_xml_paragraph_has_para_pr_id_ref() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![inline("text")],
    }]);
    assert!(
        xml.contains(r#"paraPrIDRef="0""#),
        "paraPrIDRef missing: {xml}"
    );
}

#[test]
fn section_xml_nested_inlines_bold_then_italic() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![bold_inline("B"), italic_inline("I")],
    }]);
    // Each formatting variant gets a distinct charPrIDRef; bold and italic
    // runs are separate so they must have different IDs (both non-zero).
    assert!(xml.contains("charPrIDRef="), "{xml}");
    assert!(xml.contains('B'));
    assert!(xml.contains('I'));
}

#[test]
fn section_xml_image_block() {
    let xml = section_xml(vec![Block::Image {
        src: "image001.png".into(),
        alt: "a cat".into(),
    }]);
    assert!(xml.contains("<hp:p "), "{xml}");
    assert!(
        xml.contains(r#"hp:binaryItemIDRef="image001.png""#),
        "{xml}"
    );
    assert!(xml.contains(r#"alt="a cat""#), "{xml}");
    assert!(xml.contains("<hp:img"), "{xml}");
    // 6-A: image must be wrapped in <hp:run><hp:pic>...</hp:pic></hp:run>
    assert!(
        xml.contains(r#"<hp:run charPrIDRef="0">"#),
        "image run wrapper missing: {xml}"
    );
    assert!(xml.contains("<hp:pic>"), "hp:pic wrapper missing: {xml}");
    assert!(xml.contains("</hp:pic>"), "hp:pic close missing: {xml}");
    // Verify nesting order: run → pic → img
    let run_pos = xml
        .find(r#"<hp:run charPrIDRef="0">"#)
        .expect("run position");
    let pic_pos = xml.find("<hp:pic>").expect("pic position");
    let img_pos = xml.find("<hp:img").expect("img position");
    assert!(run_pos < pic_pos, "run must precede pic: {xml}");
    assert!(pic_pos < img_pos, "pic must precede img: {xml}");
}

#[test]
fn section_xml_table_2x2() {
    let cell = |text: &str| TableCell {
        blocks: vec![Block::Paragraph {
            inlines: vec![inline(text)],
        }],
        colspan: 1,
        rowspan: 1,
    };
    let xml = section_xml(vec![Block::Table {
        col_count: 2,
        rows: vec![
            TableRow {
                cells: vec![cell("A"), cell("B")],
                is_header: false,
            },
            TableRow {
                cells: vec![cell("C"), cell("D")],
                is_header: false,
            },
        ],
    }]);
    // 5-B: table must be wrapped in a paragraph container
    assert!(xml.contains("<hp:p "), "p wrapper: {xml}");
    assert!(
        xml.contains(r#"<hp:run charPrIDRef="0">"#),
        "run wrapper: {xml}"
    );
    // 5-C: tbl must carry rowCnt and colCnt
    assert!(
        xml.contains(r#"<hp:tbl rowCnt="2" colCnt="2">"#),
        "tbl with rowCnt/colCnt: {xml}"
    );
    assert!(xml.contains("</hp:tbl>"), "tbl close: {xml}");
    assert_eq!(xml.matches("<hp:tr>").count(), 2, "two rows: {xml}");
    assert_eq!(xml.matches("<hp:tc>").count(), 4, "four cells: {xml}");
    assert!(xml.contains('A'), "{xml}");
    assert!(xml.contains('D'), "{xml}");
}

#[test]
fn section_xml_table_colspan_rowspan_present() {
    // After the fix, the writer emits <hp:cellAddr colSpan="…" rowSpan="…"/>
    // inside <hp:tc> when either value differs from 1.
    let wide_cell = TableCell {
        blocks: vec![],
        colspan: 2,
        rowspan: 1,
    };
    let xml = section_xml(vec![Block::Table {
        col_count: 2,
        rows: vec![TableRow {
            cells: vec![wide_cell],
            is_header: false,
        }],
    }]);
    assert!(xml.contains("<hp:tc>"), "plain tc emitted: {xml}");
    assert!(
        xml.contains("colSpan=\"2\""),
        "colSpan should appear: {xml}"
    );
    assert!(
        xml.contains("rowSpan=\"1\""),
        "rowSpan should appear: {xml}"
    );
    assert!(xml.contains("<hp:cellAddr"), "cellAddr element: {xml}");
}

#[test]
fn section_xml_table_no_celladdr_for_1x1() {
    // When colspan=1 and rowspan=1, no <hp:cellAddr> should be emitted.
    let cell = TableCell {
        blocks: vec![Block::Paragraph {
            inlines: vec![inline("normal")],
        }],
        colspan: 1,
        rowspan: 1,
    };
    let xml = section_xml(vec![Block::Table {
        col_count: 1,
        rows: vec![TableRow {
            cells: vec![cell],
            is_header: false,
        }],
    }]);
    assert!(xml.contains("<hp:tc>"), "tc emitted: {xml}");
    assert!(
        !xml.contains("<hp:cellAddr"),
        "cellAddr must NOT appear for 1x1: {xml}"
    );
}

#[test]
fn section_xml_math_block() {
    let xml = section_xml(vec![Block::Math {
        display: true,
        tex: r"E = mc^2".into(),
    }]);
    assert!(xml.contains("<hp:equation>"), "equation open: {xml}");
    assert!(xml.contains("</hp:equation>"), "equation close: {xml}");
    assert!(xml.contains(r"E = mc^2"), "{xml}");
    // 6-B: equation must be wrapped in <hp:run charPrIDRef="0">
    assert!(
        xml.contains(r#"<hp:run charPrIDRef="0">"#),
        "equation run wrapper missing: {xml}"
    );
    // Verify nesting order: run → equation
    let run_pos = xml
        .find(r#"<hp:run charPrIDRef="0">"#)
        .expect("run position");
    let eq_pos = xml.find("<hp:equation>").expect("equation position");
    assert!(run_pos < eq_pos, "run must precede equation: {xml}");
}

#[test]
fn section_xml_ordered_list() {
    let xml = section_xml(vec![Block::List {
        ordered: true,
        start: 1,
        items: vec![
            ListItem {
                blocks: vec![Block::Paragraph {
                    inlines: vec![inline("first")],
                }],
                children: vec![],
                checked: None,
            },
            ListItem {
                blocks: vec![Block::Paragraph {
                    inlines: vec![inline("second")],
                }],
                children: vec![],
                checked: None,
            },
        ],
    }]);
    assert!(xml.contains("first"), "{xml}");
    assert!(xml.contains("second"), "{xml}");
    assert_eq!(xml.matches("<hp:p ").count(), 2, "{xml}");
}

#[test]
fn section_xml_unordered_list() {
    let xml = section_xml(vec![Block::List {
        ordered: false,
        start: 1,
        items: vec![ListItem {
            blocks: vec![Block::Paragraph {
                inlines: vec![inline("bullet")],
            }],
            children: vec![],
            checked: None,
        }],
    }]);
    assert!(xml.contains("bullet"), "{xml}");
}

#[test]
fn section_xml_footnote_block() {
    let xml = section_xml(vec![Block::Footnote {
        id: "fn1".into(),
        content: vec![Block::Paragraph {
            inlines: vec![inline("footnote text")],
        }],
    }]);
    assert!(xml.contains("footnote text"), "{xml}");
}

#[test]
fn section_xml_blockquote() {
    let xml = section_xml(vec![Block::BlockQuote {
        blocks: vec![Block::Paragraph {
            inlines: vec![inline("quoted")],
        }],
    }]);
    assert!(xml.contains("quoted"), "{xml}");
    assert!(xml.contains("<hp:p "), "{xml}");
    // Block-quote paragraphs must reference paraPrIDRef="1" (indented style).
    assert!(
        xml.contains(r#"paraPrIDRef="1""#),
        "blockquote paragraph must use paraPrIDRef=\"1\": {xml}"
    );
    // Must NOT use paraPrIDRef="0" for the quoted paragraph.
    assert!(
        !xml.contains(r#"paraPrIDRef="0""#),
        "blockquote paragraph must NOT use paraPrIDRef=\"0\": {xml}"
    );
}

#[test]
fn section_xml_horizontal_rule() {
    let xml = section_xml(vec![Block::HorizontalRule]);
    assert!(xml.contains("<hp:p "), "{xml}");
    // The writer emits a line of em-dashes as a visual rule.
    assert!(xml.contains("───"), "{xml}");
}

#[test]
fn section_xml_page_break_emits_newpage_ctrl() {
    let xml = section_xml(vec![Block::PageBreak]);
    assert!(
        xml.contains("<hp:p "),
        "page break wrapped in paragraph: {xml}"
    );
    assert!(
        xml.contains("<hp:run"),
        "page break ctrl must live inside a run: {xml}"
    );
    assert!(
        xml.contains(r#"id="newPage""#),
        "page break must emit hp:ctrl id=newPage: {xml}"
    );
    // Must not emit any text run.
    assert!(
        !xml.contains("<hp:t>"),
        "page break paragraph must contain no text node: {xml}"
    );
}

#[test]
fn section_xml_code_block() {
    let xml = section_xml(vec![Block::CodeBlock {
        language: Some("rust".into()),
        code: "fn main() {}".into(),
    }]);
    assert!(xml.contains("<hp:p "), "{xml}");
    assert!(
        !xml.contains(r#"charPrIDRef="code""#),
        "charPrIDRef must not be the string 'code': {xml}"
    );
    assert!(
        xml.contains("charPrIDRef="),
        "charPrIDRef must be present: {xml}"
    );
    assert!(xml.contains("fn main() {}"), "{xml}");
}
