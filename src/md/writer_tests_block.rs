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
        ..Default::default()
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
    let doc = make_doc_with_blocks(vec![ir::Block::Table { rows, col_count: 2, inner_margin: None }]);
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
    let doc = make_doc_with_blocks(vec![ir::Block::Table { rows, col_count: 2, inner_margin: None }]);
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
                checked: None,
            },
            ir::ListItem {
                blocks: vec![ir::Block::Paragraph {
                    inlines: vec![plain("beta")],
                }],
                children: Vec::new(),
                checked: None,
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
                checked: None,
            },
            ir::ListItem {
                blocks: vec![ir::Block::Paragraph {
                    inlines: vec![plain("second")],
                }],
                children: Vec::new(),
                checked: None,
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
            checked: None,
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
fn write_markdown_page_break_emits_html_comment_marker() {
    let doc = make_doc_with_blocks(vec![ir::Block::PageBreak]);
    let md = write_markdown(&doc, false);
    assert!(
        md.contains("<!-- pagebreak -->"),
        "PageBreak must render as an HTML-comment marker: {md}"
    );
    // The marker must NOT be confused with a thematic break.
    assert!(
        !md.lines().any(|l| l.trim() == "---"),
        "PageBreak must not emit a thematic break: {md}"
    );
}

#[test]
fn write_markdown_page_break_between_paragraphs_preserves_order() {
    let doc = make_doc_with_blocks(vec![
        ir::Block::Paragraph {
            inlines: vec![ir::Inline::plain("before")],
        },
        ir::Block::PageBreak,
        ir::Block::Paragraph {
            inlines: vec![ir::Inline::plain("after")],
        },
    ]);
    let md = write_markdown(&doc, false);
    let before_pos = md.find("before").expect("before missing");
    let marker_pos = md.find("<!-- pagebreak -->").expect("marker missing");
    let after_pos = md.find("after").expect("after missing");
    assert!(
        before_pos < marker_pos && marker_pos < after_pos,
        "block order lost: {md}"
    );
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

// -----------------------------------------------------------------------
// write_markdown — task list (GitHub-style checkboxes)
// -----------------------------------------------------------------------

fn task_item(text: &str, checked: Option<bool>) -> ir::ListItem {
    ir::ListItem {
        blocks: vec![ir::Block::Paragraph {
            inlines: vec![plain(text)],
        }],
        children: vec![],
        checked,
    }
}

#[test]
fn write_markdown_task_list_unchecked_emits_bracket_space() {
    let doc = make_doc_with_blocks(vec![ir::Block::List {
        ordered: false,
        start: 1,
        items: vec![task_item("buy milk", Some(false))],
    }]);
    let md = write_markdown(&doc, false);
    assert!(
        md.contains("- [ ] buy milk"),
        "unchecked task item must emit '- [ ] '; got: {md:?}"
    );
}

#[test]
fn write_markdown_task_list_checked_emits_bracket_x() {
    let doc = make_doc_with_blocks(vec![ir::Block::List {
        ordered: false,
        start: 1,
        items: vec![task_item("done", Some(true))],
    }]);
    let md = write_markdown(&doc, false);
    assert!(
        md.contains("- [x] done"),
        "checked task item must emit '- [x] '; got: {md:?}"
    );
}

#[test]
fn write_markdown_task_list_normal_item_emits_plain_bullet() {
    let doc = make_doc_with_blocks(vec![ir::Block::List {
        ordered: false,
        start: 1,
        items: vec![task_item("normal", None)],
    }]);
    let md = write_markdown(&doc, false);
    assert!(
        md.contains("- normal"),
        "normal list item must emit '- '; got: {md:?}"
    );
    assert!(
        !md.contains("- [ ]") && !md.contains("- [x]"),
        "normal list item must not emit checkbox; got: {md:?}"
    );
}

#[test]
fn write_markdown_task_list_mixed_items() {
    let doc = make_doc_with_blocks(vec![ir::Block::List {
        ordered: false,
        start: 1,
        items: vec![
            task_item("done", Some(true)),
            task_item("todo", Some(false)),
            task_item("normal", None),
        ],
    }]);
    let md = write_markdown(&doc, false);
    assert!(md.contains("- [x] done"), "checked item; got: {md:?}");
    assert!(md.contains("- [ ] todo"), "unchecked item; got: {md:?}");
    assert!(md.contains("- normal"), "normal item; got: {md:?}");
}

#[test]
fn write_markdown_task_list_roundtrip() {
    use crate::md::parser::parse_markdown;
    let input = "- [x] alpha\n- [ ] beta\n- plain\n";
    let doc = parse_markdown(input);
    let output = write_markdown(&doc, false);
    assert!(
        output.contains("- [x] alpha"),
        "roundtrip checked; got: {output:?}"
    );
    assert!(
        output.contains("- [ ] beta"),
        "roundtrip unchecked; got: {output:?}"
    );
    assert!(
        output.contains("- plain"),
        "roundtrip normal; got: {output:?}"
    );
}

#[test]
fn ordered_task_list_items() {
    let doc = make_doc_with_blocks(vec![ir::Block::List {
        ordered: true,
        start: 1,
        items: vec![
            task_item("완료된 작업", Some(true)),
            task_item("미완료 작업", Some(false)),
            task_item("일반 항목", None),
        ],
    }]);
    let md = write_markdown(&doc, false);
    assert!(
        md.contains("1. [x] 완료된 작업"),
        "ordered checked item must emit '1. [x] '; got: {md:?}"
    );
    assert!(
        md.contains("2. [ ] 미완료 작업"),
        "ordered unchecked item must emit '2. [ ] '; got: {md:?}"
    );
    assert!(
        md.contains("3. 일반 항목"),
        "ordered normal item must emit '3. '; got: {md:?}"
    );
    assert!(
        !md.contains("3. [x]") && !md.contains("3. [ ]"),
        "ordered normal item must not emit checkbox; got: {md:?}"
    );
}

// -----------------------------------------------------------------------
// B-4: header/footer comment markers in Markdown output
// -----------------------------------------------------------------------

#[test]
fn header_footer_in_markdown() {
    let mut doc = ir::Document::new();
    doc.sections.push(ir::Section {
        blocks: vec![ir::Block::Paragraph {
            inlines: vec![plain("body paragraph")],
        }],
        page_layout: None,
        header: Some(vec![ir::Block::Paragraph {
            inlines: vec![plain("Page header")],
        }]),
        footer: Some(vec![ir::Block::Paragraph {
            inlines: vec![plain("Page footer")],
        }]),
        header_footer_type: None,
    });

    let md = write_markdown(&doc, false);

    // HTML comment markers must wrap the header content.
    assert!(
        md.contains("<!-- header -->"),
        "header open marker missing: {md:?}"
    );
    assert!(
        md.contains("<!-- /header -->"),
        "header close marker missing: {md:?}"
    );
    assert!(md.contains("Page header"), "header text missing: {md:?}");

    // HTML comment markers must wrap the footer content.
    assert!(
        md.contains("<!-- footer -->"),
        "footer open marker missing: {md:?}"
    );
    assert!(
        md.contains("<!-- /footer -->"),
        "footer close marker missing: {md:?}"
    );
    assert!(md.contains("Page footer"), "footer text missing: {md:?}");

    // Body text must also appear.
    assert!(md.contains("body paragraph"), "body text missing: {md:?}");
}

#[test]
fn header_footer_markers_before_body() {
    // Header and footer markers must appear before the body content.
    let mut doc = ir::Document::new();
    doc.sections.push(ir::Section {
        blocks: vec![ir::Block::Paragraph {
            inlines: vec![plain("body")],
        }],
        page_layout: None,
        header: Some(vec![ir::Block::Paragraph {
            inlines: vec![plain("hdr")],
        }]),
        footer: None,
        header_footer_type: None,
    });

    let md = write_markdown(&doc, false);

    let header_pos = md.find("<!-- header -->").expect("header marker");
    let body_pos = md.find("body").expect("body text");
    assert!(
        header_pos < body_pos,
        "header markers must precede body content: {md:?}"
    );
}

#[test]
fn no_header_footer_markers_when_none() {
    // When header/footer are None, no HTML comment markers must appear.
    let doc = make_doc_with_blocks(vec![ir::Block::Paragraph {
        inlines: vec![plain("plain body")],
    }]);

    let md = write_markdown(&doc, false);

    assert!(
        !md.contains("<!-- header -->"),
        "no header marker expected: {md:?}"
    );
    assert!(
        !md.contains("<!-- footer -->"),
        "no footer marker expected: {md:?}"
    );
}
