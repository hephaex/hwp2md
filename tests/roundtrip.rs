/// Integration tests: IR → Markdown → IR and Markdown → IR → Markdown roundtrips.
use hwp2md::ir;
use hwp2md::md::{parse_markdown, write_markdown};

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

fn plain(t: &str) -> ir::Inline {
    ir::Inline::plain(t)
}

fn make_doc(blocks: Vec<ir::Block>) -> ir::Document {
    let mut doc = ir::Document::new();
    doc.sections.push(ir::Section { blocks });
    doc
}

fn first_blocks(doc: &ir::Document) -> &[ir::Block] {
    doc.sections.first().map(|s| s.blocks.as_slice()).unwrap_or(&[])
}

// -----------------------------------------------------------------------
// IR → Markdown → IR: key structure preserved
// -----------------------------------------------------------------------

#[test]
fn roundtrip_ir_to_md_to_ir_heading() {
    let original = make_doc(vec![ir::Block::Heading {
        level: 2,
        inlines: vec![plain("Round Trip")],
    }]);

    let md = write_markdown(&original, false);
    let parsed = parse_markdown(&md);

    let blocks = first_blocks(&parsed);
    let found = blocks.iter().any(|b| matches!(b, ir::Block::Heading { level: 2, .. }));
    assert!(found, "heading level 2 not found after roundtrip; md: {md:?}");
}

#[test]
fn roundtrip_ir_to_md_to_ir_paragraph() {
    let original = make_doc(vec![ir::Block::Paragraph {
        inlines: vec![plain("Some paragraph text.")],
    }]);

    let md = write_markdown(&original, false);
    let parsed = parse_markdown(&md);

    let text: String = first_blocks(&parsed)
        .iter()
        .filter_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
            } else {
                None
            }
        })
        .collect();

    assert!(
        text.contains("Some paragraph text."),
        "paragraph text lost; parsed text: {text:?}"
    );
}

#[test]
fn roundtrip_ir_to_md_to_ir_code_block() {
    let original = make_doc(vec![ir::Block::CodeBlock {
        language: Some("python".into()),
        code: "print('hello')".into(),
    }]);

    let md = write_markdown(&original, false);
    let parsed = parse_markdown(&md);

    let found = first_blocks(&parsed).iter().any(|b| match b {
        ir::Block::CodeBlock { language, code } => {
            language.as_deref() == Some("python") && code.contains("print")
        }
        _ => false,
    });
    assert!(found, "code block not preserved; md: {md:?}");
}

#[test]
fn roundtrip_ir_to_md_to_ir_horizontal_rule() {
    let original = make_doc(vec![
        ir::Block::Paragraph {
            inlines: vec![plain("before")],
        },
        ir::Block::HorizontalRule,
        ir::Block::Paragraph {
            inlines: vec![plain("after")],
        },
    ]);

    let md = write_markdown(&original, false);
    let parsed = parse_markdown(&md);

    let has_hr = first_blocks(&parsed)
        .iter()
        .any(|b| matches!(b, ir::Block::HorizontalRule));
    assert!(has_hr, "horizontal rule not found; md: {md:?}");
}

#[test]
fn roundtrip_ir_to_md_to_ir_unordered_list() {
    let original = make_doc(vec![ir::Block::List {
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

    let md = write_markdown(&original, false);
    let parsed = parse_markdown(&md);

    let found = first_blocks(&parsed).iter().any(|b| match b {
        ir::Block::List { ordered: false, items, .. } => items.len() == 2,
        _ => false,
    });
    assert!(found, "unordered list not preserved; md: {md:?}");
}

#[test]
fn roundtrip_ir_to_md_to_ir_ordered_list() {
    let original = make_doc(vec![ir::Block::List {
        ordered: true,
        start: 1,
        items: vec![
            ir::ListItem {
                blocks: vec![ir::Block::Paragraph {
                    inlines: vec![plain("one")],
                }],
                children: Vec::new(),
            },
            ir::ListItem {
                blocks: vec![ir::Block::Paragraph {
                    inlines: vec![plain("two")],
                }],
                children: Vec::new(),
            },
        ],
    }]);

    let md = write_markdown(&original, false);
    let parsed = parse_markdown(&md);

    let found = first_blocks(&parsed).iter().any(|b| match b {
        ir::Block::List { ordered: true, items, .. } => items.len() == 2,
        _ => false,
    });
    assert!(found, "ordered list not preserved; md: {md:?}");
}

#[test]
fn roundtrip_ir_to_md_to_ir_gfm_table() {
    let rows = vec![
        ir::TableRow {
            cells: vec![
                ir::TableCell {
                    blocks: vec![ir::Block::Paragraph {
                        inlines: vec![plain("Col1")],
                    }],
                    ..Default::default()
                },
                ir::TableCell {
                    blocks: vec![ir::Block::Paragraph {
                        inlines: vec![plain("Col2")],
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
                        inlines: vec![plain("val1")],
                    }],
                    ..Default::default()
                },
                ir::TableCell {
                    blocks: vec![ir::Block::Paragraph {
                        inlines: vec![plain("val2")],
                    }],
                    ..Default::default()
                },
            ],
            is_header: false,
        },
    ];
    let original = make_doc(vec![ir::Block::Table {
        rows,
        col_count: 2,
    }]);

    let md = write_markdown(&original, false);
    let parsed = parse_markdown(&md);

    let found = first_blocks(&parsed).iter().any(|b| match b {
        ir::Block::Table { rows, col_count } => *col_count >= 2 && rows.len() >= 2,
        _ => false,
    });
    assert!(found, "GFM table not preserved; md: {md:?}");
}

#[test]
fn roundtrip_ir_to_md_to_ir_block_quote() {
    let original = make_doc(vec![ir::Block::BlockQuote {
        blocks: vec![ir::Block::Paragraph {
            inlines: vec![plain("quoted")],
        }],
    }]);

    let md = write_markdown(&original, false);
    let parsed = parse_markdown(&md);

    let has_bq = first_blocks(&parsed)
        .iter()
        .any(|b| matches!(b, ir::Block::BlockQuote { .. }));
    assert!(has_bq, "blockquote not preserved; md: {md:?}");
}

// -----------------------------------------------------------------------
// MD → IR → MD: output is stable on a second pass
// -----------------------------------------------------------------------

#[test]
fn roundtrip_md_to_ir_to_md_stable_heading() {
    let source = "## Stable Heading\n\nSome content.\n";
    let doc1 = parse_markdown(source);
    let md1 = write_markdown(&doc1, false);
    let doc2 = parse_markdown(&md1);
    let md2 = write_markdown(&doc2, false);

    // The heading marker must survive both passes.
    assert!(md1.contains("## Stable Heading"), "pass 1 md: {md1:?}");
    assert!(md2.contains("## Stable Heading"), "pass 2 md: {md2:?}");
}

#[test]
fn roundtrip_md_to_ir_to_md_stable_bold() {
    let source = "**bold text** here\n";
    let doc1 = parse_markdown(source);
    let md1 = write_markdown(&doc1, false);
    let doc2 = parse_markdown(&md1);
    let md2 = write_markdown(&doc2, false);

    assert!(md1.contains("**bold text**"), "pass 1: {md1:?}");
    assert!(md2.contains("**bold text**"), "pass 2: {md2:?}");
}

#[test]
fn roundtrip_md_to_ir_to_md_stable_code_block() {
    let source = "```bash\necho hello\n```\n";
    let doc = parse_markdown(source);
    let md = write_markdown(&doc, false);
    assert!(md.contains("```bash"), "got: {md:?}");
    assert!(md.contains("echo hello"), "got: {md:?}");
}

#[test]
fn roundtrip_mixed_content() {
    // A document with heading + paragraph + code block + list.
    let original = make_doc(vec![
        ir::Block::Heading {
            level: 1,
            inlines: vec![plain("Document")],
        },
        ir::Block::Paragraph {
            inlines: vec![plain("Intro paragraph.")],
        },
        ir::Block::CodeBlock {
            language: Some("rust".into()),
            code: "let x = 1;".into(),
        },
        ir::Block::List {
            ordered: false,
            start: 1,
            items: vec![ir::ListItem {
                blocks: vec![ir::Block::Paragraph {
                    inlines: vec![plain("list item")],
                }],
                children: Vec::new(),
            }],
        },
    ]);

    let md = write_markdown(&original, false);
    let parsed = parse_markdown(&md);
    let blocks = first_blocks(&parsed);

    let has_h1 = blocks
        .iter()
        .any(|b| matches!(b, ir::Block::Heading { level: 1, .. }));
    let has_code = blocks
        .iter()
        .any(|b| matches!(b, ir::Block::CodeBlock { .. }));
    let has_list = blocks
        .iter()
        .any(|b| matches!(b, ir::Block::List { .. }));

    assert!(has_h1, "H1 missing; md: {md:?}");
    assert!(has_code, "CodeBlock missing; md: {md:?}");
    assert!(has_list, "List missing; md: {md:?}");
}

#[test]
fn roundtrip_inline_formatting_bold_italic_strikethrough() {
    let source = "This has **bold**, *italic*, and ~~strikethrough~~ text.\n";
    let doc = parse_markdown(source);
    let md = write_markdown(&doc, false);

    assert!(md.contains("**bold**"), "bold missing; md: {md:?}");
    assert!(md.contains("*italic*"), "italic missing; md: {md:?}");
    assert!(
        md.contains("~~strikethrough~~"),
        "strikethrough missing; md: {md:?}"
    );
}

#[test]
fn roundtrip_ir_inline_flags_survive() {
    let original = make_doc(vec![ir::Block::Paragraph {
        inlines: vec![
            plain("normal "),
            ir::Inline {
                text: "bold".into(),
                bold: true,
                ..Default::default()
            },
            plain(" "),
            ir::Inline {
                text: "italic".into(),
                italic: true,
                ..Default::default()
            },
        ],
    }]);

    let md = write_markdown(&original, false);
    let parsed = parse_markdown(&md);

    if let Some(ir::Block::Paragraph { inlines }) = first_blocks(&parsed).first() {
        let has_bold = inlines.iter().any(|i| i.bold && i.text.contains("bold"));
        let has_italic = inlines.iter().any(|i| i.italic && i.text.contains("italic"));
        assert!(has_bold, "bold flag not preserved; inlines: {inlines:?}");
        assert!(has_italic, "italic flag not preserved; inlines: {inlines:?}");
    } else {
        panic!("Expected Paragraph block");
    }
}
