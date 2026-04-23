use super::common::find_children_end;
use crate::hwp::record::*;

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
pub(crate) fn parse_gshape_ctrl(records: &[Record], ctrl_idx: usize) -> (u16, u32, u32) {
    let rec = &records[ctrl_idx];

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
        if rec.tag_id == HWPTAG_GSOTYPE && rec.data.len() >= 4 {
            let kind = u32::from_le_bytes([rec.data[0], rec.data[1], rec.data[2], rec.data[3]]);
            if kind == 0 && rec.data.len() >= 6 {
                let candidate = u16::from_le_bytes([rec.data[4], rec.data[5]]);
                if candidate > 0 {
                    return candidate;
                }
            }
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hwp::record::{CTRL_GSHAPE, HWPTAG_CTRL_HEADER, HWPTAG_GSOTYPE, HWPTAG_PARA_HEADER};

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
        data[0..4].copy_from_slice(&1u32.to_le_bytes()); // kind = 1 (OLE)
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
}
