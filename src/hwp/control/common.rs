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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hwp::record::{
        HWPTAG_CTRL_HEADER, HWPTAG_LIST_HEADER, HWPTAG_PARA_HEADER, HWPTAG_TABLE,
    };

    fn make_record(tag_id: u16, level: u16) -> Record {
        Record {
            tag_id,
            level,
            data: Vec::new(),
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
}
