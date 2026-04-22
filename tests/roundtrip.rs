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
    doc.sections
        .first()
        .map(|s| s.blocks.as_slice())
        .unwrap_or(&[])
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
            }],
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

// -----------------------------------------------------------------------
// MD → IR → MD: stability tests
// -----------------------------------------------------------------------

#[test]
fn roundtrip_md_to_ir_to_md_stable_display_math() {
    // The writer emits $$\n…\n$$ for display math; ensure the formula survives.
    let original = make_doc(vec![ir::Block::Math {
        display: true,
        tex: "E=mc^2".into(),
    }]);
    let md1 = write_markdown(&original, false);

    let doc2 = parse_markdown(&md1);
    let md2 = write_markdown(&doc2, false);

    // Both passes must contain the formula.
    assert!(
        md1.contains("E=mc^2"),
        "formula missing from pass 1; md1: {md1:?}"
    );
    assert!(
        md2.contains("E=mc^2"),
        "formula missing from pass 2; md2: {md2:?}"
    );
}

#[test]
fn roundtrip_md_to_ir_to_md_stable_footnote() {
    let source = "Text[^1]\n\n[^1]: footnote body\n";
    let doc1 = parse_markdown(source);
    let md1 = write_markdown(&doc1, false);
    let doc2 = parse_markdown(&md1);
    let md2 = write_markdown(&doc2, false);

    assert!(
        md1.contains("[^1]"),
        "footnote ref missing from pass 1; md1: {md1:?}"
    );
    assert!(
        md2.contains("[^1]"),
        "footnote ref missing from pass 2; md2: {md2:?}"
    );
    assert!(
        md1.contains("footnote body"),
        "footnote body missing from pass 1; md1: {md1:?}"
    );
    assert!(
        md2.contains("footnote body"),
        "footnote body missing from pass 2; md2: {md2:?}"
    );
}

#[test]
fn roundtrip_md_to_ir_to_md_escaped_text_preserved() {
    // Backslash-escaped metacharacters: the parser strips the backslash and
    // produces plain text; the writer re-escapes on output.  The underlying
    // text content (without backslashes) must survive both passes.
    let source = "a\\*b\\_c\\~d\n";
    let doc1 = parse_markdown(source);
    let md1 = write_markdown(&doc1, false);
    let doc2 = parse_markdown(&md1);
    let md2 = write_markdown(&doc2, false);

    // Plain text "a*b_c~d" must be present in both IR representations.
    let text1: String = first_blocks(&doc1)
        .iter()
        .filter_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
            } else {
                None
            }
        })
        .collect();
    let text2: String = first_blocks(&doc2)
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
        text1.contains("a*b_c~d"),
        "plain text lost in pass 1; text1: {text1:?}"
    );
    assert!(
        text2.contains("a*b_c~d"),
        "plain text lost in pass 2; text2: {text2:?}"
    );
    // The writer must re-escape the metacharacters.
    assert!(md1.contains("\\*"), "asterisk not re-escaped; md1: {md1:?}");
    assert!(md2.contains("\\*"), "asterisk not re-escaped; md2: {md2:?}");
}

#[test]
fn roundtrip_md_to_ir_to_md_stable_image() {
    // comrak wraps a standalone image in a Paragraph and records it as an
    // inline.  The roundtrip should preserve the src and alt attributes.
    let source = "![alt text](image.png)\n";
    let doc1 = parse_markdown(source);
    let md1 = write_markdown(&doc1, false);
    let doc2 = parse_markdown(&md1);
    let md2 = write_markdown(&doc2, false);

    // Both passes must carry the URL and alt text.
    assert!(
        md1.contains("image.png") && md1.contains("alt text"),
        "image attrs missing from pass 1; md1: {md1:?}"
    );
    assert!(
        md2.contains("image.png") && md2.contains("alt text"),
        "image attrs missing from pass 2; md2: {md2:?}"
    );
}

#[test]
fn roundtrip_md_to_ir_to_md_html_table_colspan() {
    // A table with a colspan attribute triggers the HTML table path.
    // The original IR representation is built directly so we control the output.
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
    let original = make_doc(vec![ir::Block::Table { rows, col_count: 2 }]);

    let md = write_markdown(&original, false);
    assert!(md.contains("<table>"), "HTML table tag missing; md: {md:?}");
    assert!(
        md.contains("colspan=\"2\""),
        "colspan attr missing; md: {md:?}"
    );
    assert!(md.contains("wide"), "cell text missing; md: {md:?}");
}

// -----------------------------------------------------------------------
// Edge case roundtrips
// -----------------------------------------------------------------------

#[test]
fn roundtrip_empty_document() {
    // A document with no blocks must roundtrip without panicking and produce
    // output that parses back to a document with no meaningful blocks.
    let original = make_doc(vec![]);
    let md = write_markdown(&original, false);
    let parsed = parse_markdown(&md);

    // Either the section is empty or contains only empty paragraphs.
    let non_empty = first_blocks(&parsed).iter().any(|b| match b {
        ir::Block::Paragraph { inlines } => !inlines.is_empty(),
        ir::Block::HorizontalRule => true,
        _ => true,
    });
    // For an empty document, we expect no meaningful content.
    assert!(
        !non_empty,
        "empty document roundtrip produced unexpected blocks; md: {md:?}"
    );
}

#[test]
fn roundtrip_unicode_korean_text() {
    let korean = "안녕하세요 세계";
    let original = make_doc(vec![
        ir::Block::Heading {
            level: 1,
            inlines: vec![plain(korean)],
        },
        ir::Block::Paragraph {
            inlines: vec![plain("한국어 단락입니다.")],
        },
    ]);

    let md = write_markdown(&original, false);
    assert!(
        md.contains(korean),
        "Korean heading text missing; md: {md:?}"
    );
    assert!(
        md.contains("한국어 단락입니다."),
        "Korean paragraph text missing; md: {md:?}"
    );

    let parsed = parse_markdown(&md);

    let heading_text: String = first_blocks(&parsed)
        .iter()
        .filter_map(|b| {
            if let ir::Block::Heading { inlines, .. } = b {
                Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
            } else {
                None
            }
        })
        .collect();
    assert!(
        heading_text.contains(korean),
        "Korean heading lost after roundtrip; heading_text: {heading_text:?}"
    );

    let para_text: String = first_blocks(&parsed)
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
        para_text.contains("한국어"),
        "Korean paragraph text lost; para_text: {para_text:?}"
    );
}

#[test]
fn roundtrip_code_block_two_pass_content_identical() {
    // A multi-line code block with special characters must survive two full
    // write→parse→write cycles with identical code content both times.
    let code = "fn greet(name: &str) {\n    println!(\"Hello, {name}!\");\n    // a < b && c > d\n    let x = 1 * 2 + 3 - 4;\n}";
    let original = make_doc(vec![ir::Block::CodeBlock {
        language: Some("rust".into()),
        code: code.to_string(),
    }]);

    let md1 = write_markdown(&original, false);
    let doc2 = parse_markdown(&md1);
    let md2 = write_markdown(&doc2, false);
    let doc3 = parse_markdown(&md2);

    let extract_code = |doc: &ir::Document| -> Option<String> {
        first_blocks(doc).iter().find_map(|b| {
            if let ir::Block::CodeBlock { code, .. } = b {
                Some(code.clone())
            } else {
                None
            }
        })
    };

    let code2 = extract_code(&doc2).expect("code block missing after pass 1");
    let code3 = extract_code(&doc3).expect("code block missing after pass 2");

    assert_eq!(
        code2.trim(),
        code.trim(),
        "code content changed after pass 1\npass1 md:\n{md1}"
    );
    assert_eq!(
        code3.trim(),
        code.trim(),
        "code content changed after pass 2\npass2 md:\n{md2}"
    );
    assert_eq!(md1, md2, "markdown output not stable across two passes");
}
