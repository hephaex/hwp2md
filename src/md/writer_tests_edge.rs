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
    doc.sections.push(ir::Section { blocks });
    doc
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
    let doc = make_doc_with_blocks(vec![ir::Block::Table { rows, col_count: 2 }]);
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
    let doc = make_doc_with_blocks(vec![ir::Block::Table { rows, col_count: 2 }]);
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
