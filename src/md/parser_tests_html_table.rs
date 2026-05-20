//! Tests for the HTML `<table>` parser (`html_table::parse_html_table`).
//!
//! S45-06: Happy-path tests
//! S45-07: Negative / edge-case tests
//! S45-08: Round-trip tests (write IR → Markdown → parse back → compare)

use crate::md::html_table::parse_html_table;
use crate::md::write_markdown;
use super::tests::first_section_blocks;
use super::*;
use crate::ir;

// ── helpers ─────────────────────────────────────────────────────────────────

/// Extract text from the first `Inline::plain` run inside a `TableCell`.
fn cell_text(cell: &ir::TableCell) -> &str {
    match cell.blocks.first() {
        Some(ir::Block::Paragraph { inlines }) => inlines.first().map_or("", |i| i.text.as_str()),
        _ => "",
    }
}

// ── S45-06: Happy path ───────────────────────────────────────────────────────

#[test]
fn parse_html_table_basic_2x2() {
    let html = "<table>\n  <tr><td>A</td><td>B</td></tr>\n  <tr><td>C</td><td>D</td></tr>\n</table>\n";
    let block = parse_html_table(html).expect("should parse 2x2 table");
    if let ir::Block::Table { rows, col_count, .. } = block {
        assert_eq!(rows.len(), 2, "expected 2 rows");
        assert_eq!(col_count, 2, "expected col_count=2");
        assert_eq!(cell_text(&rows[0].cells[0]), "A");
        assert_eq!(cell_text(&rows[0].cells[1]), "B");
        assert_eq!(cell_text(&rows[1].cells[0]), "C");
        assert_eq!(cell_text(&rows[1].cells[1]), "D");
    } else {
        panic!("expected Block::Table, got {block:?}");
    }
}

#[test]
fn parse_html_table_colspan_only() {
    let html = "<table><tr><td colspan=\"2\">wide</td></tr></table>\n";
    let block = parse_html_table(html).expect("should parse colspan table");
    if let ir::Block::Table { rows, col_count, .. } = block {
        assert_eq!(col_count, 2, "col_count should reflect colspan");
        let cell = &rows[0].cells[0];
        assert_eq!(cell.colspan, 2, "cell colspan must be 2");
        assert_eq!(cell.rowspan, 1, "rowspan defaults to 1");
        assert_eq!(cell_text(cell), "wide");
    } else {
        panic!("expected Block::Table");
    }
}

#[test]
fn parse_html_table_rowspan_only() {
    let html = "<table><tr><td rowspan=\"2\">tall</td><td>B</td></tr><tr><td>C</td></tr></table>\n";
    let block = parse_html_table(html).expect("should parse rowspan table");
    if let ir::Block::Table { rows, .. } = block {
        let cell = &rows[0].cells[0];
        assert_eq!(cell.rowspan, 2, "rowspan must be 2");
        assert_eq!(cell.colspan, 1, "colspan defaults to 1");
        assert_eq!(cell_text(cell), "tall");
    } else {
        panic!("expected Block::Table");
    }
}

#[test]
fn parse_html_table_colspan_and_rowspan() {
    let html = "<table><tr><td colspan=\"3\" rowspan=\"2\">big</td></tr></table>\n";
    let block = parse_html_table(html).expect("should parse colspan+rowspan table");
    if let ir::Block::Table { rows, col_count, .. } = block {
        let cell = &rows[0].cells[0];
        assert_eq!(cell.colspan, 3);
        assert_eq!(cell.rowspan, 2);
        assert_eq!(col_count, 3);
        assert_eq!(cell_text(cell), "big");
    } else {
        panic!("expected Block::Table");
    }
}

#[test]
fn parse_html_table_header_detection_all_th() {
    let html = "<table><tr><th>H1</th><th>H2</th></tr><tr><td>D1</td><td>D2</td></tr></table>\n";
    let block = parse_html_table(html).expect("should parse header table");
    if let ir::Block::Table { rows, .. } = block {
        assert_eq!(rows.len(), 2);
        assert!(rows[0].is_header, "row with all <th> must be is_header=true");
        assert!(!rows[1].is_header, "row with all <td> must be is_header=false");
    } else {
        panic!("expected Block::Table");
    }
}

#[test]
fn parse_html_table_header_detection_mixed_cells() {
    // A row that mixes <th> and <td> is NOT a header row.
    let html = "<table><tr><th>H1</th><td>D2</td></tr></table>\n";
    let block = parse_html_table(html).expect("should parse mixed-cell table");
    if let ir::Block::Table { rows, .. } = block {
        assert!(
            !rows[0].is_header,
            "row with mixed <th>/<td> must be is_header=false"
        );
    } else {
        panic!("expected Block::Table");
    }
}

#[test]
fn parse_html_table_thead_tbody_wrappers() {
    // <thead>/<tbody> wrappers should be ignored — rows are treated as direct children.
    let html = "<table>\
        <thead><tr><th>H1</th><th>H2</th></tr></thead>\
        <tbody><tr><td>D1</td><td>D2</td></tr></tbody>\
        </table>\n";
    let block = parse_html_table(html).expect("should parse thead/tbody table");
    if let ir::Block::Table { rows, col_count, .. } = block {
        assert_eq!(rows.len(), 2, "should have 2 rows (one from thead, one from tbody)");
        assert_eq!(col_count, 2);
        assert!(rows[0].is_header);
        assert!(!rows[1].is_header);
    } else {
        panic!("expected Block::Table");
    }
}

#[test]
fn parse_html_table_entity_decoded_text() {
    // &amp; in cell text must be decoded to '&' in the IR.
    let html = "<table><tr><td>A &amp; B</td></tr></table>\n";
    let block = parse_html_table(html).expect("should parse entity table");
    if let ir::Block::Table { rows, .. } = block {
        let text = cell_text(&rows[0].cells[0]);
        assert_eq!(text, "A & B", "HTML entity must be decoded");
    } else {
        panic!("expected Block::Table");
    }
}

#[test]
fn parse_html_table_attribute_order_variations() {
    // rowspan before colspan — both must be parsed correctly.
    let html = "<table><tr><td rowspan=\"3\" colspan=\"2\">x</td></tr></table>\n";
    let block = parse_html_table(html).expect("should parse reversed-attr table");
    if let ir::Block::Table { rows, .. } = block {
        let cell = &rows[0].cells[0];
        assert_eq!(cell.rowspan, 3, "rowspan must be 3");
        assert_eq!(cell.colspan, 2, "colspan must be 2");
    } else {
        panic!("expected Block::Table");
    }
}

#[test]
fn parse_html_table_no_spans() {
    // Table without any span attributes — all cells default to colspan=1 rowspan=1.
    let html = "<table><tr><td>A</td><td>B</td><td>C</td></tr></table>\n";
    let block = parse_html_table(html).expect("should parse no-span table");
    if let ir::Block::Table { rows, col_count, .. } = block {
        assert_eq!(col_count, 3);
        for cell in &rows[0].cells {
            assert_eq!(cell.colspan, 1);
            assert_eq!(cell.rowspan, 1);
        }
    } else {
        panic!("expected Block::Table");
    }
}

#[test]
fn parse_html_table_col_count_from_colspan() {
    // Single row, single cell with colspan=3 → col_count must be 3.
    let html = "<table><tr><td colspan=\"3\">all</td></tr></table>\n";
    let block = parse_html_table(html).expect("should parse colspan col_count table");
    if let ir::Block::Table { col_count, .. } = block {
        assert_eq!(col_count, 3, "col_count should be sum of colspan in widest row");
    } else {
        panic!("expected Block::Table");
    }
}

// ── S45-07: Negative / edge cases ────────────────────────────────────────────

#[test]
fn parse_html_table_non_table_html_returns_none() {
    let html = "<div>foo</div>\n";
    assert!(
        parse_html_table(html).is_none(),
        "non-<table> HTML must return None"
    );
}

#[test]
fn parse_html_table_empty_table_returns_none() {
    let html = "<table></table>\n";
    assert!(
        parse_html_table(html).is_none(),
        "empty <table> must return None"
    );
}

#[test]
fn parse_html_table_span_zero_clamped_to_one() {
    // colspan="0" must be clamped to 1 (guard against invalid HTML).
    let html = "<table><tr><td colspan=\"0\">x</td></tr></table>\n";
    let block = parse_html_table(html).expect("should parse colspan=0 table");
    if let ir::Block::Table { rows, .. } = block {
        let cell = &rows[0].cells[0];
        assert_eq!(cell.colspan, 1, "colspan=0 must be clamped to 1");
    } else {
        panic!("expected Block::Table");
    }
}

#[test]
fn parse_html_table_empty_tr_skipped() {
    // An empty <tr></tr> (no cells) must not produce a row in the IR.
    let html = "<table><tr></tr><tr><td>real</td></tr></table>\n";
    let block = parse_html_table(html).expect("should parse table with empty tr");
    if let ir::Block::Table { rows, .. } = block {
        assert_eq!(rows.len(), 1, "empty <tr> must be skipped");
        assert_eq!(cell_text(&rows[0].cells[0]), "real");
    } else {
        panic!("expected Block::Table");
    }
}

#[test]
fn parse_html_table_non_numeric_span_defaults_to_one() {
    // colspan="wide" is not a number → default to 1.
    let html = "<table><tr><td colspan=\"wide\">x</td></tr></table>\n";
    let block = parse_html_table(html).expect("should parse non-numeric colspan table");
    if let ir::Block::Table { rows, .. } = block {
        let cell = &rows[0].cells[0];
        assert_eq!(cell.colspan, 1, "non-numeric colspan must default to 1");
    } else {
        panic!("expected Block::Table");
    }
}

// ── S45-08: Round-trip tests ─────────────────────────────────────────────────

/// Build an IR table document, write it to Markdown, parse back, and return blocks.
fn roundtrip(rows: Vec<ir::TableRow>, col_count: usize) -> Vec<ir::Block> {
    let doc = ir::Document {
        sections: vec![ir::Section {
            blocks: vec![ir::Block::Table {
                rows,
                col_count,
                inner_margin: None,
            }],
            page_layout: None,
            ..Default::default()
        }],
        ..ir::Document::new()
    };
    let md = write_markdown(&doc, false);
    let parsed = parse_markdown(&md);
    first_section_blocks(&parsed).to_vec()
}

#[test]
fn html_table_roundtrip_no_spans() {
    // A simple 2-row, 2-col table with no spans.  The writer emits GFM pipe table
    // (no colspan/rowspan → simple path), so we verify the block structure
    // survives the write → parse cycle.
    let rows = vec![
        ir::TableRow {
            cells: vec![
                ir::TableCell {
                    blocks: vec![ir::Block::Paragraph {
                        inlines: vec![ir::Inline::plain("A")],
                    }],
                    colspan: 1,
                    rowspan: 1,
                },
                ir::TableCell {
                    blocks: vec![ir::Block::Paragraph {
                        inlines: vec![ir::Inline::plain("B")],
                    }],
                    colspan: 1,
                    rowspan: 1,
                },
            ],
            is_header: false,
        },
        ir::TableRow {
            cells: vec![
                ir::TableCell {
                    blocks: vec![ir::Block::Paragraph {
                        inlines: vec![ir::Inline::plain("C")],
                    }],
                    colspan: 1,
                    rowspan: 1,
                },
                ir::TableCell {
                    blocks: vec![ir::Block::Paragraph {
                        inlines: vec![ir::Inline::plain("D")],
                    }],
                    colspan: 1,
                    rowspan: 1,
                },
            ],
            is_header: false,
        },
    ];
    let blocks = roundtrip(rows, 2);
    let table = blocks.iter().find_map(|b| {
        if let ir::Block::Table { rows, col_count, .. } = b {
            Some((rows, *col_count))
        } else {
            None
        }
    });
    let (rt_rows, rt_col_count) = table.expect("round-trip must produce a Table block");
    assert_eq!(rt_rows.len(), 2, "round-trip row count must be 2");
    assert_eq!(rt_col_count, 2, "round-trip col_count must be 2");
}

#[test]
fn html_table_roundtrip_colspan() {
    // colspan=2 forces HTML table emission → the parser must recover it.
    let rows = vec![
        ir::TableRow {
            cells: vec![ir::TableCell {
                blocks: vec![ir::Block::Paragraph {
                    inlines: vec![ir::Inline::plain("wide")],
                }],
                colspan: 2,
                rowspan: 1,
            }],
            is_header: false,
        },
        ir::TableRow {
            cells: vec![
                ir::TableCell {
                    blocks: vec![ir::Block::Paragraph {
                        inlines: vec![ir::Inline::plain("L")],
                    }],
                    colspan: 1,
                    rowspan: 1,
                },
                ir::TableCell {
                    blocks: vec![ir::Block::Paragraph {
                        inlines: vec![ir::Inline::plain("R")],
                    }],
                    colspan: 1,
                    rowspan: 1,
                },
            ],
            is_header: false,
        },
    ];
    let blocks = roundtrip(rows, 2);
    let (rt_rows, _) = blocks
        .iter()
        .find_map(|b| {
            if let ir::Block::Table { rows, col_count, .. } = b {
                Some((rows, *col_count))
            } else {
                None
            }
        })
        .expect("round-trip must produce a Table block");

    assert_eq!(rt_rows.len(), 2, "round-trip row count must be 2");
    assert_eq!(rt_rows[0].cells[0].colspan, 2, "colspan must survive round-trip");
    assert_eq!(cell_text(&rt_rows[0].cells[0]), "wide");
}

#[test]
fn html_table_roundtrip_asymmetric_spans() {
    // Both colspan and rowspan in the same cell.
    let rows = vec![ir::TableRow {
        cells: vec![ir::TableCell {
            blocks: vec![ir::Block::Paragraph {
                inlines: vec![ir::Inline::plain("corner")],
            }],
            colspan: 2,
            rowspan: 3,
        }],
        is_header: true,
    }];
    let blocks = roundtrip(rows, 2);
    let (rt_rows, _) = blocks
        .iter()
        .find_map(|b| {
            if let ir::Block::Table { rows, col_count, .. } = b {
                Some((rows, *col_count))
            } else {
                None
            }
        })
        .expect("round-trip must produce a Table block");

    assert_eq!(rt_rows.len(), 1);
    let cell = &rt_rows[0].cells[0];
    assert_eq!(cell.colspan, 2, "colspan must survive round-trip");
    assert_eq!(cell.rowspan, 3, "rowspan must survive round-trip");
    assert_eq!(cell_text(cell), "corner");
}

// ── W3 regression: self-closing <td/> ────────────────────────────────────────

#[test]
fn parse_html_table_self_closing_td_not_dropped() {
    let html = "<table><tr><td/><td>B</td></tr></table>";
    let block = parse_html_table(html).expect("should parse table with self-closing <td/>");
    if let ir::Block::Table { rows, col_count, .. } = block {
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].cells.len(), 2, "self-closing <td/> must not be dropped");
        assert_eq!(col_count, 2);
        assert_eq!(cell_text(&rows[0].cells[0]), "", "self-closing cell has empty text");
        assert_eq!(cell_text(&rows[0].cells[1]), "B");
    } else {
        panic!("expected Block::Table");
    }
}
