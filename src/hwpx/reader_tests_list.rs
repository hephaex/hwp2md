/// Tests for OWPML flat-paragraph list detection and nested list grouping.
///
/// OWPML encodes lists as **flat** `<hp:p>` paragraphs with `paraPrIDRef` and
/// optional `numPrIDRef` attributes rather than nested `<ol>/<ul>/<li>` elements.
/// The reader must recognise these attributes and fold consecutive list-paragraph
/// sentinels into proper `Block::List { items }` structures.
///
/// Attribute conventions (matching writer_header.rs constants):
/// - `paraPrIDRef="2"` (PARA_PR_LIST_D0) → depth-0 list item
/// - `paraPrIDRef="3"` (PARA_PR_LIST_D1) → depth-1 nested list item
/// - `numPrIDRef="1"`  (NUM_PR_DIGIT)    → ordered list (absent = unordered)
use super::*;

// ── helpers ─────────────────────────────────────────────────────────────────

/// Parse section XML and panic with a descriptive message on failure.
fn section(xml: &str) -> ir::Section {
    parse_section_xml(xml).expect("parse_section_xml must not fail")
}

/// Produce a `<hp:p>` XML fragment for a list paragraph.
///
/// - `para_pr`  — `paraPrIDRef` value ("2" = depth 0, "3" = depth 1)
/// - `num_pr`   — optional `numPrIDRef` value ("1" = ordered; omit for bullet)
/// - `text`     — visible text content
fn list_para(para_pr: &str, num_pr: Option<&str>, text: &str) -> String {
    let num_attr = num_pr
        .map(|v| format!(r#" numPrIDRef="{v}""#))
        .unwrap_or_default();
    format!(
        r#"<hp:p paraPrIDRef="{para_pr}"{num_attr}><hp:run><hp:t>{text}</hp:t></hp:run></hp:p>"#
    )
}

/// Produce a plain paragraph XML fragment (no list attributes).
fn plain_para(text: &str) -> String {
    format!(r#"<hp:p><hp:run><hp:t>{text}</hp:t></hp:run></hp:p>"#)
}

/// Extract all top-level text from the first block if it is a `Block::List`.
fn list_item_texts(block: &ir::Block) -> Vec<String> {
    match block {
        ir::Block::List { items, .. } => items
            .iter()
            .map(|item| {
                item.blocks
                    .iter()
                    .filter_map(|b| match b {
                        ir::Block::Paragraph { inlines } => {
                            Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("")
            })
            .collect(),
        other => panic!("expected Block::List, got {other:?}"),
    }
}

// ── tests: unordered flat list ───────────────────────────────────────────────

#[test]
fn flat_unordered_list_produces_list_block() {
    // Three depth-0 paragraphs without numPrIDRef → unordered Block::List.
    let xml = format!(
        "<root>{}{}{}</root>",
        list_para("2", None, "alpha"),
        list_para("2", None, "beta"),
        list_para("2", None, "gamma"),
    );
    let s = section(&xml);
    assert_eq!(
        s.blocks.len(),
        1,
        "three list paras must produce exactly one Block::List"
    );
    match &s.blocks[0] {
        ir::Block::List { ordered, items, .. } => {
            assert!(!ordered, "bullet list must be unordered");
            assert_eq!(items.len(), 3);
            let texts = list_item_texts(&s.blocks[0]);
            assert_eq!(texts, ["alpha", "beta", "gamma"]);
        }
        other => panic!("expected Block::List, got {other:?}"),
    }
}

// ── tests: ordered flat list ─────────────────────────────────────────────────

#[test]
fn flat_ordered_list_produces_ordered_list_block() {
    // Three depth-0 paragraphs with numPrIDRef="1" → ordered Block::List.
    let xml = format!(
        "<root>{}{}{}</root>",
        list_para("2", Some("1"), "first"),
        list_para("2", Some("1"), "second"),
        list_para("2", Some("1"), "third"),
    );
    let s = section(&xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::List {
            ordered,
            items,
            start,
            ..
        } => {
            assert!(*ordered, "numPrIDRef=1 list must be ordered");
            assert_eq!(*start, 1);
            assert_eq!(items.len(), 3);
            let texts = list_item_texts(&s.blocks[0]);
            assert_eq!(texts, ["first", "second", "third"]);
        }
        other => panic!("expected Block::List, got {other:?}"),
    }
}

// ── tests: single-item list edge case ────────────────────────────────────────

#[test]
fn single_item_unordered_list() {
    let xml = format!("<root>{}</root>", list_para("2", None, "only"));
    let s = section(&xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::List { ordered, items, .. } => {
            assert!(!ordered);
            assert_eq!(items.len(), 1);
            assert_eq!(list_item_texts(&s.blocks[0]), ["only"]);
        }
        other => panic!("expected Block::List, got {other:?}"),
    }
}

// ── tests: nested list (2 levels) ────────────────────────────────────────────

#[test]
fn nested_list_two_levels_produces_children() {
    // Depth-0 item, then two depth-1 items, then another depth-0 item.
    let xml = format!(
        "<root>{}{}{}{}</root>",
        list_para("2", None, "parent-a"),
        list_para("3", None, "child-a1"),
        list_para("3", None, "child-a2"),
        list_para("2", None, "parent-b"),
    );
    let s = section(&xml);
    assert_eq!(
        s.blocks.len(),
        1,
        "four list paras must collapse into one Block::List"
    );
    match &s.blocks[0] {
        ir::Block::List { ordered, items, .. } => {
            assert!(!ordered);
            assert_eq!(items.len(), 2, "must have 2 top-level items");

            // First top-level item must have 2 children.
            let first = &items[0];
            assert_eq!(first.children.len(), 2, "first item must have 2 children");
            let child_texts: Vec<String> = first
                .children
                .iter()
                .map(|c| {
                    c.blocks
                        .iter()
                        .filter_map(|b| match b {
                            ir::Block::Paragraph { inlines } => {
                                Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
                            }
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("")
                })
                .collect();
            assert_eq!(child_texts, ["child-a1", "child-a2"]);

            // Second top-level item must have no children.
            let second = &items[1];
            assert!(
                second.children.is_empty(),
                "second item must have no children"
            );
        }
        other => panic!("expected Block::List, got {other:?}"),
    }
}

// ── tests: mixed ordered/unordered at different depths ───────────────────────

#[test]
fn ordered_list_with_unordered_children() {
    // Top-level ordered, children unordered (no numPrIDRef at depth 1).
    let xml = format!(
        "<root>{}{}{}</root>",
        list_para("2", Some("1"), "ordered-parent"),
        list_para("3", None, "unordered-child-1"),
        list_para("3", None, "unordered-child-2"),
    );
    let s = section(&xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::List { ordered, items, .. } => {
            assert!(
                *ordered,
                "top-level list must be ordered (set by first item)"
            );
            assert_eq!(items.len(), 1, "one top-level item");
            assert_eq!(items[0].children.len(), 2, "two children");
        }
        other => panic!("expected Block::List, got {other:?}"),
    }
}

// ── tests: list interrupted by plain paragraph ───────────────────────────────

#[test]
fn list_followed_by_paragraph_produces_two_blocks() {
    let xml = format!(
        "<root>{}{}{}</root>",
        list_para("2", None, "item-a"),
        list_para("2", None, "item-b"),
        plain_para("after list"),
    );
    let s = section(&xml);
    assert_eq!(s.blocks.len(), 2, "list + paragraph must produce 2 blocks");
    match &s.blocks[0] {
        ir::Block::List { items, .. } => {
            assert_eq!(items.len(), 2);
        }
        other => panic!("expected Block::List first, got {other:?}"),
    }
    match &s.blocks[1] {
        ir::Block::Paragraph { inlines } => {
            assert_eq!(inlines[0].text, "after list");
        }
        other => panic!("expected Paragraph second, got {other:?}"),
    }
}

#[test]
fn paragraph_before_list_produces_two_blocks() {
    let xml = format!(
        "<root>{}{}{}</root>",
        plain_para("before list"),
        list_para("2", None, "item-a"),
        list_para("2", None, "item-b"),
    );
    let s = section(&xml);
    assert_eq!(s.blocks.len(), 2);
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            assert_eq!(inlines[0].text, "before list");
        }
        other => panic!("expected Paragraph first, got {other:?}"),
    }
    match &s.blocks[1] {
        ir::Block::List { items, .. } => {
            assert_eq!(items.len(), 2);
        }
        other => panic!("expected Block::List second, got {other:?}"),
    }
}

#[test]
fn plain_paragraph_between_two_lists_produces_three_blocks() {
    // list A · plain para · list B → 3 separate blocks.
    let xml = format!(
        "<root>{}{}{}{}{}</root>",
        list_para("2", None, "list-a-1"),
        list_para("2", None, "list-a-2"),
        plain_para("separator"),
        list_para("2", None, "list-b-1"),
        list_para("2", None, "list-b-2"),
    );
    let s = section(&xml);
    assert_eq!(
        s.blocks.len(),
        3,
        "two separate lists separated by a paragraph must give 3 blocks"
    );
    assert!(matches!(s.blocks[0], ir::Block::List { .. }));
    assert!(matches!(s.blocks[1], ir::Block::Paragraph { .. }));
    assert!(matches!(s.blocks[2], ir::Block::List { .. }));
}

// ── tests: headings are NOT list items ───────────────────────────────────────

#[test]
fn heading_with_para_pr_id_is_not_a_list_item() {
    // Even if a heading paragraph somehow carries paraPrIDRef="2", it must
    // be emitted as a Heading, not a list item.
    let xml = r#"<root>
        <hp:p styleIDRef="2" paraPrIDRef="2">
            <hp:run><hp:t>Chapter</hp:t></hp:run>
        </hp:p>
    </root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::Heading { level, inlines } => {
            assert_eq!(*level, 2);
            assert_eq!(inlines[0].text.trim(), "Chapter");
        }
        other => panic!("expected Heading, got {other:?}"),
    }
}

// ── tests: depth-1 item appearing before any depth-0 item ───────────────────

#[test]
fn orphan_depth1_item_promoted_to_top_level() {
    // A depth-1 item with no preceding depth-0 item must be promoted
    // defensively to avoid a panic.
    let xml = format!("<root>{}</root>", list_para("3", None, "orphan-child"),);
    let s = section(&xml);
    assert_eq!(
        s.blocks.len(),
        1,
        "orphan depth-1 item must still produce a list block"
    );
    match &s.blocks[0] {
        ir::Block::List { items, .. } => {
            assert_eq!(items.len(), 1);
            // The orphan is promoted to a top-level item with no children.
            assert!(items[0].children.is_empty());
        }
        other => panic!("expected Block::List, got {other:?}"),
    }
}

// ── tests: group_list_paragraphs unit tests ───────────────────────────────────

#[test]
fn group_list_paragraphs_empty_input_produces_empty_output() {
    use super::context::group_list_paragraphs;
    let result = group_list_paragraphs(vec![]);
    assert!(result.is_empty());
}

#[test]
fn group_list_paragraphs_plain_only_passes_through() {
    use super::context::{group_list_paragraphs, StagedBlock};
    let para = ir::Block::Paragraph {
        inlines: vec![ir::Inline::plain("hello".to_string())],
    };
    let result = group_list_paragraphs(vec![StagedBlock::Plain(para)]);
    assert_eq!(result.len(), 1);
    assert!(matches!(result[0], ir::Block::Paragraph { .. }));
}

#[test]
fn group_list_paragraphs_consecutive_list_paras_collapsed() {
    use super::context::{group_list_paragraphs, StagedBlock};
    let make_para = |text: &str| ir::Block::Paragraph {
        inlines: vec![ir::Inline::plain(text.to_string())],
    };
    let staged = vec![
        StagedBlock::ListPara {
            depth: 0,
            ordered: false,
            block: make_para("a"),
        },
        StagedBlock::ListPara {
            depth: 0,
            ordered: false,
            block: make_para("b"),
        },
        StagedBlock::ListPara {
            depth: 0,
            ordered: false,
            block: make_para("c"),
        },
    ];
    let result = group_list_paragraphs(staged);
    assert_eq!(result.len(), 1);
    match &result[0] {
        ir::Block::List { items, ordered, .. } => {
            assert!(!ordered);
            assert_eq!(items.len(), 3);
        }
        other => panic!("expected Block::List, got {other:?}"),
    }
}

// ── tests: roundtrip (MD → HWPX → MD) ────────────────────────────────────────

/// Write a minimal document with a nested list to an HWPX file, re-read it,
/// and verify the resulting section contains a properly structured `Block::List`.
///
/// This exercises both the writer's OWPML list emission and the reader's
/// flat-paragraph grouping.
#[test]
fn roundtrip_nested_list_md_to_hwpx_to_md() {
    use crate::hwpx::write_hwpx;
    use crate::ir::{Block, Document, Inline, ListItem, Metadata, Section};

    // Build an IR document with a 2-level nested list.
    let inner_item = ListItem {
        blocks: vec![Block::Paragraph {
            inlines: vec![Inline::plain("nested item".to_string())],
        }],
        children: vec![],
    };
    let outer_item = ListItem {
        blocks: vec![Block::Paragraph {
            inlines: vec![Inline::plain("top item".to_string())],
        }],
        children: vec![inner_item],
    };
    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::List {
                ordered: false,
                start: 1,
                items: vec![outer_item],
            }],
        }],
        assets: vec![],
    };

    // Write to a temp HWPX file.
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx must succeed");

    // Open the ZIP archive and read XML entries.
    let file = std::fs::File::open(tmp.path()).expect("open HWPX zip");
    let mut zip_archive = zip::ZipArchive::new(file).expect("must open ZIP");

    // Read the face names from header.xml (try both paths).
    let header_xml = {
        let name = if zip_archive.by_name("Contents/header.xml").is_ok() {
            "Contents/header.xml"
        } else {
            "header.xml"
        };
        let mut entry = zip_archive
            .by_name(name)
            .expect("header.xml must exist in output HWPX");
        let mut s = String::new();
        std::io::Read::read_to_string(&mut entry, &mut s).expect("header.xml must be valid UTF-8");
        s
    };
    let face_names = parse_face_names(&header_xml);

    // Read section0.xml (try both capitalizations).
    let section_xml = {
        let name = if zip_archive.by_name("Contents/section0.xml").is_ok() {
            "Contents/section0.xml"
        } else {
            "Contents/Section0.xml"
        };
        let mut entry = zip_archive
            .by_name(name)
            .expect("section0.xml must exist in output HWPX");
        let mut s = String::new();
        std::io::Read::read_to_string(&mut entry, &mut s)
            .expect("section0.xml must be valid UTF-8");
        s
    };

    let section = parse_section_xml_with_face_names(&section_xml, &face_names)
        .expect("parse_section_xml_with_face_names must succeed");

    // The re-read section must contain a Block::List.
    assert!(
        !section.blocks.is_empty(),
        "roundtrip section must not be empty"
    );
    match &section.blocks[0] {
        ir::Block::List { ordered, items, .. } => {
            assert!(!ordered, "list must be unordered");
            assert!(!items.is_empty(), "list must have at least one item");
            let first = &items[0];

            // The top-level item must carry "top item" text.
            let top_text: String = first
                .blocks
                .iter()
                .filter_map(|b| match b {
                    ir::Block::Paragraph { inlines } => {
                        Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
                    }
                    _ => None,
                })
                .collect();
            assert!(
                top_text.contains("top item"),
                "top-level item must contain 'top item'; got: {top_text:?}"
            );

            // The nested item must appear as a child.
            assert!(
                !first.children.is_empty(),
                "top-level item must have at least one child after roundtrip"
            );
            let child_text: String = first.children[0]
                .blocks
                .iter()
                .filter_map(|b| match b {
                    ir::Block::Paragraph { inlines } => {
                        Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
                    }
                    _ => None,
                })
                .collect();
            assert!(
                child_text.contains("nested item"),
                "child item must contain 'nested item'; got: {child_text:?}"
            );
        }
        other => panic!("expected Block::List after roundtrip, got {other:?}"),
    }
}
