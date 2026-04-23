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

#[test]
fn is_safe_url_scheme_accepts_https() {
    assert!(is_safe_url_scheme("https://example.com"));
    assert!(is_safe_url_scheme("HTTP://EXAMPLE.COM"));
    assert!(is_safe_url_scheme("mailto:user@example.com"));
}

#[test]
fn is_safe_url_scheme_rejects_dangerous() {
    assert!(!is_safe_url_scheme("javascript:alert(1)"));
    assert!(!is_safe_url_scheme("data:text/html,<h1>hi</h1>"));
    assert!(!is_safe_url_scheme("vbscript:msgbox"));
}

// -----------------------------------------------------------------------
// guess_mime
// -----------------------------------------------------------------------

#[test]
fn guess_mime_png_magic() {
    let data = [0x89, b'P', b'N', b'G', 0x00, 0x00];
    assert_eq!(guess_mime(&data), "image/png");
}

#[test]
fn guess_mime_jpeg_magic() {
    let data = [0xFF, 0xD8, 0xFF, 0xE0, 0x00];
    assert_eq!(guess_mime(&data), "image/jpeg");
}

#[test]
fn guess_mime_gif_magic() {
    let data = [b'G', b'I', b'F', b'8', b'9', b'a'];
    assert_eq!(guess_mime(&data), "image/gif");
}

#[test]
fn guess_mime_bmp_magic() {
    let data = [b'B', b'M', 0x00, 0x00];
    assert_eq!(guess_mime(&data), "image/bmp");
}

#[test]
fn guess_mime_webp_magic() {
    let mut data = b"RIFF".to_vec();
    data.extend_from_slice(&[0x00u8; 4]);
    data.extend_from_slice(b"WEBP");
    assert_eq!(guess_mime(&data), "image/webp");
}

#[test]
fn guess_mime_unknown_returns_octet_stream() {
    let data = [0x00, 0x01, 0x02, 0x03, 0x04];
    assert_eq!(guess_mime(&data), "application/octet-stream");
}

#[test]
fn guess_mime_too_short_returns_octet_stream() {
    assert_eq!(guess_mime(&[0x89]), "application/octet-stream");
    assert_eq!(guess_mime(&[]), "application/octet-stream");
}

// -----------------------------------------------------------------------
// mime_to_ext
// -----------------------------------------------------------------------

#[test]
fn mime_to_ext_known_types() {
    assert_eq!(mime_to_ext("image/png"), "png");
    assert_eq!(mime_to_ext("image/jpeg"), "jpg");
    assert_eq!(mime_to_ext("image/gif"), "gif");
    assert_eq!(mime_to_ext("image/bmp"), "bmp");
    assert_eq!(mime_to_ext("image/webp"), "webp");
}

#[test]
fn mime_to_ext_unknown_returns_bin() {
    assert_eq!(mime_to_ext("application/octet-stream"), "bin");
    assert_eq!(mime_to_ext("text/plain"), "bin");
}

// -----------------------------------------------------------------------
// detect_heading_level
// -----------------------------------------------------------------------

fn make_para(text: &str, para_shape_id: u16) -> HwpParagraph {
    HwpParagraph {
        text: text.to_string(),
        char_shape_ids: Vec::new(),
        para_shape_id,
        controls: Vec::new(),
    }
}

fn make_para_with_cs(text: &str, cs_id: u16) -> HwpParagraph {
    HwpParagraph {
        text: text.to_string(),
        char_shape_ids: vec![(0, cs_id)],
        para_shape_id: 0,
        controls: Vec::new(),
    }
}

#[test]
fn detect_heading_level_from_para_shape() {
    let mut doc_info = DocInfo::default();
    let mut ps = ParaShape::default();
    ps.heading_type = Some(1); // heading type 1 → level 2
    doc_info.para_shapes.push(ps);

    let para = make_para("A heading", 0);
    assert_eq!(detect_heading_level(&para, &doc_info), Some(2));
}

#[test]
fn detect_heading_level_para_shape_level_0() {
    let mut doc_info = DocInfo::default();
    let mut ps = ParaShape::default();
    ps.heading_type = Some(0); // level 0 → output level 1
    doc_info.para_shapes.push(ps);

    let para = make_para("Top heading", 0);
    assert_eq!(detect_heading_level(&para, &doc_info), Some(1));
}

#[test]
fn detect_heading_level_para_shape_level_6_clamped() {
    let mut doc_info = DocInfo::default();
    let mut ps = ParaShape::default();
    ps.heading_type = Some(6); // level 6 → (6+1).min(6) = 6
    doc_info.para_shapes.push(ps);

    let para = make_para("level6 heading", 0);
    assert_eq!(detect_heading_level(&para, &doc_info), Some(6));
}

#[test]
fn detect_heading_level_para_shape_level_7_rejected() {
    let mut doc_info = DocInfo::default();
    let mut ps = ParaShape::default();
    ps.heading_type = Some(7); // level 7 → not < 7, falls through to font heuristic
    doc_info.para_shapes.push(ps);

    // No char shapes → falls through to None
    let para = make_para("not a heading", 0);
    assert_eq!(detect_heading_level(&para, &doc_info), None);
}

#[test]
fn detect_heading_level_from_char_shape_bold_large_font() {
    let mut doc_info = DocInfo::default();
    // para shape has no heading_type
    doc_info.para_shapes.push(ParaShape::default());
    let mut cs = CharShape::default();
    cs.height = 1600; // >= HEADING1_MIN_HEIGHT
    cs.bold = true;
    doc_info.char_shapes.push(cs);

    let para = make_para_with_cs("Big bold text", 0);
    assert_eq!(detect_heading_level(&para, &doc_info), Some(1));
}

#[test]
fn detect_heading_level_bold_medium_font_returns_h2() {
    let mut doc_info = DocInfo::default();
    doc_info.para_shapes.push(ParaShape::default());
    let mut cs = CharShape::default();
    cs.height = 1400; // >= HEADING2_MIN_HEIGHT but < HEADING1_MIN_HEIGHT
    cs.bold = true;
    doc_info.char_shapes.push(cs);

    let para = make_para_with_cs("Medium bold", 0);
    assert_eq!(detect_heading_level(&para, &doc_info), Some(2));
}

#[test]
fn detect_heading_level_bold_small_font_returns_h3() {
    let mut doc_info = DocInfo::default();
    doc_info.para_shapes.push(ParaShape::default());
    let mut cs = CharShape::default();
    cs.height = 1200; // >= HEADING3_MIN_HEIGHT but < HEADING2_MIN_HEIGHT
    cs.bold = true;
    doc_info.char_shapes.push(cs);

    let para = make_para_with_cs("Small bold", 0);
    assert_eq!(detect_heading_level(&para, &doc_info), Some(3));
}

#[test]
fn detect_heading_level_not_bold_returns_none() {
    let mut doc_info = DocInfo::default();
    doc_info.para_shapes.push(ParaShape::default());
    let mut cs = CharShape::default();
    cs.height = 1600; // large but not bold
    cs.bold = false;
    doc_info.char_shapes.push(cs);

    let para = make_para_with_cs("Large not bold", 0);
    assert_eq!(detect_heading_level(&para, &doc_info), None);
}

#[test]
fn detect_heading_level_long_text_skips_heuristic() {
    let mut doc_info = DocInfo::default();
    doc_info.para_shapes.push(ParaShape::default());
    let mut cs = CharShape::default();
    cs.height = 1600;
    cs.bold = true;
    doc_info.char_shapes.push(cs);

    // Text longer than 100 chars should skip the font heuristic.
    let long_text = "A".repeat(101);
    let para = make_para_with_cs(&long_text, 0);
    assert_eq!(detect_heading_level(&para, &doc_info), None);
}

// -----------------------------------------------------------------------
// build_inlines
// -----------------------------------------------------------------------

#[test]
fn build_inlines_empty_text_returns_empty() {
    let doc_info = DocInfo::default();
    let para = make_para("", 0);
    let inlines = build_inlines(&para, &doc_info);
    assert!(inlines.is_empty());
}

#[test]
fn build_inlines_no_char_shapes_returns_plain_inline() {
    let doc_info = DocInfo::default();
    let para = make_para("Hello world", 0);
    let inlines = build_inlines(&para, &doc_info);
    assert_eq!(inlines.len(), 1);
    assert_eq!(inlines[0].text, "Hello world");
    assert!(!inlines[0].bold);
}

#[test]
fn build_inlines_with_bold_char_shape() {
    let mut doc_info = DocInfo::default();
    let mut cs = CharShape::default();
    cs.bold = true;
    doc_info.char_shapes.push(cs);

    let para = HwpParagraph {
        text: "Bold text".to_string(),
        char_shape_ids: vec![(0, 0)], // position 0, shape 0
        para_shape_id: 0,
        controls: Vec::new(),
    };
    let inlines = build_inlines(&para, &doc_info);
    assert!(!inlines.is_empty());
    assert!(inlines[0].bold);
    assert_eq!(inlines[0].text.trim_end_matches('\r'), "Bold text");
}

#[test]
fn build_inlines_with_italic_char_shape() {
    let mut doc_info = DocInfo::default();
    let mut cs = CharShape::default();
    cs.italic = true;
    doc_info.char_shapes.push(cs);

    let para = HwpParagraph {
        text: "Italic text".to_string(),
        char_shape_ids: vec![(0, 0)],
        para_shape_id: 0,
        controls: Vec::new(),
    };
    let inlines = build_inlines(&para, &doc_info);
    assert!(!inlines.is_empty());
    assert!(inlines[0].italic);
}

#[test]
fn build_inlines_unknown_cs_id_falls_back_to_plain() {
    let doc_info = DocInfo::default(); // no char shapes

    let para = HwpParagraph {
        text: "Plain fallback".to_string(),
        char_shape_ids: vec![(0, 99)], // cs_id 99 doesn't exist
        para_shape_id: 0,
        controls: Vec::new(),
    };
    let inlines = build_inlines(&para, &doc_info);
    assert!(!inlines.is_empty());
    assert!(!inlines[0].bold);
}

#[test]
fn build_inlines_position_past_end_stops() {
    let doc_info = DocInfo::default();
    let para = HwpParagraph {
        text: "Hi".to_string(),
        char_shape_ids: vec![(100, 0)], // position 100 > text length 2
        para_shape_id: 0,
        controls: Vec::new(),
    };
    let inlines = build_inlines(&para, &doc_info);
    // No inline emitted because start >= chars.len()
    assert!(inlines.is_empty());
}

#[test]
fn build_inlines_multiple_segments() {
    let mut doc_info = DocInfo::default();
    let mut cs0 = CharShape::default();
    cs0.bold = true;
    let cs1 = CharShape::default(); // not bold
    doc_info.char_shapes.push(cs0);
    doc_info.char_shapes.push(cs1);

    let para = HwpParagraph {
        text: "BoldNormal".to_string(),
        char_shape_ids: vec![(0, 0), (4, 1)],
        para_shape_id: 0,
        controls: Vec::new(),
    };
    let inlines = build_inlines(&para, &doc_info);
    assert_eq!(inlines.len(), 2);
    assert!(inlines[0].bold);
    assert!(!inlines[1].bold);
}

// -----------------------------------------------------------------------
// hwp_to_ir — basic document conversion
// -----------------------------------------------------------------------

fn make_hwp_document() -> HwpDocument {
    HwpDocument {
        header: FileHeader::default(),
        doc_info: DocInfo::default(),
        sections: Vec::new(),
        bin_data: std::collections::HashMap::new(),
        summary_title: None,
        summary_author: None,
        summary_subject: None,
        summary_keywords: Vec::new(),
    }
}

#[test]
fn hwp_to_ir_empty_document_produces_empty_ir() {
    let hwp = make_hwp_document();
    let doc = hwp_to_ir(&hwp);
    assert!(doc.sections.is_empty());
    assert!(doc.assets.is_empty());
    assert!(doc.metadata.title.is_none());
}

#[test]
fn hwp_to_ir_copies_metadata() {
    let mut hwp = make_hwp_document();
    hwp.summary_title = Some("My Title".into());
    hwp.summary_author = Some("The Author".into());
    hwp.summary_subject = Some("Subject matter".into());
    hwp.summary_keywords = vec!["key1".into(), "key2".into()];

    let doc = hwp_to_ir(&hwp);
    assert_eq!(doc.metadata.title.as_deref(), Some("My Title"));
    assert_eq!(doc.metadata.author.as_deref(), Some("The Author"));
    assert_eq!(doc.metadata.subject.as_deref(), Some("Subject matter"));
    assert_eq!(doc.metadata.keywords, vec!["key1", "key2"]);
}

#[test]
fn hwp_to_ir_paragraph_text_becomes_ir_paragraph() {
    let mut hwp = make_hwp_document();
    hwp.sections.push(HwpSection {
        paragraphs: vec![HwpParagraph {
            text: "Hello world".into(),
            char_shape_ids: Vec::new(),
            para_shape_id: 0,
            controls: Vec::new(),
        }],
    });
    let doc = hwp_to_ir(&hwp);
    assert_eq!(doc.sections.len(), 1);
    assert_eq!(doc.sections[0].blocks.len(), 1);
    if let ir::Block::Paragraph { inlines } = &doc.sections[0].blocks[0] {
        assert_eq!(inlines[0].text, "Hello world");
    } else {
        panic!("Expected Paragraph block");
    }
}

#[test]
fn hwp_to_ir_bin_data_becomes_asset() {
    let mut hwp = make_hwp_document();
    let png_header = vec![0x89u8, b'P', b'N', b'G', 0x00, 0x00, 0x00, 0x00];
    hwp.bin_data.insert(1, png_header.clone());

    let doc = hwp_to_ir(&hwp);
    assert_eq!(doc.assets.len(), 1);
    assert_eq!(doc.assets[0].mime_type, "image/png");
    assert!(doc.assets[0].name.ends_with(".png"));
    assert_eq!(doc.assets[0].data, png_header);
}

#[test]
fn hwp_to_ir_empty_paragraph_text_not_emitted() {
    let mut hwp = make_hwp_document();
    hwp.sections.push(HwpSection {
        paragraphs: vec![HwpParagraph {
            text: "   ".into(), // whitespace only → trimmed to empty
            char_shape_ids: Vec::new(),
            para_shape_id: 0,
            controls: Vec::new(),
        }],
    });
    let doc = hwp_to_ir(&hwp);
    assert_eq!(doc.sections[0].blocks.len(), 0);
}

// -----------------------------------------------------------------------
// control_to_block — ColumnBreak, SectionBreak
// -----------------------------------------------------------------------

#[test]
fn control_to_block_column_break_returns_none() {
    let ctrl = HwpControl::ColumnBreak;
    let doc_info = DocInfo::default();
    assert!(control_to_block(&ctrl, &doc_info).is_none());
}

#[test]
fn control_to_block_section_break_returns_none() {
    let ctrl = HwpControl::SectionBreak;
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

// -----------------------------------------------------------------------
// hwp_to_ir — sequential footnote / endnote IDs (L1 fix)
// -----------------------------------------------------------------------

#[test]
fn hwp_to_ir_two_footnotes_get_sequential_ids() {
    let make_fn_ctrl = || HwpControl::FootnoteEndnote {
        is_endnote: false,
        paragraphs: Vec::new(),
    };
    let mut hwp = make_hwp_document();
    hwp.sections.push(HwpSection {
        paragraphs: vec![
            HwpParagraph {
                text: String::new(),
                char_shape_ids: Vec::new(),
                para_shape_id: 0,
                controls: vec![make_fn_ctrl()],
            },
            HwpParagraph {
                text: String::new(),
                char_shape_ids: Vec::new(),
                para_shape_id: 0,
                controls: vec![make_fn_ctrl()],
            },
        ],
    });
    let doc = hwp_to_ir(&hwp);
    let footnote_ids: Vec<&str> = doc.sections[0]
        .blocks
        .iter()
        .filter_map(|b| {
            if let ir::Block::Footnote { id, .. } = b {
                Some(id.as_str())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(footnote_ids, vec!["footnote-1", "footnote-2"]);
}

#[test]
fn hwp_to_ir_two_endnotes_get_sequential_ids() {
    let make_en_ctrl = || HwpControl::FootnoteEndnote {
        is_endnote: true,
        paragraphs: Vec::new(),
    };
    let mut hwp = make_hwp_document();
    hwp.sections.push(HwpSection {
        paragraphs: vec![
            HwpParagraph {
                text: String::new(),
                char_shape_ids: Vec::new(),
                para_shape_id: 0,
                controls: vec![make_en_ctrl()],
            },
            HwpParagraph {
                text: String::new(),
                char_shape_ids: Vec::new(),
                para_shape_id: 0,
                controls: vec![make_en_ctrl()],
            },
        ],
    });
    let doc = hwp_to_ir(&hwp);
    let endnote_ids: Vec<&str> = doc.sections[0]
        .blocks
        .iter()
        .filter_map(|b| {
            if let ir::Block::Footnote { id, .. } = b {
                Some(id.as_str())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(endnote_ids, vec!["endnote-1", "endnote-2"]);
}

#[test]
fn hwp_to_ir_footnote_and_endnote_counters_are_independent() {
    let mut hwp = make_hwp_document();
    hwp.sections.push(HwpSection {
        paragraphs: vec![
            HwpParagraph {
                text: String::new(),
                char_shape_ids: Vec::new(),
                para_shape_id: 0,
                controls: vec![HwpControl::FootnoteEndnote {
                    is_endnote: false,
                    paragraphs: Vec::new(),
                }],
            },
            HwpParagraph {
                text: String::new(),
                char_shape_ids: Vec::new(),
                para_shape_id: 0,
                controls: vec![HwpControl::FootnoteEndnote {
                    is_endnote: true,
                    paragraphs: Vec::new(),
                }],
            },
            HwpParagraph {
                text: String::new(),
                char_shape_ids: Vec::new(),
                para_shape_id: 0,
                controls: vec![HwpControl::FootnoteEndnote {
                    is_endnote: false,
                    paragraphs: Vec::new(),
                }],
            },
        ],
    });
    let doc = hwp_to_ir(&hwp);
    let ids: Vec<&str> = doc.sections[0]
        .blocks
        .iter()
        .filter_map(|b| {
            if let ir::Block::Footnote { id, .. } = b {
                Some(id.as_str())
            } else {
                None
            }
        })
        .collect();
    // footnote counter and endnote counter are independent sequences.
    assert_eq!(ids, vec!["footnote-1", "endnote-1", "footnote-2"]);
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
// build_inlines — color propagation
// -----------------------------------------------------------------------

#[test]
fn build_inlines_non_black_color_sets_css_hex() {
    let mut doc_info = DocInfo::default();
    let mut cs = CharShape::default();
    // Store red (0x0000FF_00_00 in BGR order: bytes[0]=0x00 blue, [1]=0x00 green, [2]=0xFF red).
    // As a u32 little-endian: r=0xFF, g=0x00, b=0x00 → color = 0x0000_00FF (red in BGR).
    // HWP BGR: byte[0]=blue=0x00, byte[1]=green=0x00, byte[2]=red=0xFF.
    // u32 = (0xFF << 16) | (0x00 << 8) | 0x00 = 0x00FF0000
    cs.color = 0x00FF_0000; // red stored in BGR: bit[23:16]=red=0xFF
    doc_info.char_shapes.push(cs);

    let para = HwpParagraph {
        text: "Red text".to_string(),
        char_shape_ids: vec![(0, 0)],
        para_shape_id: 0,
        controls: Vec::new(),
    };
    let inlines = build_inlines(&para, &doc_info);
    assert_eq!(inlines.len(), 1);
    assert_eq!(inlines[0].color.as_deref(), Some("#FF0000"));
}

#[test]
fn build_inlines_black_color_is_none() {
    let mut doc_info = DocInfo::default();
    let mut cs = CharShape::default();
    cs.color = 0x0000_0000; // black
    doc_info.char_shapes.push(cs);

    let para = HwpParagraph {
        text: "Black text".to_string(),
        char_shape_ids: vec![(0, 0)],
        para_shape_id: 0,
        controls: Vec::new(),
    };
    let inlines = build_inlines(&para, &doc_info);
    assert_eq!(inlines.len(), 1);
    assert!(inlines[0].color.is_none(), "black must not set color");
}

#[test]
fn build_inlines_bgr_green_color_maps_correctly() {
    let mut doc_info = DocInfo::default();
    let mut cs = CharShape::default();
    // Pure green in BGR: byte[0]=0x00 blue, byte[1]=0xFF green, byte[2]=0x00 red.
    // u32 = (0x00 << 16) | (0xFF << 8) | 0x00 = 0x0000_FF00
    cs.color = 0x0000_FF00;
    doc_info.char_shapes.push(cs);

    let para = HwpParagraph {
        text: "Green".to_string(),
        char_shape_ids: vec![(0, 0)],
        para_shape_id: 0,
        controls: Vec::new(),
    };
    let inlines = build_inlines(&para, &doc_info);
    assert_eq!(inlines[0].color.as_deref(), Some("#00FF00"));
}

// -----------------------------------------------------------------------
// build_inlines — face_id → font_name resolution
// -----------------------------------------------------------------------

#[test]
fn build_inlines_face_id_resolves_font_name() {
    let mut doc_info = DocInfo::default();
    doc_info.face_names = vec!["Arial".to_string(), "Batang".to_string()];
    let mut cs = CharShape::default();
    cs.face_id = 1; // index into face_names → "Batang"
    doc_info.char_shapes.push(cs);

    let para = HwpParagraph {
        text: "Korean".to_string(),
        char_shape_ids: vec![(0, 0)],
        para_shape_id: 0,
        controls: Vec::new(),
    };
    let inlines = build_inlines(&para, &doc_info);
    assert_eq!(inlines.len(), 1);
    assert_eq!(inlines[0].font_name.as_deref(), Some("Batang"));
}

#[test]
fn build_inlines_face_id_out_of_bounds_font_name_is_none() {
    let mut doc_info = DocInfo::default();
    // face_names is empty, so any face_id is out of bounds.
    let mut cs = CharShape::default();
    cs.face_id = 5;
    doc_info.char_shapes.push(cs);

    let para = HwpParagraph {
        text: "Text".to_string(),
        char_shape_ids: vec![(0, 0)],
        para_shape_id: 0,
        controls: Vec::new(),
    };
    let inlines = build_inlines(&para, &doc_info);
    assert!(inlines[0].font_name.is_none());
}
