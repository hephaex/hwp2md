//! HTML `<table>` parser that converts raw HTML markup into [`crate::ir::Block::Table`].
//!
//! This module is invoked from the Markdown parser whenever a [`comrak`]
//! `HtmlBlock` node contains an HTML table literal.  It uses `quick_xml` for
//! streaming SAX-style parsing so that it handles arbitrarily large tables
//! without allocating a full DOM.
//!
//! # Supported subset
//!
//! | Feature | Behaviour |
//! |---------|-----------|
//! | `<table>`, `<tr>`, `<th>`, `<td>` | Parsed |
//! | `<thead>`, `<tbody>`, `<tfoot>` | Ignored — rows treated as direct children |
//! | `colspan` attribute | Parsed, clamped to ≥ 1 |
//! | `rowspan` attribute | Parsed, clamped to ≥ 1 |
//! | HTML entities in cell text | Decoded via `quick_xml` |
//! | Nested `<table>` | Warns and returns `None` |
//!
//! # Known Limitations
//!
//! The following constructs are **not** round-tripped through the IR:
//!
//! | Construct | Behaviour |
//! |-----------|-----------|
//! | Nested `<table>` inside a cell | Returns `None` for the entire outer table |
//! | Block-level content inside cells (`<p>`, `<ul>`, …) | Flattened to plain text |
//! | Inline-formatted content inside cells (`<b>`, `<em>`, …) | Formatting stripped, plain text only |
//! | Unescaped `&` in cell text | May cause `quick_xml` parse error → `None` fallback |
//!
//! # Returns
//!
//! Returns `None` and logs a `tracing::warn!` when:
//! - The input does not start with `<table` (case-insensitive after trimming).
//! - The table has zero rows after parsing.
//! - `quick_xml` reports a parse error.
//! - A nested `<table>` is encountered.

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::ir;

/// Parse an HTML `<table>` literal and return the equivalent [`ir::Block::Table`].
///
/// Returns `None` (with a `tracing::warn!`) when the input is not a valid table.
#[must_use]
pub(crate) fn parse_html_table(literal: &str) -> Option<ir::Block> {
    let trimmed = literal.trim();
    if !trimmed
        .get(..6)
        .is_some_and(|s| s.eq_ignore_ascii_case("<table"))
    {
        tracing::warn!("html_table: input does not start with <table, skipping");
        return None;
    }

    let mut reader = Reader::from_str(trimmed);
    reader.config_mut().check_end_names = false;

    let rows = walk_table_events(&mut reader)?;

    let col_count = rows
        .iter()
        .map(|r| r.cells.iter().map(|c| c.colspan as usize).sum::<usize>())
        .max()
        .unwrap_or(0);

    Some(ir::Block::Table { rows, col_count, inner_margin: None })
}

/// Walk `quick_xml` events for a `<table>` block and return completed rows.
///
/// Returns `None` on nested tables, parse errors, or zero-row results.
#[allow(clippy::too_many_lines)] // streaming state machine; extraction would reduce clarity
fn walk_table_events(reader: &mut Reader<&[u8]>) -> Option<Vec<ir::TableRow>> {
    let mut table_depth: u32 = 0;
    let mut current_row: Option<RowBuilder> = None;
    let mut current_cell: Option<CellBuilder> = None;
    let mut rows: Vec<ir::TableRow> = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let local = local_name(e.name().as_ref());
                match local.to_ascii_lowercase().as_str() {
                    "table" => {
                        table_depth += 1;
                        if table_depth > 1 {
                            tracing::warn!(
                                "html_table: nested <table> not supported, returning None"
                            );
                            return None;
                        }
                    }
                    "tr" => {
                        if let Some(row) = current_row.take() {
                            if !row.cells.is_empty() {
                                rows.push(finish_row(row));
                            }
                        }
                        current_row = Some(RowBuilder::default());
                    }
                    "th" | "td" => {
                        if current_row.is_none() {
                            current_row = Some(RowBuilder::default());
                        }
                        let is_th = local.eq_ignore_ascii_case("th");
                        let (colspan, rowspan) = parse_span_attrs(e);
                        current_cell =
                            Some(CellBuilder { text: String::new(), colspan, rowspan, is_th });
                    }
                    _ => {}
                }
            }
            // Self-closing <td/> / <th/>: no End event will follow — push immediately.
            Ok(Event::Empty(ref e)) => {
                let local = local_name(e.name().as_ref());
                match local.to_ascii_lowercase().as_str() {
                    "table" => {
                        table_depth += 1;
                        if table_depth > 1 {
                            tracing::warn!(
                                "html_table: nested <table> not supported, returning None"
                            );
                            return None;
                        }
                    }
                    "tr" => {
                        if let Some(row) = current_row.take() {
                            if !row.cells.is_empty() {
                                rows.push(finish_row(row));
                            }
                        }
                        current_row = Some(RowBuilder::default());
                    }
                    "th" | "td" => {
                        if current_row.is_none() {
                            current_row = Some(RowBuilder::default());
                        }
                        let is_th = local.eq_ignore_ascii_case("th");
                        let (colspan, rowspan) = parse_span_attrs(e);
                        let cell = CellBuilder { text: String::new(), colspan, rowspan, is_th };
                        if let Some(row) = current_row.as_mut() {
                            row.cells.push(cell);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let local = local_name(e.name().as_ref());
                match local.to_ascii_lowercase().as_str() {
                    "table" => {
                        table_depth = table_depth.saturating_sub(1);
                        if table_depth == 0 {
                            if let Some(row) = current_row.take() {
                                if !row.cells.is_empty() {
                                    rows.push(finish_row(row));
                                }
                            }
                        }
                    }
                    "th" | "td" => {
                        if let Some(cell) = current_cell.take() {
                            if let Some(row) = current_row.as_mut() {
                                row.cells.push(cell);
                            }
                        }
                    }
                    "tr" => {
                        if let Some(row) = current_row.take() {
                            if !row.cells.is_empty() {
                                rows.push(finish_row(row));
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                if let Some(cell) = current_cell.as_mut() {
                    match e.unescape() {
                        Ok(text) => cell.text.push_str(&text),
                        Err(err) => tracing::warn!("html_table: entity decode error: {err}"),
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(err) => {
                tracing::warn!("html_table: quick_xml parse error: {err}");
                return None;
            }
            _ => {}
        }
    }

    if rows.is_empty() {
        tracing::warn!("html_table: table has zero rows, returning None");
        return None;
    }
    Some(rows)
}

// ---- Internal helpers -------------------------------------------------------

/// Extract the local name from a potentially namespace-prefixed tag bytes slice.
///
/// `quick_xml` returns the full qualified name (e.g. `ns:td`); we only need the
/// part after the last colon for case-insensitive comparison.
fn local_name(name: &[u8]) -> String {
    let s = std::str::from_utf8(name).unwrap_or("");
    s.rsplit(':').next().unwrap_or(s).to_owned()
}

/// Parse `colspan` and `rowspan` integer attributes from a start-tag event.
///
/// Missing or non-numeric attributes default to 1.  Zero values are clamped to 1.
fn parse_span_attrs(e: &quick_xml::events::BytesStart<'_>) -> (u32, u32) {
    let mut colspan: u32 = 1;
    let mut rowspan: u32 = 1;

    for attr in e.attributes().flatten() {
        let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
        match key {
            "colspan" => {
                if let Ok(val) = std::str::from_utf8(attr.value.as_ref()) {
                    if let Ok(n) = val.parse::<u32>() {
                        colspan = n.max(1);
                    }
                    // Non-numeric or zero → keep default 1
                }
            }
            "rowspan" => {
                if let Ok(val) = std::str::from_utf8(attr.value.as_ref()) {
                    if let Ok(n) = val.parse::<u32>() {
                        rowspan = n.max(1);
                    }
                }
            }
            _ => {}
        }
    }

    (colspan, rowspan)
}

// ---- Builder types ----------------------------------------------------------

#[derive(Default)]
struct RowBuilder {
    cells: Vec<CellBuilder>,
}

struct CellBuilder {
    text: String,
    colspan: u32,
    rowspan: u32,
    /// `true` when this cell was introduced by `<th>` rather than `<td>`.
    is_th: bool,
}

/// Convert a completed [`RowBuilder`] into an [`ir::TableRow`].
///
/// A row is considered a header row when **all** its cells were `<th>` elements.
fn finish_row(row: RowBuilder) -> ir::TableRow {
    let is_header = !row.cells.is_empty() && row.cells.iter().all(|c| c.is_th);
    let cells = row
        .cells
        .into_iter()
        .map(|c| ir::TableCell {
            blocks: vec![ir::Block::Paragraph {
                inlines: vec![ir::Inline::plain(c.text)],
            }],
            colspan: c.colspan,
            rowspan: c.rowspan,
        })
        .collect();
    ir::TableRow { cells, is_header }
}
