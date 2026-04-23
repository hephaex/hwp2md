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
    use super::dispatcher::parse_ctrl_header_at;

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
                        p.controls.push(HwpControl::Equation { script });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hwp::reader::encode_u16s_test;
    use crate::hwp::record::{HWPTAG_PARA_HEADER, HWPTAG_PARA_TEXT};

    fn make_record(tag_id: u16, level: u16) -> Record {
        Record {
            tag_id,
            level,
            data: Vec::new(),
        }
    }

    fn make_record_with_data(tag_id: u16, level: u16, data: Vec<u8>) -> Record {
        Record {
            tag_id,
            level,
            data,
        }
    }

    // -----------------------------------------------------------------------
    // find_children_end
    // -----------------------------------------------------------------------

    #[test]
    fn find_children_end_no_children() {
        let records = vec![
            make_record(HWPTAG_CTRL_HEADER, 0),
            make_record(HWPTAG_PARA_HEADER, 0),
        ];
        assert_eq!(find_children_end(&records, 0), 1);
    }

    #[test]
    fn find_children_end_with_children() {
        let records = vec![
            make_record(HWPTAG_CTRL_HEADER, 1),
            make_record(HWPTAG_TABLE, 2),
            make_record(HWPTAG_LIST_HEADER, 2),
            make_record(HWPTAG_PARA_HEADER, 1),
        ];
        assert_eq!(find_children_end(&records, 0), 3);
    }

    #[test]
    fn find_children_end_deeply_nested() {
        let records = vec![
            make_record(HWPTAG_CTRL_HEADER, 0),
            make_record(HWPTAG_TABLE, 1),
            make_record(HWPTAG_LIST_HEADER, 2),
            make_record(HWPTAG_PARA_HEADER, 3),
            make_record(HWPTAG_PARA_HEADER, 0),
        ];
        assert_eq!(find_children_end(&records, 0), 4);
    }

    #[test]
    fn find_children_end_at_last_record() {
        let records = vec![make_record(HWPTAG_CTRL_HEADER, 0)];
        assert_eq!(find_children_end(&records, 0), 1);
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
}
