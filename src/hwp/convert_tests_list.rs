//! Tests for HWP binary list detection and grouping.
//!
//! Tests cover:
//! 1. `detect_list_kind` — per-paragraph classification
//! 2. `detect_list_kind_from_text` (via `detect_list_kind`) — text heuristics
//! 3. `hwp_to_ir` integration — grouping of consecutive list paragraphs

use super::{detect_list_kind, hwp_to_ir, ListKind};
use crate::hwp::model::{DocInfo, FileHeader, HwpDocument, HwpParagraph, HwpSection, ParaShape};
use crate::ir;
use std::collections::HashMap;

// -- helpers ------------------------------------------------------------------

fn make_para(text: &str) -> HwpParagraph {
    HwpParagraph {
        text: text.to_string(),
        char_shape_ids: Vec::new(),
        para_shape_id: 0,
        controls: Vec::new(),
        raw_para_text: None,
    }
}

fn make_para_with_ps(text: &str, ps_id: u16) -> HwpParagraph {
    HwpParagraph {
        text: text.to_string(),
        char_shape_ids: Vec::new(),
        para_shape_id: ps_id,
        controls: Vec::new(),
        raw_para_text: None,
    }
}

fn doc_info_empty() -> DocInfo {
    DocInfo::default()
}

/// `DocInfo` with `ParaShape[0].numbering_id = Some(1)`.
fn doc_info_with_numbering() -> DocInfo {
    let mut di = DocInfo::default();
    di.para_shapes.push(ParaShape {
        numbering_id: Some(1),
        ..Default::default()
    });
    di
}

fn make_hwp_doc(paras: Vec<HwpParagraph>) -> HwpDocument {
    HwpDocument {
        header: FileHeader::default(),
        doc_info: DocInfo::default(),
        sections: vec![HwpSection { paragraphs: paras }],
        bin_data: HashMap::new(),
        summary_title: None,
        summary_author: None,
        summary_subject: None,
        summary_keywords: Vec::new(),
    }
}

fn make_hwp_doc_with_di(doc_info: DocInfo, paras: Vec<HwpParagraph>) -> HwpDocument {
    HwpDocument {
        header: FileHeader::default(),
        doc_info,
        sections: vec![HwpSection { paragraphs: paras }],
        bin_data: HashMap::new(),
        summary_title: None,
        summary_author: None,
        summary_subject: None,
        summary_keywords: Vec::new(),
    }
}

// -- detect_list_kind: bullet characters (Tier 2 heuristic) ------------------

#[test]
fn detect_list_kind_bullet_circle_is_unordered() {
    let para = make_para("● First item");
    assert_eq!(
        detect_list_kind(&para, &doc_info_empty()),
        Some(ListKind::Unordered)
    );
}

#[test]
fn detect_list_kind_bullet_square_is_unordered() {
    let para = make_para("■ Second item");
    assert_eq!(
        detect_list_kind(&para, &doc_info_empty()),
        Some(ListKind::Unordered)
    );
}

#[test]
fn detect_list_kind_bullet_triangle_is_unordered() {
    let para = make_para("▶ Arrow item");
    assert_eq!(
        detect_list_kind(&para, &doc_info_empty()),
        Some(ListKind::Unordered)
    );
}

#[test]
fn detect_list_kind_bullet_dot_is_unordered() {
    let para = make_para("• Dot item");
    assert_eq!(
        detect_list_kind(&para, &doc_info_empty()),
        Some(ListKind::Unordered)
    );
}

#[test]
fn detect_list_kind_bullet_dash_is_unordered() {
    let para = make_para("- Dash item");
    assert_eq!(
        detect_list_kind(&para, &doc_info_empty()),
        Some(ListKind::Unordered)
    );
}

#[test]
fn detect_list_kind_bullet_star_is_unordered() {
    let para = make_para("* Star item");
    assert_eq!(
        detect_list_kind(&para, &doc_info_empty()),
        Some(ListKind::Unordered)
    );
}

// -- detect_list_kind: numbered prefixes (Tier 2 heuristic) ------------------

#[test]
fn detect_list_kind_number_dot_space_is_ordered() {
    let para = make_para("1. First item");
    assert_eq!(
        detect_list_kind(&para, &doc_info_empty()),
        Some(ListKind::Ordered)
    );
}

#[test]
fn detect_list_kind_number_paren_space_is_ordered() {
    let para = make_para("2) Second item");
    assert_eq!(
        detect_list_kind(&para, &doc_info_empty()),
        Some(ListKind::Ordered)
    );
}

#[test]
fn detect_list_kind_alpha_dot_space_is_ordered() {
    let para = make_para("a. Alpha item");
    assert_eq!(
        detect_list_kind(&para, &doc_info_empty()),
        Some(ListKind::Ordered)
    );
}

#[test]
fn detect_list_kind_alpha_paren_space_is_ordered() {
    let para = make_para("b) Beta item");
    assert_eq!(
        detect_list_kind(&para, &doc_info_empty()),
        Some(ListKind::Ordered)
    );
}

#[test]
fn detect_list_kind_three_digit_number_is_ordered() {
    let para = make_para("100. Centesimal item");
    assert_eq!(
        detect_list_kind(&para, &doc_info_empty()),
        Some(ListKind::Ordered)
    );
}

// -- detect_list_kind: plain paragraph -- no detection -----------------------

#[test]
fn detect_list_kind_plain_paragraph_returns_none() {
    let para = make_para("This is a normal paragraph.");
    assert_eq!(detect_list_kind(&para, &doc_info_empty()), None);
}

#[test]
fn detect_list_kind_empty_paragraph_returns_none() {
    let para = make_para("");
    assert_eq!(detect_list_kind(&para, &doc_info_empty()), None);
}

#[test]
fn detect_list_kind_whitespace_only_returns_none() {
    let para = make_para("   ");
    assert_eq!(detect_list_kind(&para, &doc_info_empty()), None);
}

// -- edge cases: year-like patterns must NOT match ----------------------------

#[test]
fn detect_list_kind_year_number_is_not_ordered() {
    // 4-digit number followed by a non-separator character must NOT match.
    let para = make_para("2026 business plan");
    assert_eq!(detect_list_kind(&para, &doc_info_empty()), None);
}

#[test]
fn detect_list_kind_four_digit_no_separator_is_not_ordered() {
    let para = make_para("1234 abcd");
    assert_eq!(detect_list_kind(&para, &doc_info_empty()), None);
}

#[test]
fn detect_list_kind_number_without_space_after_dot_is_not_ordered() {
    // "1.Second" -- no space after dot.
    let para = make_para("1.Second");
    assert_eq!(detect_list_kind(&para, &doc_info_empty()), None);
}

#[test]
fn detect_list_kind_bullet_without_trailing_space_is_not_list() {
    // Bullet directly followed by text, no separator space.
    let para = make_para("●text without space");
    assert_eq!(detect_list_kind(&para, &doc_info_empty()), None);
}

// -- Tier 1: binary numbering_id ---------------------------------------------

#[test]
fn detect_list_kind_numbering_id_with_bullet_text_is_unordered() {
    let di = doc_info_with_numbering();
    let para = make_para_with_ps("● Formal bullet", 0);
    assert_eq!(detect_list_kind(&para, &di), Some(ListKind::Unordered));
}

#[test]
fn detect_list_kind_numbering_id_with_ordered_text_is_ordered() {
    let di = doc_info_with_numbering();
    let para = make_para_with_ps("1. Formal numbered", 0);
    assert_eq!(detect_list_kind(&para, &di), Some(ListKind::Ordered));
}

#[test]
fn detect_list_kind_numbering_id_with_plain_text_defaults_to_unordered() {
    // numbering_id set but text has no recognisable prefix.
    let di = doc_info_with_numbering();
    let para = make_para_with_ps("Plain text in a formal list", 0);
    assert_eq!(detect_list_kind(&para, &di), Some(ListKind::Unordered));
}

#[test]
fn detect_list_kind_no_numbering_id_plain_text_returns_none() {
    let di = doc_info_with_numbering();
    // para_shape_id = 1 but doc_info only has one shape at index 0.
    let para = make_para_with_ps("Regular paragraph", 1);
    assert_eq!(detect_list_kind(&para, &di), None);
}

// -- hwp_to_ir: grouping consecutive list paragraphs -------------------------

#[test]
fn hwp_to_ir_bullet_paragraph_produces_list_block() {
    let hwp = make_hwp_doc(vec![make_para("- Single bullet")]);
    let doc = hwp_to_ir(&hwp);
    assert_eq!(doc.sections[0].blocks.len(), 1);
    assert!(
        matches!(
            doc.sections[0].blocks[0],
            ir::Block::List { ordered: false, .. }
        ),
        "expected unordered Block::List, got {:?}",
        doc.sections[0].blocks[0]
    );
}

#[test]
fn hwp_to_ir_numbered_paragraph_produces_ordered_list_block() {
    let hwp = make_hwp_doc(vec![make_para("1. Single item")]);
    let doc = hwp_to_ir(&hwp);
    assert_eq!(doc.sections[0].blocks.len(), 1);
    assert!(
        matches!(
            doc.sections[0].blocks[0],
            ir::Block::List { ordered: true, .. }
        ),
        "expected ordered Block::List, got {:?}",
        doc.sections[0].blocks[0]
    );
}

#[test]
fn hwp_to_ir_consecutive_bullets_produce_single_list() {
    let hwp = make_hwp_doc(vec![
        make_para("- Item one"),
        make_para("- Item two"),
        make_para("- Item three"),
    ]);
    let doc = hwp_to_ir(&hwp);
    assert_eq!(
        doc.sections[0].blocks.len(),
        1,
        "three consecutive bullet paragraphs must collapse into one Block::List"
    );
    if let ir::Block::List { ordered, items, .. } = &doc.sections[0].blocks[0] {
        assert!(!ordered, "must be unordered");
        assert_eq!(items.len(), 3);
    } else {
        panic!("expected Block::List");
    }
}

#[test]
fn hwp_to_ir_consecutive_numbered_produce_single_list() {
    let hwp = make_hwp_doc(vec![
        make_para("1. First"),
        make_para("2. Second"),
        make_para("3. Third"),
    ]);
    let doc = hwp_to_ir(&hwp);
    assert_eq!(doc.sections[0].blocks.len(), 1);
    if let ir::Block::List { ordered, items, .. } = &doc.sections[0].blocks[0] {
        assert!(*ordered, "must be ordered");
        assert_eq!(items.len(), 3);
    } else {
        panic!("expected Block::List");
    }
}

#[test]
fn hwp_to_ir_mixed_bullet_numbered_produces_two_lists() {
    let hwp = make_hwp_doc(vec![
        make_para("- Bullet A"),
        make_para("- Bullet B"),
        make_para("1. Number one"),
        make_para("2. Number two"),
    ]);
    let doc = hwp_to_ir(&hwp);
    assert_eq!(
        doc.sections[0].blocks.len(),
        2,
        "bullet list then numbered list must produce 2 Block::List values"
    );
    assert!(matches!(
        doc.sections[0].blocks[0],
        ir::Block::List { ordered: false, .. }
    ));
    assert!(matches!(
        doc.sections[0].blocks[1],
        ir::Block::List { ordered: true, .. }
    ));
}

#[test]
fn hwp_to_ir_plain_paragraph_between_lists_produces_three_blocks() {
    let hwp = make_hwp_doc(vec![
        make_para("- Bullet A"),
        make_para("- Bullet B"),
        make_para("Regular paragraph here."),
        make_para("- Bullet C"),
        make_para("- Bullet D"),
    ]);
    let doc = hwp_to_ir(&hwp);
    assert_eq!(doc.sections[0].blocks.len(), 3);
    assert!(matches!(doc.sections[0].blocks[0], ir::Block::List { .. }));
    assert!(matches!(
        doc.sections[0].blocks[1],
        ir::Block::Paragraph { .. }
    ));
    assert!(matches!(doc.sections[0].blocks[2], ir::Block::List { .. }));
}

#[test]
fn hwp_to_ir_regular_paragraph_not_detected_as_list() {
    let hwp = make_hwp_doc(vec![make_para("This is a normal sentence.")]);
    let doc = hwp_to_ir(&hwp);
    assert_eq!(doc.sections[0].blocks.len(), 1);
    assert!(
        matches!(doc.sections[0].blocks[0], ir::Block::Paragraph { .. }),
        "plain text must remain a Paragraph"
    );
}

#[test]
fn hwp_to_ir_year_paragraph_not_detected_as_list() {
    // 4-digit number followed by non-separator text must NOT become a list.
    let hwp = make_hwp_doc(vec![make_para("2026 business plan")]);
    let doc = hwp_to_ir(&hwp);
    assert_eq!(doc.sections[0].blocks.len(), 1);
    assert!(
        matches!(doc.sections[0].blocks[0], ir::Block::Paragraph { .. }),
        "year-like number prefix must remain a Paragraph, not become a list"
    );
}

#[test]
fn hwp_to_ir_heading_not_treated_as_list_item() {
    // Paragraph with heading_type = Some(0) must become a Heading, not a list.
    let mut di = DocInfo::default();
    di.para_shapes.push(ParaShape {
        heading_type: Some(0),
        ..Default::default()
    });
    // Even if text starts with "- " it should remain a Heading.
    let hwp = make_hwp_doc_with_di(di, vec![make_para_with_ps("- Heading text", 0)]);
    let doc = hwp_to_ir(&hwp);
    assert_eq!(doc.sections[0].blocks.len(), 1);
    assert!(
        matches!(doc.sections[0].blocks[0], ir::Block::Heading { .. }),
        "heading paragraph must remain a Heading even if text starts with bullet"
    );
}

#[test]
fn hwp_to_ir_binary_numbering_id_groups_plain_text_into_list() {
    // Paragraphs with numbering_id set (formal binary list) must become a
    // Block::List even when the text has no bullet/number prefix.
    let di = doc_info_with_numbering();
    let hwp = make_hwp_doc_with_di(
        di,
        vec![
            make_para_with_ps("First formal item", 0),
            make_para_with_ps("Second formal item", 0),
        ],
    );
    let doc = hwp_to_ir(&hwp);
    assert_eq!(doc.sections[0].blocks.len(), 1);
    if let ir::Block::List { items, .. } = &doc.sections[0].blocks[0] {
        assert_eq!(items.len(), 2);
    } else {
        panic!("expected Block::List for formal binary list paragraphs");
    }
}
