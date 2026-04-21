use crate::hwp::eqedit::eqedit_to_latex;
use crate::hwp::model::*;
use crate::hwp::reader::{extract_paragraph_text, parse_char_shape_refs};
use crate::hwp::record::*;

/// Returns the index one past the last child of `records[parent_idx]`.
///
/// Children are defined as consecutive records with `level > parent_level`.
pub(crate) fn find_children_end(records: &[Record], parent_idx: usize) -> usize {
    let parent_level = records[parent_idx].level;
    let mut idx = parent_idx + 1;
    while idx < records.len() && records[idx].level > parent_level {
        idx += 1;
    }
    idx
}

/// Extract `HwpParagraph`s from records in `[start, end)`, treating them as a
/// self-contained sub-stream (e.g. a table cell or footnote body).
pub(crate) fn extract_paragraphs_from_range(
    records: &[Record],
    start: usize,
    end: usize,
) -> Vec<HwpParagraph> {
    let mut paras: Vec<HwpParagraph> = Vec::new();
    let mut current: Option<HwpParagraph> = None;
    let mut idx = start;

    while idx < end {
        let rec = &records[idx];
        match rec.tag_id {
            HWPTAG_PARA_HEADER => {
                if let Some(p) = current.take() {
                    paras.push(p);
                }
                let para_shape_id = if rec.data.len() >= 6 {
                    u16::from_le_bytes([rec.data[4], rec.data[5]])
                } else {
                    0
                };
                current = Some(HwpParagraph {
                    text: String::new(),
                    char_shape_ids: Vec::new(),
                    para_shape_id,
                    controls: Vec::new(),
                });
                idx += 1;
            }
            HWPTAG_PARA_TEXT => {
                if let Some(ref mut p) = current {
                    p.text = extract_paragraph_text(&rec.data);
                }
                idx += 1;
            }
            HWPTAG_PARA_CHAR_SHAPE => {
                if let Some(ref mut p) = current {
                    p.char_shape_ids = parse_char_shape_refs(&rec.data);
                }
                idx += 1;
            }
            HWPTAG_EQEDIT => {
                if let Some(ref mut p) = current {
                    let (script, _) = read_utf16le_str(&rec.data, 2);
                    if !script.is_empty() {
                        let tex = eqedit_to_latex(&script);
                        p.controls.push(HwpControl::Equation { script: tex });
                    }
                }
                idx += 1;
            }
            HWPTAG_CTRL_HEADER => {
                // Nested controls inside cells (e.g. images within a table cell).
                // Parse and attach to current paragraph, then skip the subtree.
                if let Some(ctrl) = parse_ctrl_header_at(records, idx) {
                    if let Some(ref mut p) = current {
                        p.controls.push(ctrl);
                    }
                }
                idx = find_children_end(records, idx);
            }
            _ => {
                idx += 1;
            }
        }
    }

    if let Some(p) = current {
        paras.push(p);
    }
    paras
}

/// Parse a `CTRL_TABLE` subtree starting at `ctrl_idx` in `records`.
///
/// Returns `(row_count, col_count, cells)`.
pub(crate) fn parse_table_ctrl(
    records: &[Record],
    ctrl_idx: usize,
) -> (u16, u16, Vec<HwpTableCell>) {
    let ctrl_end = find_children_end(records, ctrl_idx);
    let mut row_count: u16 = 0;
    let mut col_count: u16 = 0;
    let mut cells: Vec<HwpTableCell> = Vec::new();

    let mut idx = ctrl_idx + 1;
    while idx < ctrl_end {
        let rec = &records[idx];
        match rec.tag_id {
            HWPTAG_TABLE => {
                // TABLE record layout (minimum 8 bytes):
                //   bytes 0-3: properties (u32)
                //   bytes 4-5: row count (u16)
                //   bytes 6-7: col count (u16)
                if rec.data.len() >= 6 {
                    row_count = u16::from_le_bytes([rec.data[4], rec.data[5]]);
                }
                if rec.data.len() >= 8 {
                    col_count = u16::from_le_bytes([rec.data[6], rec.data[7]]);
                }
                tracing::debug!("TABLE dims: {row_count}×{col_count}");
                idx += 1;
            }
            HWPTAG_LIST_HEADER => {
                // LIST_HEADER record for a single table cell.
                // Layout (minimum 10 bytes):
                //   bytes 0-1: properties (u16)
                //   bytes 2-3: col (u16)   ← address within the row
                //   bytes 4-5: row (u16)   ← row address
                //   bytes 6-7: col_span (u16)
                //   bytes 8-9: row_span (u16)
                let col = if rec.data.len() >= 4 {
                    u16::from_le_bytes([rec.data[2], rec.data[3]])
                } else {
                    0
                };
                let row = if rec.data.len() >= 6 {
                    u16::from_le_bytes([rec.data[4], rec.data[5]])
                } else {
                    0
                };
                let col_span = if rec.data.len() >= 8 {
                    let v = u16::from_le_bytes([rec.data[6], rec.data[7]]);
                    if v == 0 { 1 } else { v }
                } else {
                    1
                };
                let row_span = if rec.data.len() >= 10 {
                    let v = u16::from_le_bytes([rec.data[8], rec.data[9]]);
                    if v == 0 { 1 } else { v }
                } else {
                    1
                };

                let cell_end = find_children_end(records, idx);
                let paragraphs = extract_paragraphs_from_range(records, idx + 1, cell_end);

                cells.push(HwpTableCell {
                    row,
                    col,
                    row_span,
                    col_span,
                    paragraphs,
                });
                idx = cell_end;
            }
            _ => {
                idx += 1;
            }
        }
    }

    (row_count, col_count, cells)
}

/// Parse the GShapeObject CTRL_HEADER subtree starting at `ctrl_idx`.
///
/// Returns `(bin_data_id, width, height)`.  All values are 0 when unavailable.
///
/// The CTRL_HEADER data for `gso ` layout:
///   bytes  0- 3: ctrl_id (already validated by caller)
///   bytes  4- 7: ctrl header properties (u32)
///   bytes  8-11: y offset (i32)  — ignored
///   bytes 12-15: x offset (i32)  — ignored
///   bytes 16-19: width (hwp unit, 1/7200 inch)
///   bytes 20-23: height (hwp unit)
///
/// The BinData reference lives in a child `HWPTAG_GSOTYPE` record.
/// GSOTYPE layout for a picture (GSOType == 0):
///   bytes  0- 3: GSOType kind (u32) — 0 = picture
///   bytes  4- 7: color fill (u32)
///   ...varies by kind...
/// For pictures (kind 0), the bin data ID is at offset 80 (u16) in the GSOTYPE body.
/// In practice this offset varies; we probe for it defensively.
pub(crate) fn parse_gshape_ctrl(records: &[Record], ctrl_idx: usize) -> (u16, u32, u32) {
    let rec = &records[ctrl_idx];

    // Extract width and height from the CTRL_HEADER data itself.
    let width = if rec.data.len() >= 20 {
        u32::from_le_bytes([rec.data[16], rec.data[17], rec.data[18], rec.data[19]])
    } else {
        0
    };
    let height = if rec.data.len() >= 24 {
        u32::from_le_bytes([rec.data[20], rec.data[21], rec.data[22], rec.data[23]])
    } else {
        0
    };

    // Search child records for HWPTAG_GSOTYPE which carries the bin data reference.
    let ctrl_end = find_children_end(records, ctrl_idx);
    let bin_data_id = find_gsotype_bin_id(records, ctrl_idx + 1, ctrl_end);

    tracing::debug!("GSHAPE: bin_id={bin_data_id} width={width} height={height}");
    (bin_data_id, width, height)
}

/// Scan `records[start..end]` for a `HWPTAG_GSOTYPE` record and extract the
/// BinData ID.  The ID is a `u16` embedded at a known offset in the record data.
///
/// HWP 5.0 GSOTYPE (picture kind = 0) body layout relevant fields:
///   bytes  0- 3: GSOType kind (u32) — 0 = picture, 1 = OLE, ...
/// For kind 0 (picture), the BinData index follows at the end of a fixed-size
/// header.  In practice the ID is stored at offset 2 inside a nested
/// `HWPTAG_BEGIN + 68` (GSOPicture) record.  We fall back to scanning for
/// a plausible non-zero u16 at known candidate offsets.
pub(crate) fn find_gsotype_bin_id(records: &[Record], start: usize, end: usize) -> u16 {
    for rec in records.iter().skip(start).take(end.saturating_sub(start)) {
        if rec.tag_id == HWPTAG_GSOTYPE {
            // GSOTYPE record for a picture:
            //   bytes 0-3: kind (0 = picture)
            //   bytes 4-7: fill color (u32)
            //   ...
            // The embedded BinData ID for pictures is at byte offset 2 of a
            // child "picSub" structure.  Empirically it sits at offset 0 of
            // a sub-record (tag HWPTAG_BEGIN+68), but we also check at offset
            // 2 and 4 within this record itself when there are no children.
            if rec.data.len() >= 4 {
                let kind = u32::from_le_bytes([
                    rec.data[0],
                    rec.data[1],
                    rec.data[2],
                    rec.data[3],
                ]);
                if kind == 0 && rec.data.len() >= 6 {
                    // Candidate at offset 4 (u16).
                    let candidate = u16::from_le_bytes([rec.data[4], rec.data[5]]);
                    if candidate > 0 {
                        return candidate;
                    }
                }
            }
        }
    }
    0
}

/// Parse the `CTRL_HEADER` at `ctrl_idx` and return the corresponding
/// `HwpControl` variant, or `None` if the control type is unknown/malformed.
/// Parse the URL from a `hyln` (CTRL_HYPERLINK) CTRL_HEADER record.
///
/// Layout (observed): bytes 0-3 = ctrl_id (`hyln`), bytes 4-5 = UTF-16LE
/// char count, followed by that many UTF-16LE code units.  The exact
/// field layout may vary across HWP versions; we apply a plausibility
/// check on the decoded URL and return empty on garbage.
pub(crate) fn parse_hyperlink_url(rec: &Record) -> String {
    if rec.data.len() < 6 {
        return String::new();
    }
    let (url, _) = read_utf16le_str(&rec.data, 4);
    if url.is_empty() || url.contains('\0') {
        return String::new();
    }
    url
}

pub(crate) fn parse_ctrl_header_at(records: &[Record], ctrl_idx: usize) -> Option<HwpControl> {
    let rec = &records[ctrl_idx];
    if rec.data.len() < 4 {
        tracing::debug!(
            "CTRL_HEADER at index {ctrl_idx}: data too short ({} bytes)",
            rec.data.len()
        );
        return None;
    }

    let ctrl_id = u32::from_le_bytes([rec.data[0], rec.data[1], rec.data[2], rec.data[3]]);

    match ctrl_id {
        CTRL_TABLE => {
            let (row_count, col_count, cells) = parse_table_ctrl(records, ctrl_idx);
            tracing::debug!(
                "Parsed table: {row_count}×{col_count}, {} cells",
                cells.len()
            );
            Some(HwpControl::Table {
                row_count,
                col_count,
                cells,
            })
        }
        CTRL_GSHAPE => {
            let (bin_data_id, width, height) = parse_gshape_ctrl(records, ctrl_idx);
            Some(HwpControl::Image {
                bin_data_id,
                width,
                height,
            })
        }
        CTRL_FOOTNOTE => {
            let ctrl_end = find_children_end(records, ctrl_idx);
            let paragraphs = extract_paragraphs_from_range(records, ctrl_idx + 1, ctrl_end);
            Some(HwpControl::FootnoteEndnote {
                is_endnote: false,
                paragraphs,
            })
        }
        CTRL_ENDNOTE => {
            let ctrl_end = find_children_end(records, ctrl_idx);
            let paragraphs = extract_paragraphs_from_range(records, ctrl_idx + 1, ctrl_end);
            Some(HwpControl::FootnoteEndnote {
                is_endnote: true,
                paragraphs,
            })
        }
        CTRL_HYPERLINK => {
            let url = parse_hyperlink_url(rec);
            if url.is_empty() {
                None
            } else {
                Some(HwpControl::Hyperlink { url })
            }
        }
        CTRL_PAGE_BREAK => Some(HwpControl::PageBreak),
        CTRL_COL_BREAK => Some(HwpControl::ColumnBreak),
        _ => {
            tracing::debug!(
                "CTRL_HEADER at index {ctrl_idx}: unhandled ctrl_id=0x{ctrl_id:08X}"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hwp::reader::encode_u16s_test;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Build a minimal Record for testing purposes.
    fn make_record(tag_id: u16, level: u16) -> Record {
        Record {
            tag_id,
            level,
            data: Vec::new(),
        }
    }

    /// Build a Record with a specific data payload.
    fn make_record_with_data(tag_id: u16, level: u16, data: Vec<u8>) -> Record {
        Record {
            tag_id,
            level,
            data,
        }
    }

    /// Build a TABLE record with given row_count and col_count.
    fn make_table_record(level: u16, row_count: u16, col_count: u16) -> Record {
        let mut data = vec![0u8; 8];
        // bytes 0-3: properties (zeroed)
        data[4..6].copy_from_slice(&row_count.to_le_bytes());
        data[6..8].copy_from_slice(&col_count.to_le_bytes());
        make_record_with_data(HWPTAG_TABLE, level, data)
    }

    /// Build a LIST_HEADER record describing one table cell at (row, col).
    fn make_list_header_record(
        level: u16,
        col: u16,
        row: u16,
        col_span: u16,
        row_span: u16,
    ) -> Record {
        let mut data = vec![0u8; 10];
        // bytes 0-1: properties
        data[2..4].copy_from_slice(&col.to_le_bytes());
        data[4..6].copy_from_slice(&row.to_le_bytes());
        data[6..8].copy_from_slice(&col_span.to_le_bytes());
        data[8..10].copy_from_slice(&row_span.to_le_bytes());
        make_record_with_data(HWPTAG_LIST_HEADER, level, data)
    }

    /// Build a CTRL_HEADER record with ctrl_id `tbl `.
    fn make_ctrl_header_table(level: u16) -> Record {
        make_record_with_data(HWPTAG_CTRL_HEADER, level, CTRL_TABLE.to_le_bytes().to_vec())
    }

    // -----------------------------------------------------------------------
    // find_children_end
    // -----------------------------------------------------------------------

    #[test]
    fn find_children_end_no_children() {
        // Parent at level 0, immediately followed by a sibling at the same level.
        let records = vec![
            make_record(HWPTAG_CTRL_HEADER, 0),
            make_record(HWPTAG_PARA_HEADER, 0), // sibling, not child
        ];
        assert_eq!(find_children_end(&records, 0), 1);
    }

    #[test]
    fn find_children_end_with_children() {
        // Parent at level 1, two children at level 2, then a sibling at level 1.
        let records = vec![
            make_record(HWPTAG_CTRL_HEADER, 1), // index 0 (parent)
            make_record(HWPTAG_TABLE, 2),        // index 1 (child)
            make_record(HWPTAG_LIST_HEADER, 2),  // index 2 (child)
            make_record(HWPTAG_PARA_HEADER, 1),  // index 3 (sibling — stops here)
        ];
        assert_eq!(find_children_end(&records, 0), 3);
    }

    #[test]
    fn find_children_end_deeply_nested() {
        // Parent at 0, child at 1, grandchild at 2 — all are "descendants" of 0.
        let records = vec![
            make_record(HWPTAG_CTRL_HEADER, 0),  // index 0
            make_record(HWPTAG_TABLE, 1),         // index 1 (child)
            make_record(HWPTAG_LIST_HEADER, 2),   // index 2 (grandchild)
            make_record(HWPTAG_PARA_HEADER, 3),   // index 3 (great-grandchild)
            make_record(HWPTAG_PARA_HEADER, 0),   // index 4 (sibling)
        ];
        assert_eq!(find_children_end(&records, 0), 4);
    }

    #[test]
    fn find_children_end_at_last_record() {
        // Parent is the last element — no children, end == len.
        let records = vec![make_record(HWPTAG_CTRL_HEADER, 0)];
        assert_eq!(find_children_end(&records, 0), 1);
    }

    // -----------------------------------------------------------------------
    // parse_table_ctrl
    // -----------------------------------------------------------------------

    #[test]
    fn parse_table_ctrl_dimensions() {
        // Flat record sequence:
        //   [0] CTRL_HEADER(tbl )  level=0
        //   [1] TABLE              level=1  (2 rows × 3 cols)
        let records = vec![make_ctrl_header_table(0), make_table_record(1, 2, 3)];
        let (rows, cols, cells) = parse_table_ctrl(&records, 0);
        assert_eq!(rows, 2);
        assert_eq!(cols, 3);
        assert!(cells.is_empty(), "no LIST_HEADERs so no cells expected");
    }

    #[test]
    fn parse_table_ctrl_with_cells() {
        // One 1×2 table (1 row, 2 cols) with two cells each containing one paragraph.
        //
        // Record sequence (level notation: CH=0, TABLE/LH=1, PH=2, PT=3):
        //   [0] CTRL_HEADER(tbl)   level=0
        //   [1] TABLE(1×2)         level=1
        //   [2] LIST_HEADER(r=0,c=0, span 1×1) level=1
        //   [3] PARA_HEADER        level=2
        //   [4] PARA_TEXT("A")     level=3   ← inside cell (0,0)
        //   [5] LIST_HEADER(r=0,c=1, span 1×1) level=1
        //   [6] PARA_HEADER        level=2
        //   [7] PARA_TEXT("B")     level=3   ← inside cell (0,1)
        let text_a = encode_u16s_test(&[b'A' as u16]);
        let text_b = encode_u16s_test(&[b'B' as u16]);

        let mut para_header_data = vec![0u8; 6]; // 6 bytes minimum
        para_header_data[4] = 0; // para_shape_id = 0
        para_header_data[5] = 0;

        let records = vec![
            make_ctrl_header_table(0),               // [0]
            make_table_record(1, 1, 2),              // [1]
            make_list_header_record(1, 0, 0, 1, 1),  // [2]
            make_record_with_data(HWPTAG_PARA_HEADER, 2, para_header_data.clone()), // [3]
            make_record_with_data(HWPTAG_PARA_TEXT, 3, text_a), // [4]
            make_list_header_record(1, 1, 0, 1, 1),  // [5]
            make_record_with_data(HWPTAG_PARA_HEADER, 2, para_header_data.clone()), // [6]
            make_record_with_data(HWPTAG_PARA_TEXT, 3, text_b), // [7]
        ];

        let (rows, cols, cells) = parse_table_ctrl(&records, 0);
        assert_eq!(rows, 1);
        assert_eq!(cols, 2);
        assert_eq!(cells.len(), 2);

        // Cell (0,0) should have text "A"
        let cell_00 = cells
            .iter()
            .find(|c| c.row == 0 && c.col == 0)
            .expect("cell (0,0)");
        assert_eq!(cell_00.paragraphs.len(), 1);
        assert_eq!(cell_00.paragraphs[0].text, "A");

        // Cell (0,1) should have text "B"
        let cell_01 = cells
            .iter()
            .find(|c| c.row == 0 && c.col == 1)
            .expect("cell (0,1)");
        assert_eq!(cell_01.paragraphs.len(), 1);
        assert_eq!(cell_01.paragraphs[0].text, "B");
    }

    #[test]
    fn parse_table_ctrl_cell_spans() {
        // A cell with row_span=2, col_span=2 must be recorded faithfully.
        let records = vec![
            make_ctrl_header_table(0),
            make_table_record(1, 2, 2),
            make_list_header_record(1, 0, 0, 2, 2), // merged cell spanning 2×2
        ];
        let (_rows, _cols, cells) = parse_table_ctrl(&records, 0);
        assert_eq!(cells.len(), 1);
        assert_eq!(cells[0].col_span, 2);
        assert_eq!(cells[0].row_span, 2);
    }

    #[test]
    fn parse_table_ctrl_malformed_table_record_short_data() {
        // TABLE record with only 4 bytes — row/col count cannot be read.
        // Must not panic; dimensions default to 0.
        let mut short_data = vec![0u8; 4];
        short_data[0..4].copy_from_slice(&0u32.to_le_bytes()); // properties only
        let records = vec![
            make_ctrl_header_table(0),
            make_record_with_data(HWPTAG_TABLE, 1, short_data),
        ];
        let (rows, cols, cells) = parse_table_ctrl(&records, 0);
        assert_eq!(rows, 0);
        assert_eq!(cols, 0);
        assert!(cells.is_empty());
    }

    // -----------------------------------------------------------------------
    // parse_ctrl_header_at
    // -----------------------------------------------------------------------

    #[test]
    fn parse_ctrl_header_at_table_returns_table_control() {
        let records = vec![make_ctrl_header_table(0), make_table_record(1, 3, 4)];
        let ctrl = parse_ctrl_header_at(&records, 0).expect("should return Some");
        assert!(
            matches!(
                ctrl,
                HwpControl::Table {
                    row_count: 3,
                    col_count: 4,
                    ..
                }
            ),
            "expected Table{{row_count:3, col_count:4}}, got {ctrl:?}"
        );
    }

    #[test]
    fn parse_ctrl_header_at_short_data_returns_none() {
        // CTRL_HEADER with fewer than 4 bytes of data → cannot read ctrl_id.
        let records = vec![make_record_with_data(HWPTAG_CTRL_HEADER, 0, vec![0u8; 2])];
        assert!(parse_ctrl_header_at(&records, 0).is_none());
    }

    #[test]
    fn parse_ctrl_header_at_unknown_ctrl_id_returns_none() {
        // An unrecognised ctrl_id must return None gracefully.
        let unknown_id: u32 = 0xDEAD_BEEF;
        let records = vec![make_record_with_data(
            HWPTAG_CTRL_HEADER,
            0,
            unknown_id.to_le_bytes().to_vec(),
        )];
        assert!(parse_ctrl_header_at(&records, 0).is_none());
    }

    #[test]
    fn parse_ctrl_header_at_footnote_returns_footnote_endnote() {
        let fn_id = CTRL_FOOTNOTE.to_le_bytes().to_vec();
        let records = vec![make_record_with_data(HWPTAG_CTRL_HEADER, 0, fn_id)];
        let ctrl = parse_ctrl_header_at(&records, 0).expect("Some");
        assert!(matches!(
            ctrl,
            HwpControl::FootnoteEndnote {
                is_endnote: false,
                ..
            }
        ));
    }

    #[test]
    fn parse_ctrl_header_at_endnote_returns_footnote_endnote() {
        let en_id = CTRL_ENDNOTE.to_le_bytes().to_vec();
        let records = vec![make_record_with_data(HWPTAG_CTRL_HEADER, 0, en_id)];
        let ctrl = parse_ctrl_header_at(&records, 0).expect("Some");
        assert!(matches!(
            ctrl,
            HwpControl::FootnoteEndnote {
                is_endnote: true,
                ..
            }
        ));
    }

    #[test]
    fn parse_ctrl_header_at_page_break_returns_page_break() {
        let pb_id = CTRL_PAGE_BREAK.to_le_bytes().to_vec();
        let records = vec![make_record_with_data(HWPTAG_CTRL_HEADER, 0, pb_id)];
        let ctrl = parse_ctrl_header_at(&records, 0).expect("Some");
        assert!(matches!(ctrl, HwpControl::PageBreak));
    }

    // -----------------------------------------------------------------------
    // extract_paragraphs_from_range
    // -----------------------------------------------------------------------

    #[test]
    fn extract_paragraphs_from_range_empty_range() {
        let records: Vec<Record> = Vec::new();
        let paras = extract_paragraphs_from_range(&records, 0, 0);
        assert!(paras.is_empty());
    }

    #[test]
    fn extract_paragraphs_from_range_single_paragraph() {
        let text_data = encode_u16s_test(&[b'H' as u16, b'i' as u16]);
        let mut ph_data = vec![0u8; 6];
        ph_data[4] = 0;
        ph_data[5] = 0;

        let records = vec![
            make_record_with_data(HWPTAG_PARA_HEADER, 2, ph_data),
            make_record_with_data(HWPTAG_PARA_TEXT, 3, text_data),
        ];
        let paras = extract_paragraphs_from_range(&records, 0, records.len());
        assert_eq!(paras.len(), 1);
        assert_eq!(paras[0].text, "Hi");
    }

    #[test]
    fn extract_paragraphs_from_range_multiple_paragraphs() {
        let text_a = encode_u16s_test(&[b'A' as u16]);
        let text_b = encode_u16s_test(&[b'B' as u16]);
        let ph_data = vec![0u8; 6];

        let records = vec![
            make_record_with_data(HWPTAG_PARA_HEADER, 2, ph_data.clone()),
            make_record_with_data(HWPTAG_PARA_TEXT, 3, text_a),
            make_record_with_data(HWPTAG_PARA_HEADER, 2, ph_data.clone()),
            make_record_with_data(HWPTAG_PARA_TEXT, 3, text_b),
        ];
        let paras = extract_paragraphs_from_range(&records, 0, records.len());
        assert_eq!(paras.len(), 2);
        assert_eq!(paras[0].text, "A");
        assert_eq!(paras[1].text, "B");
    }

    // --- find_gsotype_bin_id / parse_gshape_ctrl tests ---

    #[test]
    fn find_gsotype_bin_id_returns_id_from_picture_record() {
        let mut data = vec![0u8; 6];
        data[0..4].copy_from_slice(&0u32.to_le_bytes()); // kind = 0 (picture)
        data[4..6].copy_from_slice(&42u16.to_le_bytes()); // bin_data_id = 42
        let records = vec![Record {
            tag_id: HWPTAG_GSOTYPE,
            level: 1,
            data,
        }];
        assert_eq!(find_gsotype_bin_id(&records, 0, 1), 42);
    }

    #[test]
    fn find_gsotype_bin_id_returns_zero_when_no_gsotype() {
        let records = vec![Record {
            tag_id: HWPTAG_PARA_HEADER,
            level: 0,
            data: vec![0u8; 8],
        }];
        assert_eq!(find_gsotype_bin_id(&records, 0, 1), 0);
    }

    #[test]
    fn find_gsotype_bin_id_skips_non_picture_kind() {
        let mut data = vec![0u8; 6];
        data[0..4].copy_from_slice(&1u32.to_le_bytes()); // kind = 1 (OLE, not picture)
        data[4..6].copy_from_slice(&10u16.to_le_bytes());
        let records = vec![Record {
            tag_id: HWPTAG_GSOTYPE,
            level: 1,
            data,
        }];
        assert_eq!(find_gsotype_bin_id(&records, 0, 1), 0);
    }

    #[test]
    fn parse_gshape_ctrl_extracts_dimensions_and_bin_id() {
        let mut ctrl_data = vec![0u8; 24];
        ctrl_data[0..4].copy_from_slice(&CTRL_GSHAPE.to_le_bytes());
        ctrl_data[16..20].copy_from_slice(&800u32.to_le_bytes()); // width
        ctrl_data[20..24].copy_from_slice(&600u32.to_le_bytes()); // height

        let mut gsotype_data = vec![0u8; 6];
        gsotype_data[0..4].copy_from_slice(&0u32.to_le_bytes()); // picture
        gsotype_data[4..6].copy_from_slice(&7u16.to_le_bytes()); // bin_data_id

        let records = vec![
            Record {
                tag_id: HWPTAG_CTRL_HEADER,
                level: 0,
                data: ctrl_data,
            },
            Record {
                tag_id: HWPTAG_GSOTYPE,
                level: 1,
                data: gsotype_data,
            },
        ];
        let (bin_id, w, h) = parse_gshape_ctrl(&records, 0);
        assert_eq!(bin_id, 7);
        assert_eq!(w, 800);
        assert_eq!(h, 600);
    }

    // --- hyperlink tests ---

    #[test]
    fn parse_hyperlink_url_valid() {
        let url_chars: Vec<u16> = "https://example.com".encode_utf16().collect();
        let mut data = CTRL_HYPERLINK.to_le_bytes().to_vec();
        data.extend_from_slice(&(url_chars.len() as u16).to_le_bytes());
        for ch in &url_chars {
            data.extend_from_slice(&ch.to_le_bytes());
        }
        let rec = Record {
            tag_id: HWPTAG_CTRL_HEADER,
            level: 0,
            data,
        };
        assert_eq!(parse_hyperlink_url(&rec), "https://example.com");
    }

    #[test]
    fn parse_hyperlink_url_truncated_returns_empty() {
        let data = CTRL_HYPERLINK.to_le_bytes().to_vec();
        let rec = Record {
            tag_id: HWPTAG_CTRL_HEADER,
            level: 0,
            data,
        };
        assert_eq!(parse_hyperlink_url(&rec), "");
    }

    #[test]
    fn parse_ctrl_header_at_hyperlink_returns_control() {
        let url_chars: Vec<u16> = "https://test.org".encode_utf16().collect();
        let mut data = CTRL_HYPERLINK.to_le_bytes().to_vec();
        data.extend_from_slice(&(url_chars.len() as u16).to_le_bytes());
        for ch in &url_chars {
            data.extend_from_slice(&ch.to_le_bytes());
        }
        let records = vec![Record {
            tag_id: HWPTAG_CTRL_HEADER,
            level: 0,
            data,
        }];
        let ctrl = parse_ctrl_header_at(&records, 0).expect("Some");
        assert!(matches!(ctrl, HwpControl::Hyperlink { url } if url == "https://test.org"));
    }

    #[test]
    fn parse_ctrl_header_at_hyperlink_empty_url_returns_none() {
        let mut data = CTRL_HYPERLINK.to_le_bytes().to_vec();
        data.extend_from_slice(&0u16.to_le_bytes()); // zero-length URL
        let records = vec![Record {
            tag_id: HWPTAG_CTRL_HEADER,
            level: 0,
            data,
        }];
        assert!(parse_ctrl_header_at(&records, 0).is_none());
    }
}
