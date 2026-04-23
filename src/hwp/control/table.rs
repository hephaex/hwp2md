use crate::hwp::model::*;
use crate::hwp::record::*;
use super::common::{extract_paragraphs_from_range, find_children_end};

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

                // Vertical alignment is stored at byte 26 of the LIST_HEADER data.
                // Values: 0 = top, 1 = center, 2 = bottom.
                let vertical_align = if rec.data.len() >= 27 { rec.data[26] } else { 0 };

                // Cells in row 0 are considered header cells.
                let is_header = row == 0;

                let cell_end = find_children_end(records, idx);
                let paragraphs = extract_paragraphs_from_range(records, idx + 1, cell_end);

                cells.push(HwpTableCell {
                    row,
                    col,
                    row_span,
                    col_span,
                    vertical_align,
                    is_header,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hwp::reader::encode_u16s_test;
    use crate::hwp::record::{
        HWPTAG_CTRL_HEADER, HWPTAG_LIST_HEADER, HWPTAG_PARA_HEADER, HWPTAG_PARA_TEXT, HWPTAG_TABLE,
    };

    fn make_record_with_data(tag_id: u16, level: u16, data: Vec<u8>) -> Record {
        Record {
            tag_id,
            level,
            data,
        }
    }

    fn make_table_record(level: u16, row_count: u16, col_count: u16) -> Record {
        let mut data = vec![0u8; 8];
        data[4..6].copy_from_slice(&row_count.to_le_bytes());
        data[6..8].copy_from_slice(&col_count.to_le_bytes());
        make_record_with_data(HWPTAG_TABLE, level, data)
    }

    fn make_list_header_record(
        level: u16,
        col: u16,
        row: u16,
        col_span: u16,
        row_span: u16,
    ) -> Record {
        let mut data = vec![0u8; 10];
        data[2..4].copy_from_slice(&col.to_le_bytes());
        data[4..6].copy_from_slice(&row.to_le_bytes());
        data[6..8].copy_from_slice(&col_span.to_le_bytes());
        data[8..10].copy_from_slice(&row_span.to_le_bytes());
        make_record_with_data(HWPTAG_LIST_HEADER, level, data)
    }

    fn make_ctrl_header_table(level: u16) -> Record {
        make_record_with_data(HWPTAG_CTRL_HEADER, level, CTRL_TABLE.to_le_bytes().to_vec())
    }

    fn make_list_header_with_valign(
        level: u16,
        col: u16,
        row: u16,
        col_span: u16,
        row_span: u16,
        vertical_align: u8,
    ) -> Record {
        let mut data = vec![0u8; 27];
        data[2..4].copy_from_slice(&col.to_le_bytes());
        data[4..6].copy_from_slice(&row.to_le_bytes());
        data[6..8].copy_from_slice(&col_span.to_le_bytes());
        data[8..10].copy_from_slice(&row_span.to_le_bytes());
        data[26] = vertical_align;
        make_record_with_data(HWPTAG_LIST_HEADER, level, data)
    }

    // -----------------------------------------------------------------------
    // parse_table_ctrl
    // -----------------------------------------------------------------------

    #[test]
    fn parse_table_ctrl_dimensions() {
        let records = vec![make_ctrl_header_table(0), make_table_record(1, 2, 3)];
        let (rows, cols, cells) = parse_table_ctrl(&records, 0);
        assert_eq!(rows, 2);
        assert_eq!(cols, 3);
        assert!(cells.is_empty(), "no LIST_HEADERs so no cells expected");
    }

    #[test]
    fn parse_table_ctrl_with_cells() {
        let text_a = encode_u16s_test(&[b'A' as u16]);
        let text_b = encode_u16s_test(&[b'B' as u16]);

        let mut para_header_data = vec![0u8; 6];
        para_header_data[4] = 0;
        para_header_data[5] = 0;

        let records = vec![
            make_ctrl_header_table(0),
            make_table_record(1, 1, 2),
            make_list_header_record(1, 0, 0, 1, 1),
            make_record_with_data(HWPTAG_PARA_HEADER, 2, para_header_data.clone()),
            make_record_with_data(HWPTAG_PARA_TEXT, 3, text_a),
            make_list_header_record(1, 1, 0, 1, 1),
            make_record_with_data(HWPTAG_PARA_HEADER, 2, para_header_data.clone()),
            make_record_with_data(HWPTAG_PARA_TEXT, 3, text_b),
        ];

        let (rows, cols, cells) = parse_table_ctrl(&records, 0);
        assert_eq!(rows, 1);
        assert_eq!(cols, 2);
        assert_eq!(cells.len(), 2);

        let cell_00 = cells
            .iter()
            .find(|c| c.row == 0 && c.col == 0)
            .expect("cell (0,0)");
        assert_eq!(cell_00.paragraphs.len(), 1);
        assert_eq!(cell_00.paragraphs[0].text, "A");

        let cell_01 = cells
            .iter()
            .find(|c| c.row == 0 && c.col == 1)
            .expect("cell (0,1)");
        assert_eq!(cell_01.paragraphs.len(), 1);
        assert_eq!(cell_01.paragraphs[0].text, "B");
    }

    #[test]
    fn parse_table_ctrl_cell_spans() {
        let records = vec![
            make_ctrl_header_table(0),
            make_table_record(1, 2, 2),
            make_list_header_record(1, 0, 0, 2, 2),
        ];
        let (_rows, _cols, cells) = parse_table_ctrl(&records, 0);
        assert_eq!(cells.len(), 1);
        assert_eq!(cells[0].col_span, 2);
        assert_eq!(cells[0].row_span, 2);
    }

    #[test]
    fn parse_table_ctrl_malformed_table_record_short_data() {
        let mut short_data = vec![0u8; 4];
        short_data[0..4].copy_from_slice(&0u32.to_le_bytes());
        let records = vec![
            make_ctrl_header_table(0),
            make_record_with_data(HWPTAG_TABLE, 1, short_data),
        ];
        let (rows, cols, cells) = parse_table_ctrl(&records, 0);
        assert_eq!(rows, 0);
        assert_eq!(cols, 0);
        assert!(cells.is_empty());
    }

    #[test]
    fn parse_table_ctrl_cell_row0_is_header_true() {
        let records = vec![
            make_ctrl_header_table(0),
            make_table_record(1, 1, 1),
            make_list_header_record(1, 0, 0, 1, 1),
        ];
        let (_rows, _cols, cells) = parse_table_ctrl(&records, 0);
        assert_eq!(cells.len(), 1);
        assert!(cells[0].is_header, "row 0 cell must be is_header");
    }

    #[test]
    fn parse_table_ctrl_cell_row1_is_header_false() {
        let records = vec![
            make_ctrl_header_table(0),
            make_table_record(1, 2, 1),
            make_list_header_record(1, 0, 1, 1, 1),
        ];
        let (_rows, _cols, cells) = parse_table_ctrl(&records, 0);
        assert_eq!(cells.len(), 1);
        assert!(!cells[0].is_header, "row 1 cell must not be is_header");
    }

    #[test]
    fn parse_table_ctrl_vertical_align_center_parsed() {
        let records = vec![
            make_ctrl_header_table(0),
            make_table_record(1, 1, 1),
            make_list_header_with_valign(1, 0, 0, 1, 1, 1),
        ];
        let (_rows, _cols, cells) = parse_table_ctrl(&records, 0);
        assert_eq!(cells.len(), 1);
        assert_eq!(cells[0].vertical_align, 1);
    }

    #[test]
    fn parse_table_ctrl_vertical_align_bottom_parsed() {
        let records = vec![
            make_ctrl_header_table(0),
            make_table_record(1, 1, 1),
            make_list_header_with_valign(1, 0, 0, 1, 1, 2),
        ];
        let (_rows, _cols, cells) = parse_table_ctrl(&records, 0);
        assert_eq!(cells.len(), 1);
        assert_eq!(cells[0].vertical_align, 2);
    }

    #[test]
    fn parse_table_ctrl_vertical_align_defaults_to_top_when_short_data() {
        let records = vec![
            make_ctrl_header_table(0),
            make_table_record(1, 1, 1),
            make_list_header_record(1, 0, 0, 1, 1),
        ];
        let (_rows, _cols, cells) = parse_table_ctrl(&records, 0);
        assert_eq!(cells.len(), 1);
        assert_eq!(cells[0].vertical_align, 0);
    }
}
