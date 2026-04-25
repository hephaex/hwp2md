use super::*;
use crate::ir;

// -----------------------------------------------------------------------
// Shared helpers (re-declared here for standalone module use)
// -----------------------------------------------------------------------

fn plain(t: &str) -> ir::Inline {
    ir::Inline::plain(t)
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
    let doc = ir::Document {
        sections: vec![ir::Section {
            blocks: vec![ir::Block::Table { rows, col_count: 2 }],
        }],
        ..ir::Document::new()
    };
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
    let doc = ir::Document {
        sections: vec![ir::Section {
            blocks: vec![ir::Block::Table { rows, col_count: 2 }],
        }],
        ..ir::Document::new()
    };
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
