use super::*;
use crate::ir;

// -----------------------------------------------------------------------
// Shared helpers (re-declared here for standalone module use)
// -----------------------------------------------------------------------

fn plain(t: &str) -> ir::Inline {
    ir::Inline::plain(t)
}

fn make_doc_with_blocks(blocks: Vec<ir::Block>) -> ir::Document {
    let mut doc = ir::Document::new();
    doc.sections.push(ir::Section {
        blocks,
        page_layout: None,
    });
    doc
}

// -----------------------------------------------------------------------
// write_markdown — headings and paragraphs
// -----------------------------------------------------------------------

#[test]
fn write_markdown_heading_levels() {
    for level in 1u8..=6 {
        let doc = make_doc_with_blocks(vec![ir::Block::Heading {
            level,
            inlines: vec![plain("Title")],
        }]);
        let md = write_markdown(&doc, false);
        let hashes = "#".repeat(level as usize);
        assert!(
            md.starts_with(&format!("{hashes} Title")),
            "level {level}: got {md:?}"
        );
    }
}

#[test]
fn write_markdown_paragraph() {
    let doc = make_doc_with_blocks(vec![ir::Block::Paragraph {
        inlines: vec![plain("Hello, world.")],
    }]);
    let md = write_markdown(&doc, false);
    assert!(md.contains("Hello, world."));
}

// -----------------------------------------------------------------------
// write_markdown — code blocks
// -----------------------------------------------------------------------

#[test]
fn write_markdown_code_block() {
    let doc = make_doc_with_blocks(vec![ir::Block::CodeBlock {
        language: Some("rust".into()),
        code: "fn main() {}".into(),
    }]);
    let md = write_markdown(&doc, false);
    assert!(md.contains("```rust\n"), "got: {md}");
    assert!(md.contains("fn main() {}"), "got: {md}");
    assert!(md.contains("\n```"), "got: {md}");
}

#[test]
fn write_markdown_code_block_no_language() {
    let doc = make_doc_with_blocks(vec![ir::Block::CodeBlock {
        language: None,
        code: "raw code".into(),
    }]);
    let md = write_markdown(&doc, false);
    assert!(md.contains("```\n"), "got: {md}");
    assert!(md.contains("raw code"), "got: {md}");
}

// -----------------------------------------------------------------------
// write_markdown — tables (GFM)
// -----------------------------------------------------------------------

#[test]
fn write_markdown_simple_gfm_table() {
    let rows = vec![
        ir::TableRow {
            cells: vec![
                ir::TableCell {
                    blocks: vec![ir::Block::Paragraph {
                        inlines: vec![plain("Name")],
                    }],
                    ..Default::default()
                },
                ir::TableCell {
                    blocks: vec![ir::Block::Paragraph {
                        inlines: vec![plain("Age")],
                    }],
                    ..Default::default()
                },
            ],
            is_header: true,
        },
        ir::TableRow {
            cells: vec![
                ir::TableCell {
                    blocks: vec![ir::Block::Paragraph {
                        inlines: vec![plain("Alice")],
                    }],
                    ..Default::default()
                },
                ir::TableCell {
                    blocks: vec![ir::Block::Paragraph {
                        inlines: vec![plain("30")],
                    }],
                    ..Default::default()
                },
            ],
            is_header: false,
        },
    ];
    let doc = make_doc_with_blocks(vec![ir::Block::Table { rows, col_count: 2 }]);
    let md = write_markdown(&doc, false);
    assert!(md.contains("| Name | Age |"), "got: {md}");
    assert!(md.contains("| --- |"), "got: {md}");
    assert!(md.contains("| Alice | 30 |"), "got: {md}");
}

#[test]
fn write_markdown_complex_table_html_fallback() {
    // A cell with colspan > 1 must trigger the HTML table fallback.
    let rows = vec![ir::TableRow {
        cells: vec![ir::TableCell {
            blocks: vec![ir::Block::Paragraph {
                inlines: vec![plain("wide")],
            }],
            colspan: 2,
            rowspan: 1,
        }],
        is_header: true,
    }];
    let doc = make_doc_with_blocks(vec![ir::Block::Table { rows, col_count: 2 }]);
    let md = write_markdown(&doc, false);
    assert!(md.contains("<table>"), "got: {md}");
    assert!(md.contains("colspan=\"2\""), "got: {md}");
}

// -----------------------------------------------------------------------
// write_markdown — lists
// -----------------------------------------------------------------------

#[test]
fn write_markdown_unordered_list() {
    let doc = make_doc_with_blocks(vec![ir::Block::List {
        ordered: false,
        start: 1,
        items: vec![
            ir::ListItem {
                blocks: vec![ir::Block::Paragraph {
                    inlines: vec![plain("alpha")],
                }],
                children: Vec::new(),
            },
            ir::ListItem {
                blocks: vec![ir::Block::Paragraph {
                    inlines: vec![plain("beta")],
                }],
                children: Vec::new(),
            },
        ],
    }]);
    let md = write_markdown(&doc, false);
    assert!(md.contains("- alpha"), "got: {md}");
    assert!(md.contains("- beta"), "got: {md}");
}

#[test]
fn write_markdown_ordered_list() {
    let doc = make_doc_with_blocks(vec![ir::Block::List {
        ordered: true,
        start: 1,
        items: vec![
            ir::ListItem {
                blocks: vec![ir::Block::Paragraph {
                    inlines: vec![plain("first")],
                }],
                children: Vec::new(),
            },
            ir::ListItem {
                blocks: vec![ir::Block::Paragraph {
                    inlines: vec![plain("second")],
                }],
                children: Vec::new(),
            },
        ],
    }]);
    let md = write_markdown(&doc, false);
    assert!(md.contains("1. first"), "got: {md}");
    assert!(md.contains("2. second"), "got: {md}");
}

#[test]
fn write_markdown_list_item_with_multiple_blocks() {
    let doc = make_doc_with_blocks(vec![ir::Block::List {
        ordered: false,
        start: 1,
        items: vec![ir::ListItem {
            blocks: vec![
                ir::Block::Paragraph {
                    inlines: vec![plain("first block")],
                },
                ir::Block::Paragraph {
                    inlines: vec![plain("continuation block")],
                },
            ],
            children: Vec::new(),
        }],
    }]);
    let md = write_markdown(&doc, false);
    assert!(md.contains("first block"), "got: {md}");
    assert!(md.contains("continuation block"), "got: {md}");
}

// -----------------------------------------------------------------------
// write_markdown — image, HR, math, footnote
// -----------------------------------------------------------------------

#[test]
fn write_markdown_image() {
    let doc = make_doc_with_blocks(vec![ir::Block::Image {
        src: "img.png".into(),
        alt: "a picture".into(),
    }]);
    let md = write_markdown(&doc, false);
    assert!(md.contains("![a picture](img.png)"), "got: {md}");
}

#[test]
fn write_markdown_horizontal_rule() {
    let doc = make_doc_with_blocks(vec![ir::Block::HorizontalRule]);
    let md = write_markdown(&doc, false);
    assert!(md.contains("---"), "got: {md}");
}

#[test]
fn write_markdown_math_display() {
    let doc = make_doc_with_blocks(vec![ir::Block::Math {
        display: true,
        tex: "E=mc^2".into(),
    }]);
    let md = write_markdown(&doc, false);
    assert!(md.contains("$$\n"), "got: {md}");
    assert!(md.contains("E=mc^2"), "got: {md}");
}

#[test]
fn write_markdown_math_inline() {
    let doc = make_doc_with_blocks(vec![ir::Block::Math {
        display: false,
        tex: "x+y".into(),
    }]);
    let md = write_markdown(&doc, false);
    assert!(md.contains("$x+y$"), "got: {md}");
}

#[test]
fn write_markdown_footnote() {
    let doc = make_doc_with_blocks(vec![ir::Block::Footnote {
        id: "fn1".into(),
        content: vec![ir::Block::Paragraph {
            inlines: vec![plain("footnote text")],
        }],
    }]);
    let md = write_markdown(&doc, false);
    assert!(md.contains("[^fn1]:"), "got: {md}");
    assert!(md.contains("footnote text"), "got: {md}");
}

// -----------------------------------------------------------------------
// write_markdown — blockquote
// -----------------------------------------------------------------------

#[test]
fn write_markdown_blockquote_nested_paragraph() {
    let doc = make_doc_with_blocks(vec![ir::Block::BlockQuote {
        blocks: vec![ir::Block::Paragraph {
            inlines: vec![plain("quoted text")],
        }],
    }]);
    let md = write_markdown(&doc, false);
    assert!(md.contains("> quoted text"), "got: {md}");
}
