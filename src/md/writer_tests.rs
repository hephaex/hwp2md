use super::*;
use crate::ir;

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

fn plain(t: &str) -> ir::Inline {
    ir::Inline::plain(t)
}

fn make_doc_with_blocks(blocks: Vec<ir::Block>) -> ir::Document {
    let mut doc = ir::Document::new();
    doc.sections.push(ir::Section { blocks });
    doc
}

// -----------------------------------------------------------------------
// escape_yaml
// -----------------------------------------------------------------------

#[test]
fn escape_yaml_backslash() {
    assert_eq!(escape_yaml("a\\b"), "a\\\\b");
}

#[test]
fn escape_yaml_double_quote() {
    assert_eq!(escape_yaml("say \"hi\""), "say \\\"hi\\\"");
}

#[test]
fn escape_yaml_no_special_chars() {
    assert_eq!(escape_yaml("hello world"), "hello world");
}

#[test]
fn escape_yaml_newline() {
    assert_eq!(escape_yaml("line1\nline2"), "line1\\nline2");
}

#[test]
fn escape_yaml_carriage_return() {
    assert_eq!(escape_yaml("a\rb"), "a\\rb");
}

#[test]
fn escape_yaml_tab() {
    assert_eq!(escape_yaml("a\tb"), "a\\tb");
}

// -----------------------------------------------------------------------
// render_inlines
// -----------------------------------------------------------------------

#[test]
fn render_inlines_plain() {
    assert_eq!(render_inlines(&[plain("hello")]), "hello");
}

#[test]
fn render_inlines_bold() {
    let inlines = vec![ir::Inline {
        text: "bold".into(),
        bold: true,
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "**bold**");
}

#[test]
fn render_inlines_italic() {
    let inlines = vec![ir::Inline {
        text: "em".into(),
        italic: true,
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "*em*");
}

#[test]
fn render_inlines_bold_italic() {
    let inlines = vec![ir::Inline {
        text: "bi".into(),
        bold: true,
        italic: true,
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "***bi***");
}

#[test]
fn render_inlines_strikethrough() {
    let inlines = vec![ir::Inline {
        text: "del".into(),
        strikethrough: true,
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "~~del~~");
}

#[test]
fn render_inlines_underline() {
    let inlines = vec![ir::Inline {
        text: "ul".into(),
        underline: true,
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "<u>ul</u>");
}

#[test]
fn render_inlines_superscript() {
    let inlines = vec![ir::Inline {
        text: "sup".into(),
        superscript: true,
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "<sup>sup</sup>");
}

#[test]
fn render_inlines_subscript() {
    let inlines = vec![ir::Inline {
        text: "sub".into(),
        subscript: true,
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "<sub>sub</sub>");
}

#[test]
fn render_inlines_code() {
    let inlines = vec![ir::Inline {
        text: "code()".into(),
        code: true,
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "`code()`");
}

#[test]
fn render_inlines_link() {
    let inlines = vec![ir::Inline {
        text: "click".into(),
        link: Some("https://example.com".into()),
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "[click](https://example.com)");
}

#[test]
fn render_inlines_footnote_ref() {
    let inlines = vec![ir::Inline {
        text: String::new(),
        footnote_ref: Some("1".into()),
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "[^1]");
}

// -----------------------------------------------------------------------
// write_markdown — block types
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

#[test]
fn write_markdown_frontmatter() {
    let mut doc = ir::Document::new();
    doc.metadata.title = Some("My Title".into());
    doc.metadata.author = Some("Author Name".into());
    doc.sections.push(ir::Section { blocks: Vec::new() });
    let md = write_markdown(&doc, true);
    assert!(md.starts_with("---\n"), "got: {md}");
    assert!(md.contains("title: \"My Title\""), "got: {md}");
    assert!(md.contains("author: \"Author Name\""), "got: {md}");
}

#[test]
fn write_markdown_multi_section_separator() {
    let mut doc = ir::Document::new();
    doc.sections.push(ir::Section {
        blocks: vec![ir::Block::Paragraph {
            inlines: vec![plain("Section 1")],
        }],
    });
    doc.sections.push(ir::Section {
        blocks: vec![ir::Block::Paragraph {
            inlines: vec![plain("Section 2")],
        }],
    });
    let md = write_markdown(&doc, false);
    assert!(md.contains("\n---\n"), "got: {md}");
    assert!(md.contains("Section 1"), "got: {md}");
    assert!(md.contains("Section 2"), "got: {md}");
}

// -----------------------------------------------------------------------
// render_inlines — edge cases
// -----------------------------------------------------------------------

// Issue 1: bold + italic + strikethrough nesting.
// ~~***text***~~ is valid GFM and renders correctly.
#[test]
fn render_inlines_bold_italic_strikethrough() {
    let inlines = vec![ir::Inline {
        text: "all".into(),
        bold: true,
        italic: true,
        strikethrough: true,
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "~~***all***~~");
}

// Issue 2: underline + bold — Markdown bold applied first, HTML <u> wraps.
// GFM processes Markdown inside HTML inline elements, so <u>**text**</u>
// renders as underlined bold.
#[test]
fn render_inlines_underline_bold_order() {
    let inlines = vec![ir::Inline {
        text: "ub".into(),
        bold: true,
        underline: true,
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "<u>**ub**</u>");
}

// Issue 3: link with bold text — bold wraps the label, not the URL.
#[test]
fn render_inlines_bold_link() {
    let inlines = vec![ir::Inline {
        text: "click".into(),
        bold: true,
        link: Some("https://example.com".into()),
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "[**click**](https://example.com)");
}

// Issue 4: footnote_ref appended after formatted text.
#[test]
fn render_inlines_bold_then_footnote_ref() {
    let inlines = vec![ir::Inline {
        text: "note".into(),
        bold: true,
        footnote_ref: Some("fn1".into()),
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "**note**[^fn1]");
}

// Issue 5a: empty text with bold must NOT produce `****`.
#[test]
fn render_inlines_empty_text_bold_skips_markers() {
    let inlines = vec![ir::Inline {
        text: String::new(),
        bold: true,
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "");
}

// Issue 5b: empty text + bold + footnote_ref — formatting skipped,
// footnote reference still emitted.
#[test]
fn render_inlines_empty_text_bold_with_footnote_ref() {
    let inlines = vec![ir::Inline {
        text: String::new(),
        bold: true,
        footnote_ref: Some("2".into()),
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "[^2]");
}

// Issue 5c: empty text with link — [](url) is preserved as-is.
#[test]
fn render_inlines_empty_text_link() {
    let inlines = vec![ir::Inline {
        text: String::new(),
        link: Some("https://example.com".into()),
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "[](https://example.com)");
}

// Issue 6: Markdown metacharacters in inline text must be escaped.
#[test]
fn render_inlines_escapes_asterisk() {
    assert_eq!(render_inlines(&[plain("a*b")]), r"a\*b");
}

#[test]
fn render_inlines_escapes_underscore() {
    assert_eq!(render_inlines(&[plain("a_b")]), r"a\_b");
}

#[test]
fn render_inlines_escapes_tilde() {
    assert_eq!(render_inlines(&[plain("a~b")]), r"a\~b");
}

#[test]
fn render_inlines_escapes_brackets() {
    assert_eq!(render_inlines(&[plain("[link]")]), r"\[link\]");
}

#[test]
fn render_inlines_escapes_backtick() {
    assert_eq!(render_inlines(&[plain("a`b")]), "a\\`b");
}

#[test]
fn render_inlines_escapes_backslash() {
    assert_eq!(render_inlines(&[plain(r"a\b")]), r"a\\b");
}

// -----------------------------------------------------------------------
// escape_paragraph_line_start — # and > at line start
// -----------------------------------------------------------------------

#[test]
fn paragraph_starting_with_hash_space_is_escaped() {
    let doc = make_doc_with_blocks(vec![ir::Block::Paragraph {
        inlines: vec![plain("# not a heading")],
    }]);
    let md = write_markdown(&doc, false);
    assert!(
        md.contains("\\# not a heading"),
        "expected escaped #; got: {md:?}"
    );
}

#[test]
fn paragraph_starting_with_double_hash_is_escaped() {
    let doc = make_doc_with_blocks(vec![ir::Block::Paragraph {
        inlines: vec![plain("## still not a heading")],
    }]);
    let md = write_markdown(&doc, false);
    assert!(
        md.contains("\\## still not a heading"),
        "expected escaped ##; got: {md:?}"
    );
}

#[test]
fn paragraph_starting_with_gt_is_escaped() {
    let doc = make_doc_with_blocks(vec![ir::Block::Paragraph {
        inlines: vec![plain("> not a blockquote")],
    }]);
    let md = write_markdown(&doc, false);
    assert!(
        md.contains("\\> not a blockquote"),
        "expected escaped >; got: {md:?}"
    );
}

#[test]
fn paragraph_hash_in_middle_not_escaped() {
    let doc = make_doc_with_blocks(vec![ir::Block::Paragraph {
        inlines: vec![plain("color #ff0000")],
    }]);
    let md = write_markdown(&doc, false);
    assert!(
        md.contains("color #ff0000"),
        "mid-text # must not be escaped; got: {md:?}"
    );
    assert!(
        !md.contains("\\#"),
        "unexpected escape of mid-text #; got: {md:?}"
    );
}

#[test]
fn paragraph_gt_in_middle_not_escaped() {
    let doc = make_doc_with_blocks(vec![ir::Block::Paragraph {
        inlines: vec![plain("a > b")],
    }]);
    let md = write_markdown(&doc, false);
    assert!(
        md.contains("a > b"),
        "mid-text > must not be escaped; got: {md:?}"
    );
    assert!(
        !md.contains("\\>"),
        "unexpected escape of mid-text >; got: {md:?}"
    );
}

// Code spans bypass escaping — raw text is wrapped in backticks.
#[test]
fn render_inlines_code_no_escape() {
    let inlines = vec![ir::Inline {
        text: "a*b_c".into(),
        code: true,
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "`a*b_c`");
}

// -----------------------------------------------------------------------
// escape_inline unit tests
// -----------------------------------------------------------------------

#[test]
fn escape_inline_no_special_chars() {
    assert_eq!(escape_inline("hello world"), "hello world");
}

#[test]
fn escape_inline_all_special_chars() {
    // Input:  \  `  *  _  ~  [  ]
    // Output: \\ \` \* \_ \~ \[ \]
    let input = "\\`*_~[]";
    let expected = "\\\\\\`\\*\\_\\~\\[\\]";
    assert_eq!(escape_inline(input), expected);
}

#[test]
fn escape_inline_mixed() {
    // Asterisks in plain text must be escaped.
    assert_eq!(escape_inline("price: *$5*"), r"price: \*$5\*");
}

// -----------------------------------------------------------------------
// write_frontmatter — additional metadata fields
// -----------------------------------------------------------------------

#[test]
fn write_markdown_frontmatter_with_created_date() {
    let mut doc = ir::Document::new();
    doc.metadata.title = Some("Doc".into());
    doc.metadata.created = Some("2026-04-22".into());
    doc.sections.push(ir::Section { blocks: Vec::new() });
    let md = write_markdown(&doc, true);
    assert!(md.contains("date: \"2026-04-22\""), "got: {md}");
}

#[test]
fn write_markdown_frontmatter_with_subject() {
    let mut doc = ir::Document::new();
    doc.metadata.subject = Some("The subject".into());
    doc.sections.push(ir::Section { blocks: Vec::new() });
    let md = write_markdown(&doc, true);
    assert!(md.contains("subject: \"The subject\""), "got: {md}");
}

#[test]
fn write_markdown_frontmatter_with_description() {
    let mut doc = ir::Document::new();
    doc.metadata.description = Some("A description".into());
    doc.sections.push(ir::Section { blocks: Vec::new() });
    let md = write_markdown(&doc, true);
    assert!(md.contains("description: \"A description\""), "got: {md}");
}

#[test]
fn write_markdown_frontmatter_with_keywords() {
    let mut doc = ir::Document::new();
    doc.metadata.keywords = vec!["rust".into(), "hwp".into(), "converter".into()];
    doc.sections.push(ir::Section { blocks: Vec::new() });
    let md = write_markdown(&doc, true);
    assert!(md.contains("keywords:"), "got: {md}");
    assert!(md.contains("rust"), "got: {md}");
    assert!(md.contains("hwp"), "got: {md}");
    assert!(md.contains("converter"), "got: {md}");
}

#[test]
fn write_markdown_frontmatter_all_fields() {
    let mut doc = ir::Document::new();
    doc.metadata.title = Some("Full".into());
    doc.metadata.author = Some("Author".into());
    doc.metadata.created = Some("2026-01-01".into());
    doc.metadata.subject = Some("Subj".into());
    doc.metadata.description = Some("Desc".into());
    doc.metadata.keywords = vec!["a".into(), "b".into()];
    doc.sections.push(ir::Section { blocks: Vec::new() });
    let md = write_markdown(&doc, true);
    assert!(md.starts_with("---\n"), "got: {md}");
    assert!(md.contains("title:"), "got: {md}");
    assert!(md.contains("author:"), "got: {md}");
    assert!(md.contains("date:"), "got: {md}");
    assert!(md.contains("subject:"), "got: {md}");
    assert!(md.contains("description:"), "got: {md}");
    assert!(md.contains("keywords:"), "got: {md}");
}

#[test]
fn write_markdown_frontmatter_no_fields_emits_empty_block() {
    let mut doc = ir::Document::new();
    // No metadata fields set.
    doc.sections.push(ir::Section { blocks: Vec::new() });
    let md = write_markdown(&doc, true);
    // Should start with ---\n and end with ---\n\n even with no fields.
    assert!(md.starts_with("---\n---\n"), "got: {md}");
}

// -----------------------------------------------------------------------
// cell_to_text — non-paragraph block types inside cells
// -----------------------------------------------------------------------

#[test]
fn write_markdown_table_cell_with_code_block_uses_fallback_text() {
    // A cell containing a CodeBlock triggers cell_to_text's fallback branch.
    let rows = vec![
        ir::TableRow {
            cells: vec![ir::TableCell {
                blocks: vec![ir::Block::CodeBlock {
                    language: Some("rust".into()),
                    code: "let x = 1;".into(),
                }],
                ..Default::default()
            }],
            is_header: true,
        },
        ir::TableRow {
            cells: vec![ir::TableCell {
                blocks: vec![ir::Block::Paragraph {
                    inlines: vec![plain("data")],
                }],
                ..Default::default()
            }],
            is_header: false,
        },
    ];
    let doc = make_doc_with_blocks(vec![ir::Block::Table { rows, col_count: 1 }]);
    let md = write_markdown(&doc, false);
    // Should contain the code content (or at minimum not panic).
    assert!(md.contains("let x = 1;") || md.contains("```"), "got: {md}");
}

#[test]
fn write_markdown_table_cell_with_image_uses_fallback_text() {
    let rows = vec![ir::TableRow {
        cells: vec![ir::TableCell {
            blocks: vec![ir::Block::Image {
                src: "img.png".into(),
                alt: "photo".into(),
            }],
            ..Default::default()
        }],
        is_header: true,
    }];
    let doc = make_doc_with_blocks(vec![ir::Block::Table { rows, col_count: 1 }]);
    let md = write_markdown(&doc, false);
    assert!(md.contains("img.png") || md.contains("photo"), "got: {md}");
}

#[test]
fn write_markdown_table_cell_with_math_block() {
    let rows = vec![ir::TableRow {
        cells: vec![ir::TableCell {
            blocks: vec![ir::Block::Math {
                display: true,
                tex: "E=mc^2".into(),
            }],
            ..Default::default()
        }],
        is_header: true,
    }];
    let doc = make_doc_with_blocks(vec![ir::Block::Table { rows, col_count: 1 }]);
    let md = write_markdown(&doc, false);
    assert!(md.contains("E=mc^2"), "got: {md}");
}

// -----------------------------------------------------------------------
// write_block — BlockQuote with nested content
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
// write_list — items with multiple blocks (continuation indent)
// -----------------------------------------------------------------------

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
// HTML table — rowspan attribute
// -----------------------------------------------------------------------

#[test]
fn write_markdown_html_table_with_rowspan() {
    let rows = vec![
        ir::TableRow {
            cells: vec![ir::TableCell {
                blocks: vec![ir::Block::Paragraph {
                    inlines: vec![plain("header")],
                }],
                colspan: 1,
                rowspan: 2,
            }],
            is_header: true,
        },
        ir::TableRow {
            cells: vec![ir::TableCell {
                blocks: vec![ir::Block::Paragraph {
                    inlines: vec![plain("body")],
                }],
                colspan: 1,
                rowspan: 1,
            }],
            is_header: false,
        },
    ];
    let doc = make_doc_with_blocks(vec![ir::Block::Table { rows, col_count: 1 }]);
    let md = write_markdown(&doc, false);
    assert!(md.contains("<table>"), "got: {md}");
    assert!(md.contains("rowspan=\"2\""), "got: {md}");
}

// -----------------------------------------------------------------------
// GFM table — pipe-escaped cell content
// -----------------------------------------------------------------------

#[test]
fn write_markdown_table_cell_pipe_escaped() {
    let rows = vec![ir::TableRow {
        cells: vec![ir::TableCell {
            blocks: vec![ir::Block::Paragraph {
                inlines: vec![plain("a | b")],
            }],
            ..Default::default()
        }],
        is_header: true,
    }];
    let doc = make_doc_with_blocks(vec![ir::Block::Table { rows, col_count: 1 }]);
    let md = write_markdown(&doc, false);
    // The | inside cell text must be escaped to \|
    assert!(md.contains("\\|"), "pipe must be escaped; got: {md}");
}

// -----------------------------------------------------------------------
// escape_paragraph_line_start — multiline handling
// -----------------------------------------------------------------------

#[test]
fn paragraph_multiline_second_line_hash_escaped() {
    // The second line starts with # — it must also be escaped.
    let doc = make_doc_with_blocks(vec![ir::Block::Paragraph {
        inlines: vec![plain("normal line\n# second line heading")],
    }]);
    let md = write_markdown(&doc, false);
    assert!(
        md.contains("\\# second line heading"),
        "second-line # must be escaped; got: {md:?}"
    );
    assert!(
        md.contains("normal line"),
        "first line must be preserved; got: {md:?}"
    );
}

#[test]
fn paragraph_multiline_second_line_gt_escaped() {
    let doc = make_doc_with_blocks(vec![ir::Block::Paragraph {
        inlines: vec![plain("first\n> second")],
    }]);
    let md = write_markdown(&doc, false);
    assert!(
        md.contains("\\> second"),
        "second-line > must be escaped; got: {md:?}"
    );
}

#[test]
fn paragraph_multiline_list_marker_escaped() {
    // A line starting with "- " is a list marker and must be escaped.
    let doc = make_doc_with_blocks(vec![ir::Block::Paragraph {
        inlines: vec![plain("text\n- list item")],
    }]);
    let md = write_markdown(&doc, false);
    assert!(
        md.contains("\\- list item"),
        "second-line list marker must be escaped; got: {md:?}"
    );
}

#[test]
fn paragraph_multiline_ordered_list_marker_escaped() {
    // A line matching digit+"." is an ordered list marker and must be escaped.
    let doc = make_doc_with_blocks(vec![ir::Block::Paragraph {
        inlines: vec![plain("text\n1. first item")],
    }]);
    let md = write_markdown(&doc, false);
    assert!(
        md.contains("\\1. first item"),
        "ordered list marker must be escaped; got: {md:?}"
    );
}

#[test]
fn paragraph_multiline_thematic_break_escaped() {
    let doc = make_doc_with_blocks(vec![ir::Block::Paragraph {
        inlines: vec![plain("text\n---")],
    }]);
    let md = write_markdown(&doc, false);
    assert!(
        md.contains("\\---"),
        "thematic break must be escaped; got: {md:?}"
    );
}

#[test]
fn paragraph_first_line_normal_not_double_escaped() {
    // If the first line is plain, no backslash should be prepended.
    let doc = make_doc_with_blocks(vec![ir::Block::Paragraph {
        inlines: vec![plain("hello\nworld")],
    }]);
    let md = write_markdown(&doc, false);
    assert!(
        md.contains("hello\nworld"),
        "plain multiline must not be escaped; got: {md:?}"
    );
}

// -----------------------------------------------------------------------
// render_inlines — color field
// -----------------------------------------------------------------------

#[test]
fn render_inlines_color_wraps_in_span() {
    let inlines = vec![ir::Inline {
        text: "red".into(),
        color: Some("#FF0000".into()),
        ..Default::default()
    }];
    assert_eq!(
        render_inlines(&inlines),
        "<span style=\"color:#FF0000\">red</span>"
    );
}

#[test]
fn render_inlines_no_color_no_span() {
    let inlines = vec![ir::Inline {
        text: "plain".into(),
        color: None,
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "plain");
}

#[test]
fn render_inlines_color_with_bold_span_wraps_bold() {
    // Bold decoration is applied first; the span wraps the fully-decorated text.
    let inlines = vec![ir::Inline {
        text: "bold red".into(),
        bold: true,
        color: Some("#FF0000".into()),
        ..Default::default()
    }];
    assert_eq!(
        render_inlines(&inlines),
        "<span style=\"color:#FF0000\">**bold red**</span>"
    );
}

#[test]
fn render_inlines_empty_color_string_no_span() {
    // An empty string in color must not produce a <span>.
    let inlines = vec![ir::Inline {
        text: "text".into(),
        color: Some(String::new()),
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "text");
}

#[test]
fn render_inlines_color_applied_before_link() {
    // Color span wraps the label; the outer [label](url) form is applied after.
    let inlines = vec![ir::Inline {
        text: "click".into(),
        color: Some("#0000FF".into()),
        link: Some("https://example.com".into()),
        ..Default::default()
    }];
    let out = render_inlines(&inlines);
    // The span must be inside the link label, not wrapping the `[label](url)`.
    assert_eq!(
        out,
        "[<span style=\"color:#0000FF\">click</span>](https://example.com)"
    );
}
