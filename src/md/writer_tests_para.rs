use super::*;
use crate::ir;

// -----------------------------------------------------------------------
// Shared helpers
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
// Code spans bypass escaping
// -----------------------------------------------------------------------

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
    doc.sections.push(ir::Section {
        blocks: Vec::new(),
        page_layout: None,
        ..Default::default()
    });
    let md = write_markdown(&doc, true);
    assert!(md.contains("date: \"2026-04-22\""), "got: {md}");
}

#[test]
fn write_markdown_frontmatter_with_subject() {
    let mut doc = ir::Document::new();
    doc.metadata.subject = Some("The subject".into());
    doc.sections.push(ir::Section {
        blocks: Vec::new(),
        page_layout: None,
        ..Default::default()
    });
    let md = write_markdown(&doc, true);
    assert!(md.contains("subject: \"The subject\""), "got: {md}");
}

#[test]
fn write_markdown_frontmatter_with_description() {
    let mut doc = ir::Document::new();
    doc.metadata.description = Some("A description".into());
    doc.sections.push(ir::Section {
        blocks: Vec::new(),
        page_layout: None,
        ..Default::default()
    });
    let md = write_markdown(&doc, true);
    assert!(md.contains("description: \"A description\""), "got: {md}");
}

#[test]
fn write_markdown_frontmatter_with_keywords() {
    let mut doc = ir::Document::new();
    doc.metadata.keywords = vec!["rust".into(), "hwp".into(), "converter".into()];
    doc.sections.push(ir::Section {
        blocks: Vec::new(),
        page_layout: None,
        ..Default::default()
    });
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
    doc.sections.push(ir::Section {
        blocks: Vec::new(),
        page_layout: None,
        ..Default::default()
    });
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
    doc.sections.push(ir::Section {
        blocks: Vec::new(),
        page_layout: None,
        ..Default::default()
    });
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
    let doc = make_doc_with_blocks(vec![ir::Block::Table {
        rows,
        col_count: 1,
        inner_margin: None,
    }]);
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
    let doc = make_doc_with_blocks(vec![ir::Block::Table {
        rows,
        col_count: 1,
        inner_margin: None,
    }]);
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
    let doc = make_doc_with_blocks(vec![ir::Block::Table {
        rows,
        col_count: 1,
        inner_margin: None,
    }]);
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
            checked: None,
        }],
    }]);
    let md = write_markdown(&doc, false);
    assert!(md.contains("first block"), "got: {md}");
    assert!(md.contains("continuation block"), "got: {md}");
}

// -----------------------------------------------------------------------
// Security S2: HTML table cell text must be entity-escaped
// -----------------------------------------------------------------------

#[test]
fn write_markdown_html_table_cell_script_tag_is_escaped() {
    // A cell containing a <script> tag must NOT appear verbatim in the output.
    let rows = vec![ir::TableRow {
        cells: vec![ir::TableCell {
            blocks: vec![ir::Block::Paragraph {
                inlines: vec![plain("<script>alert(1)</script>")],
            }],
            colspan: 2,
            rowspan: 1,
        }],
        is_header: true,
    }];
    let doc = make_doc_with_blocks(vec![ir::Block::Table {
        rows,
        col_count: 2,
        inner_margin: None,
    }]);
    let md = write_markdown(&doc, false);
    assert!(
        md.contains("<table>"),
        "must use HTML table path; got: {md}"
    );
    assert!(
        !md.contains("<script>"),
        "raw <script> tag must be entity-escaped; got: {md}"
    );
    assert!(
        md.contains("&lt;script&gt;"),
        "must contain entity-escaped &lt;script&gt;; got: {md}"
    );
}

#[test]
fn write_markdown_html_table_cell_ampersand_escaped() {
    let rows = vec![ir::TableRow {
        cells: vec![ir::TableCell {
            blocks: vec![ir::Block::Paragraph {
                inlines: vec![plain("AT&T")],
            }],
            colspan: 2,
            rowspan: 1,
        }],
        is_header: true,
    }];
    let doc = make_doc_with_blocks(vec![ir::Block::Table {
        rows,
        col_count: 2,
        inner_margin: None,
    }]);
    let md = write_markdown(&doc, false);
    assert!(
        md.contains("&amp;"),
        "& must be escaped to &amp;; got: {md}"
    );
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
    let doc = make_doc_with_blocks(vec![ir::Block::Table {
        rows,
        col_count: 1,
        inner_margin: None,
    }]);
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
    let doc = make_doc_with_blocks(vec![ir::Block::Table {
        rows,
        col_count: 1,
        inner_margin: None,
    }]);
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

// -----------------------------------------------------------------------
// L2 fix: inline math block must emit a trailing blank line
// -----------------------------------------------------------------------

#[test]
fn write_markdown_inline_math_has_trailing_blank_line() {
    let doc = make_doc_with_blocks(vec![ir::Block::Math {
        display: false,
        tex: "x+y".into(),
    }]);
    let md = write_markdown(&doc, false);
    // Must end with two newlines so the next block is separated.
    assert!(
        md.contains("$x+y$\n\n"),
        "inline math must be followed by a blank line; got: {md:?}"
    );
}

#[test]
fn write_markdown_inline_math_followed_by_paragraph_has_blank_line() {
    let doc = make_doc_with_blocks(vec![
        ir::Block::Math {
            display: false,
            tex: "a^2".into(),
        },
        ir::Block::Paragraph {
            inlines: vec![plain("next paragraph")],
        },
    ]);
    let md = write_markdown(&doc, false);
    // There must be a blank line between the math block and the paragraph.
    assert!(
        md.contains("$a^2$\n\nnext paragraph"),
        "inline math and following paragraph must be separated by a blank line; got: {md:?}"
    );
}

// -----------------------------------------------------------------------
// L3 fix: HTML table header uses row.is_header, not row index
// -----------------------------------------------------------------------

#[test]
fn write_markdown_html_table_second_row_is_header_uses_th() {
    // Only the second row is marked is_header — first row should use <td>.
    let rows = vec![
        ir::TableRow {
            cells: vec![ir::TableCell {
                blocks: vec![ir::Block::Paragraph {
                    inlines: vec![plain("data")],
                }],
                colspan: 2, // force HTML fallback
                rowspan: 1,
            }],
            is_header: false,
        },
        ir::TableRow {
            cells: vec![ir::TableCell {
                blocks: vec![ir::Block::Paragraph {
                    inlines: vec![plain("header")],
                }],
                colspan: 1,
                rowspan: 1,
            }],
            is_header: true,
        },
    ];
    let doc = make_doc_with_blocks(vec![ir::Block::Table {
        rows,
        col_count: 2,
        inner_margin: None,
    }]);
    let md = write_markdown(&doc, false);
    // First row (is_header=false) → <td…>; second row (is_header=true) → <th…>
    let first_td_pos = md.find("<td").expect("<td must appear in output");
    let second_th_pos = md.rfind("<th").expect("<th must appear in output");
    assert!(
        first_td_pos < second_th_pos,
        "<td (row 0) must appear before <th (row 1); got: {md:?}"
    );
}

#[test]
fn write_markdown_html_table_first_row_not_header_uses_td() {
    // When is_header=false on row 0, it must NOT use <th>.
    let rows = vec![ir::TableRow {
        cells: vec![ir::TableCell {
            blocks: vec![ir::Block::Paragraph {
                inlines: vec![plain("cell")],
            }],
            colspan: 2,
            rowspan: 1,
        }],
        is_header: false,
    }];
    let doc = make_doc_with_blocks(vec![ir::Block::Table {
        rows,
        col_count: 2,
        inner_margin: None,
    }]);
    let md = write_markdown(&doc, false);
    assert!(
        md.contains("<td"),
        "row with is_header=false must use <td…>; got: {md:?}"
    );
    assert!(
        !md.contains("<th"),
        "row with is_header=false must not use <th…>; got: {md:?}"
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

// -----------------------------------------------------------------------
// render_inlines — ruby annotation field
// -----------------------------------------------------------------------

#[test]
fn render_inlines_ruby_annotation_produces_html_tags() {
    let inlines = vec![ir::Inline {
        text: "漢字".into(),
        ruby: Some("한자".into()),
        ..ir::Inline::default()
    }];
    assert_eq!(render_inlines(&inlines), "<ruby>漢字<rt>한자</rt></ruby>");
}

#[test]
fn render_inlines_ruby_annotation_none_no_ruby_tags() {
    let inlines = vec![ir::Inline {
        text: "漢字".into(),
        ruby: None,
        ..ir::Inline::default()
    }];
    let out = render_inlines(&inlines);
    assert!(
        !out.contains("<ruby>"),
        "no ruby tags when annotation is None; got: {out}"
    );
    assert!(
        out.contains("漢字"),
        "base text must still appear; got: {out}"
    );
}

#[test]
fn render_inlines_ruby_empty_annotation_no_ruby_tags() {
    // Some("") — annotation present but empty — must not emit HTML ruby tags.
    let inlines = vec![ir::Inline {
        text: "漢字".into(),
        ruby: Some(String::new()),
        ..ir::Inline::default()
    }];
    let out = render_inlines(&inlines);
    assert!(
        !out.contains("<ruby>"),
        "empty annotation must not emit ruby tags; got: {out}"
    );
}

#[test]
fn render_inlines_ruby_annotation_html_special_chars_escaped() {
    // Annotation text containing '<', '>', '&' must be entity-escaped.
    let inlines = vec![ir::Inline {
        text: "base".into(),
        ruby: Some("a<b>&c".into()),
        ..ir::Inline::default()
    }];
    let out = render_inlines(&inlines);
    assert!(out.contains("&lt;"), "< must be escaped; got: {out}");
    assert!(out.contains("&gt;"), "> must be escaped; got: {out}");
    assert!(out.contains("&amp;"), "& must be escaped; got: {out}");
    assert!(
        !out.contains("<b>"),
        "raw < must not appear unescaped; got: {out}"
    );
}

#[test]
fn render_inlines_ruby_with_bold_base_text() {
    // Bold decoration on base text applies before the ruby wrapper.
    let inlines = vec![ir::Inline {
        text: "漢字".into(),
        bold: true,
        ruby: Some("한자".into()),
        ..ir::Inline::default()
    }];
    let out = render_inlines(&inlines);
    // Bold markers must be inside the <ruby> wrapper.
    assert!(
        out.contains("<ruby>**漢字**<rt>한자</rt></ruby>"),
        "got: {out}"
    );
}

#[test]
fn write_markdown_paragraph_with_ruby_inline() {
    let doc = make_doc_with_blocks(vec![ir::Block::Paragraph {
        inlines: vec![ir::Inline {
            text: "漢字".into(),
            ruby: Some("한자".into()),
            ..ir::Inline::default()
        }],
    }]);
    let md = write_markdown(&doc, false);
    assert!(
        md.contains("<ruby>漢字<rt>한자</rt></ruby>"),
        "paragraph must contain ruby HTML; got: {md}"
    );
}
