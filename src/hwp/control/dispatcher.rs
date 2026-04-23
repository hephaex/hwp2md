use crate::hwp::model::*;
use crate::hwp::record::*;
use super::common::{extract_paragraphs_from_range, find_children_end};
use super::hyperlink::parse_hyperlink_url;
use super::image::parse_gshape_ctrl;
use super::table::parse_table_ctrl;

/// Parse the `CTRL_HEADER` at `ctrl_idx` and return the corresponding
/// `HwpControl` variant, or `None` if the control type is unknown/malformed.
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
            tracing::debug!("CTRL_HEADER at index {ctrl_idx}: unhandled ctrl_id=0x{ctrl_id:08X}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hwp::record::{
        CTRL_ENDNOTE, CTRL_FOOTNOTE, CTRL_HYPERLINK, CTRL_PAGE_BREAK, CTRL_TABLE, HWPTAG_CTRL_HEADER,
        HWPTAG_TABLE,
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

    fn make_ctrl_header_table(level: u16) -> Record {
        make_record_with_data(HWPTAG_CTRL_HEADER, level, CTRL_TABLE.to_le_bytes().to_vec())
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
        let records = vec![make_record_with_data(HWPTAG_CTRL_HEADER, 0, vec![0u8; 2])];
        assert!(parse_ctrl_header_at(&records, 0).is_none());
    }

    #[test]
    fn parse_ctrl_header_at_unknown_ctrl_id_returns_none() {
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
        data.extend_from_slice(&0u16.to_le_bytes());
        let records = vec![Record {
            tag_id: HWPTAG_CTRL_HEADER,
            level: 0,
            data,
        }];
        assert!(parse_ctrl_header_at(&records, 0).is_none());
    }
}
