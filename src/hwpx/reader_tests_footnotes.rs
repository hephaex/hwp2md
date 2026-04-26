use super::*;
use std::collections::HashMap;

// -----------------------------------------------------------------------
// Helper: unwrap the section and panic with a descriptive message on error.
// -----------------------------------------------------------------------

fn section(xml: &str) -> ir::Section {
    parse_section_xml(xml).expect("parse_section_xml must not fail")
}

// -----------------------------------------------------------------------
// parse_section_xml -- footnote / endnote parsing
// -----------------------------------------------------------------------

fn first_footnote(s: &ir::Section) -> (&str, &[ir::Block]) {
    match &s.blocks[0] {
        ir::Block::Footnote { id, content } => (id.as_str(), content.as_slice()),
        other => panic!("expected Block::Footnote, got {other:?}"),
    }
}

#[test]
fn footnote_produces_footnote_block() {
    let xml = r#"<root><hp:fn id="1"><hp:p><hp:run><hp:t>note text</hp:t></hp:run></hp:p></hp:fn></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1, "one footnote block expected");
    let (id, content) = first_footnote(&s);
    assert_eq!(id, "1");
    assert_eq!(
        content.len(),
        1,
        "footnote must have exactly one inner block"
    );
    match &content[0] {
        ir::Block::Paragraph { inlines } => {
            assert_eq!(inlines[0].text, "note text");
        }
        other => panic!("expected Paragraph inside footnote, got {other:?}"),
    }
}

#[test]
fn endnote_produces_footnote_block() {
    let xml =
        r#"<root><hp:en id="2"><hp:p><hp:run><hp:t>end note</hp:t></hp:run></hp:p></hp:en></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    let (id, content) = first_footnote(&s);
    assert_eq!(id, "2");
    match &content[0] {
        ir::Block::Paragraph { inlines } => {
            assert_eq!(inlines[0].text, "end note");
        }
        other => panic!("expected Paragraph inside endnote block, got {other:?}"),
    }
}

#[test]
fn footnote_alt_tag_name() {
    let xml = r#"<root><hp:footnote id="3"><hp:p><hp:run><hp:t>alt tag</hp:t></hp:run></hp:p></hp:footnote></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    let (id, content) = first_footnote(&s);
    assert_eq!(id, "3");
    match &content[0] {
        ir::Block::Paragraph { inlines } => {
            assert_eq!(inlines[0].text, "alt tag");
        }
        other => panic!("expected Paragraph inside footnote (alt tag), got {other:?}"),
    }
}

#[test]
fn note_ref_produces_footnote_ref_inline() {
    // <hp:noteRef noteId="1"/> produces an Inline with footnote_ref set and empty text.
    let xml = r#"<root><hp:p><hp:noteRef noteId="1"/></hp:p></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1, "one paragraph block expected");
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            assert_eq!(inlines.len(), 1, "one inline expected");
            assert_eq!(
                inlines[0].footnote_ref.as_deref(),
                Some("1"),
                "inline must carry footnote_ref=\"1\""
            );
            assert!(
                inlines[0].text.is_empty(),
                "footnote_ref inline must have empty text"
            );
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

#[test]
fn empty_footnote_ignored() {
    let xml = r#"<root><hp:fn id="1"></hp:fn></root>"#;
    let s = section(xml);
    assert!(
        s.blocks.is_empty(),
        "empty footnote must not produce a Block::Footnote"
    );
}

// -----------------------------------------------------------------------
// Cross-cutting: context x element combinations
// -----------------------------------------------------------------------

#[test]
fn image_inside_footnote_goes_to_footnote_blocks() {
    let xml = r#"<root><hp:fn id="1"><hp:img src="fig.png" alt="fn-img"/></hp:fn></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::Footnote { content, .. } => {
            assert!(
                content
                    .iter()
                    .any(|b| matches!(b, ir::Block::Image { src, .. } if src == "fig.png")),
                "footnote must contain the image block"
            );
        }
        other => panic!("expected Footnote, got {other:?}"),
    }
}

#[test]
fn image_inside_list_item_goes_to_list_item_blocks() {
    let xml = r#"<root><ul><li><hp:img src="pic.png" alt="li-img"/></li></ul></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::List { items, .. } => {
            assert_eq!(items.len(), 1);
            assert!(
                items[0]
                    .blocks
                    .iter()
                    .any(|b| matches!(b, ir::Block::Image { src, .. } if src == "pic.png")),
                "list item must contain the image block"
            );
        }
        other => panic!("expected List, got {other:?}"),
    }
}

#[test]
fn linebreak_inside_list_item_appends_newline() {
    let xml = r#"<root><ul><li><hp:p><hp:run><hp:t>before</hp:t><hp:lineBreak/></hp:run></hp:p></li></ul></root>"#;
    let s = section(xml);
    match &s.blocks[0] {
        ir::Block::List { items, .. } => {
            let text: String = items[0]
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
                text.contains('\n'),
                "lineBreak in list item must produce newline; got: {text:?}"
            );
        }
        other => panic!("expected List, got {other:?}"),
    }
}

#[test]
fn resolve_bin_refs_inside_footnote() {
    let bin_map: HashMap<String, String> =
        [("BIN0002".to_string(), "BinData/BIN0002.jpg".to_string())]
            .into_iter()
            .collect();
    let mut section = ir::Section {
        blocks: vec![ir::Block::Footnote {
            id: "1".to_string(),
            content: vec![ir::Block::Image {
                src: "BIN0002".to_string(),
                alt: String::new(),
            }],
        }],

        page_layout: None,
    };
    resolve_bin_refs(&mut section, &bin_map);
    match &section.blocks[0] {
        ir::Block::Footnote { content, .. } => match &content[0] {
            ir::Block::Image { src, .. } => {
                assert_eq!(src, "BinData/BIN0002.jpg");
            }
            other => panic!("expected Image, got {other:?}"),
        },
        other => panic!("expected Footnote, got {other:?}"),
    }
}

#[test]
fn resolve_bin_refs_inside_list() {
    let bin_map: HashMap<String, String> =
        [("BIN0003".to_string(), "BinData/BIN0003.png".to_string())]
            .into_iter()
            .collect();
    let mut section = ir::Section {
        blocks: vec![ir::Block::List {
            ordered: false,
            start: 1,
            items: vec![ir::ListItem {
                blocks: vec![ir::Block::Image {
                    src: "BIN0003".to_string(),
                    alt: String::new(),
                }],
                children: Vec::new(),
            }],
        }],

        page_layout: None,
    };
    resolve_bin_refs(&mut section, &bin_map);
    match &section.blocks[0] {
        ir::Block::List { items, .. } => match &items[0].blocks[0] {
            ir::Block::Image { src, .. } => {
                assert_eq!(src, "BinData/BIN0003.png");
            }
            other => panic!("expected Image, got {other:?}"),
        },
        other => panic!("expected List, got {other:?}"),
    }
}
