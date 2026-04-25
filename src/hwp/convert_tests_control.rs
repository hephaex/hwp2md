use super::*;

// -----------------------------------------------------------------------
// control_to_block (IR conversion)
// -----------------------------------------------------------------------

#[test]
fn control_to_block_image_produces_image_block() {
    let ctrl = HwpControl::Image {
        bin_data_id: 7,
        width: 100,
        height: 200,
    };
    let doc_info = DocInfo::default();
    let block = control_to_block(&ctrl, &doc_info).expect("Some");
    assert!(
        matches!(block, ir::Block::Image { ref src, .. } if src == "image_7.bin"),
        "expected Image block with src=image_7.bin, got {block:?}"
    );
}

#[test]
fn control_to_block_empty_table_produces_table_block() {
    let ctrl = HwpControl::Table {
        row_count: 2,
        col_count: 3,
        cells: Vec::new(),
    };
    let doc_info = DocInfo::default();
    let block = control_to_block(&ctrl, &doc_info).expect("Some");
    assert!(matches!(block, ir::Block::Table { col_count: 3, .. }));
}

#[test]
fn control_to_block_footnote_produces_footnote_block() {
    let ctrl = HwpControl::FootnoteEndnote {
        is_endnote: false,
        paragraphs: Vec::new(),
    };
    let doc_info = DocInfo::default();
    let block = control_to_block(&ctrl, &doc_info).expect("Some");
    assert!(matches!(block, ir::Block::Footnote { .. }));
}

#[test]
fn control_to_block_page_break_returns_none() {
    let ctrl = HwpControl::PageBreak;
    let doc_info = DocInfo::default();
    assert!(control_to_block(&ctrl, &doc_info).is_none());
}

#[test]
fn control_to_block_table_groups_cells_into_rows() {
    // 2×2 table with 4 cells.
    let make_cell = |row: u16, col: u16, text: &str| HwpTableCell {
        row,
        col,
        row_span: 1,
        col_span: 1,
        vertical_align: 0,
        is_header: row == 0,
        paragraphs: vec![HwpParagraph {
            text: text.to_string(),
            char_shape_ids: Vec::new(),
            para_shape_id: 0,
            controls: Vec::new(),
            raw_para_text: None,
        }],
    };
    let ctrl = HwpControl::Table {
        row_count: 2,
        col_count: 2,
        cells: vec![
            make_cell(0, 0, "r0c0"),
            make_cell(0, 1, "r0c1"),
            make_cell(1, 0, "r1c0"),
            make_cell(1, 1, "r1c1"),
        ],
    };
    let doc_info = DocInfo::default();
    let block = control_to_block(&ctrl, &doc_info).expect("Some");
    if let ir::Block::Table { rows, col_count } = block {
        assert_eq!(col_count, 2);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].cells.len(), 2);
        assert_eq!(rows[1].cells.len(), 2);
        // First row is marked as header.
        assert!(rows[0].is_header);
        assert!(!rows[1].is_header);
    } else {
        panic!("Expected Table block");
    }
}

#[test]
fn control_to_block_caps_malformed_row_index() {
    let ctrl = HwpControl::Table {
        row_count: 1,
        col_count: 1,
        cells: vec![HwpTableCell {
            row: 50_000, // absurdly large
            col: 0,
            row_span: 1,
            col_span: 1,
            vertical_align: 0,
            is_header: false,
            paragraphs: vec![],
        }],
    };
    let doc_info = DocInfo::default();
    let block = control_to_block(&ctrl, &doc_info).expect("Some");
    if let ir::Block::Table { rows, .. } = block {
        // Row with index 50_000 should be silently dropped (cap is 10_000)
        assert!(rows.len() <= 10_000);
    } else {
        panic!("Expected Table block");
    }
}

#[test]
fn control_to_block_hyperlink_with_url_produces_paragraph() {
    let ctrl = HwpControl::Hyperlink {
        url: "https://example.com".into(),
    };
    let doc_info = DocInfo::default();
    let block = control_to_block(&ctrl, &doc_info).expect("Some");
    if let ir::Block::Paragraph { inlines } = block {
        assert_eq!(inlines.len(), 1);
        assert_eq!(inlines[0].text, "https://example.com");
        assert_eq!(inlines[0].link.as_deref(), Some("https://example.com"));
    } else {
        panic!("Expected Paragraph block");
    }
}

#[test]
fn control_to_block_hyperlink_empty_url_returns_none() {
    let ctrl = HwpControl::Hyperlink { url: String::new() };
    let doc_info = DocInfo::default();
    assert!(control_to_block(&ctrl, &doc_info).is_none());
}

#[test]
fn control_to_block_hyperlink_javascript_url_rejected() {
    let ctrl = HwpControl::Hyperlink {
        url: "javascript:alert(1)".into(),
    };
    let doc_info = DocInfo::default();
    assert!(control_to_block(&ctrl, &doc_info).is_none());
}

// -----------------------------------------------------------------------
// control_to_block — ColumnBreak, SectionBreak, endnote
// -----------------------------------------------------------------------

#[test]
fn control_to_block_column_break_returns_none() {
    let ctrl = HwpControl::ColumnBreak;
    let doc_info = DocInfo::default();
    assert!(control_to_block(&ctrl, &doc_info).is_none());
}

/// `control_to_block` (no counter) still produces the plain prefix IDs — it is
/// used by nested table/blockquote paths that do not carry document-level state.
#[test]
fn control_to_block_endnote_produces_footnote_with_endnote_id() {
    let ctrl = HwpControl::FootnoteEndnote {
        is_endnote: true,
        paragraphs: Vec::new(),
    };
    let doc_info = DocInfo::default();
    let block = control_to_block(&ctrl, &doc_info).expect("Some");
    assert!(matches!(block, ir::Block::Footnote { .. }));
}

#[test]
fn control_to_block_equation_produces_math_block() {
    let ctrl = HwpControl::Equation {
        script: "a + b".into(),
    };
    let doc_info = DocInfo::default();
    let block = control_to_block(&ctrl, &doc_info).expect("Some");
    assert!(matches!(block, ir::Block::Math { display: false, .. }));
}

#[test]
fn control_to_block_ftp_hyperlink_accepted() {
    let ctrl = HwpControl::Hyperlink {
        url: "ftp://files.example.com/archive.tar.gz".into(),
    };
    let doc_info = DocInfo::default();
    let block = control_to_block(&ctrl, &doc_info).expect("ftp should be accepted");
    assert!(matches!(block, ir::Block::Paragraph { .. }));
}

#[test]
fn control_to_block_table_with_zero_col_count_infers_from_cells() {
    let ctrl = HwpControl::Table {
        row_count: 1,
        col_count: 0, // unknown
        cells: vec![
            HwpTableCell {
                row: 0,
                col: 0,
                row_span: 1,
                col_span: 1,
                vertical_align: 0,
                is_header: true,
                paragraphs: vec![],
            },
            HwpTableCell {
                row: 0,
                col: 1,
                row_span: 1,
                col_span: 1,
                vertical_align: 0,
                is_header: true,
                paragraphs: vec![],
            },
        ],
    };
    let doc_info = DocInfo::default();
    let block = control_to_block(&ctrl, &doc_info).expect("Some");
    if let ir::Block::Table { col_count, .. } = block {
        // Should infer at least 2 from cell col indices (0+1=2 max col+1)
        assert!(col_count >= 2);
    } else {
        panic!("Expected Table block");
    }
}

// -----------------------------------------------------------------------
// control_to_block — HwpControl::Ruby
// -----------------------------------------------------------------------

#[test]
fn control_to_block_ruby_with_both_texts_produces_paragraph() {
    let ctrl = HwpControl::Ruby {
        base_text: "漢字".into(),
        ruby_text: "한자".into(),
    };
    let doc_info = DocInfo::default();
    let block = control_to_block(&ctrl, &doc_info).expect("should return Some");
    if let ir::Block::Paragraph { inlines } = block {
        assert_eq!(inlines.len(), 1);
        assert_eq!(inlines[0].text, "漢字");
        assert_eq!(inlines[0].ruby.as_deref(), Some("한자"));
    } else {
        panic!("expected Paragraph, got {block:?}");
    }
}

#[test]
fn control_to_block_ruby_with_empty_ruby_text_no_annotation() {
    let ctrl = HwpControl::Ruby {
        base_text: "漢字".into(),
        ruby_text: String::new(),
    };
    let doc_info = DocInfo::default();
    let block = control_to_block(&ctrl, &doc_info).expect("should return Some for non-empty base");
    if let ir::Block::Paragraph { inlines } = block {
        assert_eq!(inlines[0].text, "漢字");
        assert!(
            inlines[0].ruby.is_none(),
            "empty ruby_text must produce None annotation"
        );
    } else {
        panic!("expected Paragraph, got {block:?}");
    }
}

#[test]
fn control_to_block_ruby_both_empty_returns_none() {
    let ctrl = HwpControl::Ruby {
        base_text: String::new(),
        ruby_text: String::new(),
    };
    let doc_info = DocInfo::default();
    assert!(
        control_to_block(&ctrl, &doc_info).is_none(),
        "both empty must return None"
    );
}
