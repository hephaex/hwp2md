use super::*;

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

fn make_para(text: &str, para_shape_id: u16) -> HwpParagraph {
    HwpParagraph {
        text: text.to_string(),
        char_shape_ids: Vec::new(),
        para_shape_id,
        controls: Vec::new(),
        raw_para_text: None,
    }
}

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

// -----------------------------------------------------------------------
// hwp_to_ir — basic document conversion
// -----------------------------------------------------------------------

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
            raw_para_text: None,
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
            raw_para_text: None,
        }],
    });
    let doc = hwp_to_ir(&hwp);
    assert_eq!(doc.sections[0].blocks.len(), 0);
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
                raw_para_text: None,
            },
            HwpParagraph {
                text: String::new(),
                char_shape_ids: Vec::new(),
                para_shape_id: 0,
                controls: vec![make_fn_ctrl()],
                raw_para_text: None,
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
                raw_para_text: None,
            },
            HwpParagraph {
                text: String::new(),
                char_shape_ids: Vec::new(),
                para_shape_id: 0,
                controls: vec![make_en_ctrl()],
                raw_para_text: None,
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
                raw_para_text: None,
            },
            HwpParagraph {
                text: String::new(),
                char_shape_ids: Vec::new(),
                para_shape_id: 0,
                controls: vec![HwpControl::FootnoteEndnote {
                    is_endnote: true,
                    paragraphs: Vec::new(),
                }],
                raw_para_text: None,
            },
            HwpParagraph {
                text: String::new(),
                char_shape_ids: Vec::new(),
                para_shape_id: 0,
                controls: vec![HwpControl::FootnoteEndnote {
                    is_endnote: false,
                    paragraphs: Vec::new(),
                }],
                raw_para_text: None,
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

// -----------------------------------------------------------------------
// hwp_to_ir — heading detection uses make_para helper
// -----------------------------------------------------------------------

#[test]
fn hwp_to_ir_heading_para_shape_becomes_heading_block() {
    let mut hwp = make_hwp_document();
    // Insert a para_shape with heading_type = Some(0) at index 0.
    hwp.doc_info.para_shapes.push(ParaShape {
        heading_type: Some(0),
        ..Default::default()
    });
    hwp.sections.push(HwpSection {
        paragraphs: vec![make_para("Top Heading", 0)],
    });
    let doc = hwp_to_ir(&hwp);
    assert_eq!(doc.sections[0].blocks.len(), 1);
    if let ir::Block::Heading { level, inlines } = &doc.sections[0].blocks[0] {
        assert_eq!(*level, 1);
        assert_eq!(inlines[0].text, "Top Heading");
    } else {
        panic!(
            "expected Heading block, got {:?}",
            doc.sections[0].blocks[0]
        );
    }
}
