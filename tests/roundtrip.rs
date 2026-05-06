/// Integration tests: IR → Markdown → IR and Markdown → IR → Markdown roundtrips.
use hwp2md::ir;
use hwp2md::md::{parse_markdown, write_markdown};

#[path = "common/mod.rs"]
mod common;

use common::{first_blocks, make_doc, plain};

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
    let found = blocks
        .iter()
        .any(|b| matches!(b, ir::Block::Heading { level: 2, .. }));
    assert!(
        found,
        "heading level 2 not found after roundtrip; md: {md:?}"
    );
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
fn roundtrip_ir_to_md_to_ir_page_break_preserves_block_and_position() {
    let original = make_doc(vec![
        ir::Block::Paragraph {
            inlines: vec![plain("page one")],
        },
        ir::Block::PageBreak,
        ir::Block::Paragraph {
            inlines: vec![plain("page two")],
        },
    ]);

    let md = write_markdown(&original, false);
    assert!(
        md.contains("<!-- pagebreak -->"),
        "page break marker missing in markdown output: {md:?}"
    );

    let parsed = parse_markdown(&md);
    let kinds: Vec<&'static str> = first_blocks(&parsed)
        .iter()
        .map(|b| match b {
            ir::Block::Paragraph { .. } => "para",
            ir::Block::PageBreak => "pb",
            _ => "other",
        })
        .collect();
    assert_eq!(
        kinds,
        vec!["para", "pb", "para"],
        "block sequence lost across roundtrip; md: {md:?}; kinds: {kinds:?}"
    );
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

    let md = write_markdown(&original, false);
    let parsed = parse_markdown(&md);

    let found = first_blocks(&parsed).iter().any(|b| match b {
        ir::Block::List {
            ordered: false,
            items,
            ..
        } => items.len() == 2,
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
                checked: None,
            },
            ir::ListItem {
                blocks: vec![ir::Block::Paragraph {
                    inlines: vec![plain("two")],
                }],
                children: Vec::new(),
                checked: None,
            },
        ],
    }]);

    let md = write_markdown(&original, false);
    let parsed = parse_markdown(&md);

    let found = first_blocks(&parsed).iter().any(|b| match b {
        ir::Block::List {
            ordered: true,
            items,
            ..
        } => items.len() == 2,
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
    let original = make_doc(vec![ir::Block::Table { rows, col_count: 2 }]);

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
                checked: None,
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
    let has_list = blocks.iter().any(|b| matches!(b, ir::Block::List { .. }));

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
        let has_italic = inlines
            .iter()
            .any(|i| i.italic && i.text.contains("italic"));
        assert!(has_bold, "bold flag not preserved; inlines: {inlines:?}");
        assert!(
            has_italic,
            "italic flag not preserved; inlines: {inlines:?}"
        );
    } else {
        panic!("Expected Paragraph block");
    }
}

// -----------------------------------------------------------------------
// IR → Markdown → IR: additional block types
// -----------------------------------------------------------------------

#[test]
fn roundtrip_ir_to_md_to_ir_display_math() {
    let original = make_doc(vec![ir::Block::Math {
        display: true,
        tex: "E=mc^2".into(),
    }]);

    let md = write_markdown(&original, false);
    let parsed = parse_markdown(&md);

    // comrak may parse the multi-line $$…$$ block as a block-level Math node
    // or, depending on version/context, as a Paragraph whose inline text
    // contains the delimiters.  Both representations carry the formula.
    let contains_formula = first_blocks(&parsed).iter().any(|b| match b {
        ir::Block::Math { tex, .. } => tex.contains("E=mc^2"),
        ir::Block::Paragraph { inlines } => inlines.iter().any(|i| i.text.contains("E=mc^2")),
        _ => false,
    });
    assert!(
        contains_formula,
        "display math formula lost after roundtrip; md: {md:?}"
    );
}

#[test]
fn roundtrip_ir_to_md_to_ir_footnote() {
    // A paragraph that references footnote "fn1" + the Footnote definition block.
    let original = make_doc(vec![
        ir::Block::Paragraph {
            inlines: vec![
                plain("See"),
                ir::Inline {
                    text: String::new(),
                    footnote_ref: Some("fn1".into()),
                    ..Default::default()
                },
            ],
        },
        ir::Block::Footnote {
            id: "fn1".into(),
            content: vec![ir::Block::Paragraph {
                inlines: vec![plain("footnote body")],
            }],
        },
    ]);

    let md = write_markdown(&original, false);
    let parsed = parse_markdown(&md);

    let has_fn = first_blocks(&parsed)
        .iter()
        .any(|b| matches!(b, ir::Block::Footnote { id, .. } if id == "fn1"));
    assert!(
        has_fn,
        "footnote block not found after roundtrip; md: {md:?}"
    );
}

#[test]
fn roundtrip_ir_to_md_to_ir_image_block() {
    let original = make_doc(vec![ir::Block::Image {
        src: "photo.png".into(),
        alt: "A photo".into(),
    }]);

    let md = write_markdown(&original, false);
    // The writer emits ![A photo](photo.png).  The parser may return an
    // ir::Block::Image or inline text — either way the src and alt must survive.
    assert!(
        md.contains("photo.png") && md.contains("A photo"),
        "image attributes missing from md: {md:?}"
    );

    let parsed = parse_markdown(&md);
    let found = first_blocks(&parsed).iter().any(|b| match b {
        ir::Block::Image { src, alt } => src.contains("photo.png") && alt.contains("A photo"),
        ir::Block::Paragraph { inlines } => inlines
            .iter()
            .any(|i| i.text.contains("photo.png") && i.text.contains("A photo")),
        _ => false,
    });
    assert!(found, "image not recovered after roundtrip; md: {md:?}");
}

#[test]
fn roundtrip_ir_to_md_to_ir_nested_list() {
    // A list with one top-level item that has a sub-list child.
    let original = make_doc(vec![ir::Block::List {
        ordered: false,
        start: 1,
        items: vec![ir::ListItem {
            blocks: vec![ir::Block::Paragraph {
                inlines: vec![plain("parent")],
            }],
            children: vec![ir::ListItem {
                blocks: vec![ir::Block::Paragraph {
                    inlines: vec![plain("child")],
                }],
                children: Vec::new(),
                checked: None,
            }],
            checked: None,
        }],
    }]);

    let md = write_markdown(&original, false);

    // Both "parent" and "child" text must appear in the markdown output.
    assert!(md.contains("parent"), "parent text missing; md: {md:?}");
    assert!(md.contains("child"), "child text missing; md: {md:?}");

    let parsed = parse_markdown(&md);

    // The parser nests the child list inside the parent item's blocks.
    // Accept any structure that carries both text values.
    fn collect_text(blocks: &[ir::Block]) -> String {
        let mut out = String::new();
        for b in blocks {
            match b {
                ir::Block::Paragraph { inlines } => {
                    for i in inlines {
                        out.push_str(&i.text);
                    }
                }
                ir::Block::List { items, .. } => {
                    for item in items {
                        out.push_str(&collect_text(&item.blocks));
                        out.push_str(&collect_text_items(&item.children));
                    }
                }
                _ => {}
            }
        }
        out
    }
    fn collect_text_items(items: &[ir::ListItem]) -> String {
        let mut out = String::new();
        for item in items {
            out.push_str(&collect_text(&item.blocks));
            out.push_str(&collect_text_items(&item.children));
        }
        out
    }

    let all_text = collect_text(first_blocks(&parsed));
    assert!(
        all_text.contains("parent"),
        "parent text lost after roundtrip; blocks: {:?}",
        first_blocks(&parsed)
    );
    assert!(
        all_text.contains("child"),
        "child text lost after roundtrip; blocks: {:?}",
        first_blocks(&parsed)
    );
}

#[test]
fn roundtrip_ir_to_md_to_ir_frontmatter() {
    let mut doc = ir::Document::new();
    doc.metadata.title = Some("My Title".into());
    doc.metadata.author = Some("Test Author".into());
    doc.sections.push(ir::Section {
        blocks: vec![ir::Block::Paragraph {
            inlines: vec![plain("body")],
        }],

        page_layout: None,
        ..Default::default()
    });

    let md = write_markdown(&doc, true);
    assert!(
        md.contains("title:"),
        "frontmatter title key missing; md: {md:?}"
    );

    let parsed = parse_markdown(&md);
    assert_eq!(
        parsed.metadata.title.as_deref(),
        Some("My Title"),
        "title not preserved; md: {md:?}"
    );
    assert_eq!(
        parsed.metadata.author.as_deref(),
        Some("Test Author"),
        "author not preserved; md: {md:?}"
    );
}

