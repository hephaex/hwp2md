use crate::hwp::model::*;
use crate::hwp::record::read_utf16le_str;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

pub(crate) fn parse_char_shape(data: &[u8]) -> CharShape {
    // HWP 5.0 CharShape record layout:
    //   bytes  0-13: face_id array (7 × u16 = 14 bytes)
    //   bytes 14-20: ratio array   (7 × u8)
    //   bytes 21-27: spacing array (7 × i8)
    //   bytes 28-34: rel_size array(7 × u8)
    //   bytes 35-41: offset array  (7 × i8)
    //   bytes 42-45: height        (i32)
    //   bytes 46-49: attribute flags (u32) ← bold/italic/underline/strikethrough
    //   bytes 50-53: shadow space  (i16 × 2)
    //   bytes 54-57: color         (u32)
    let mut shape = CharShape::default();
    if data.len() < 58 {
        return shape;
    }

    let mut cur = Cursor::new(data);
    // Read first face_id; the remaining 6 face_ids are skipped via set_position.
    if let Ok(face_id) = cur.read_u16::<LittleEndian>() {
        shape.face_id = face_id;
    }

    // Jump directly to height at byte 42 instead of manually skipping each field.
    cur.set_position(42);

    if let Ok(h) = cur.read_i32::<LittleEndian>() {
        shape.height = h.max(0) as u32;
    }

    // Attribute flags at bytes 46-49.
    if data.len() >= 50 {
        let attr = u32::from_le_bytes([data[46], data[47], data[48], data[49]]);
        shape.bold = (attr & 0x01) != 0;
        shape.italic = (attr & 0x02) != 0;
        shape.underline = (attr & 0x04) != 0;
        shape.strikethrough = (attr & 0x40) != 0;
        let sub_super = (attr >> 16) & 0x03;
        shape.superscript = sub_super == 1;
        shape.subscript = sub_super == 2;
    }

    // Color at bytes 54-57.
    if data.len() >= 58 {
        shape.color = u32::from_le_bytes([data[54], data[55], data[56], data[57]]);
    }

    shape
}

pub(crate) fn parse_para_shape(data: &[u8]) -> ParaShape {
    let mut shape = ParaShape::default();
    if data.len() < 8 {
        return shape;
    }

    let attr1 = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let alignment_val = attr1 & 0x07;
    shape.alignment = match alignment_val {
        0 => Alignment::Justify,
        1 => Alignment::Left,
        2 => Alignment::Right,
        3 => Alignment::Center,
        _ => Alignment::Left,
    };
    let head_type = (attr1 >> 24) & 0x03;
    if head_type == 1 {
        let para_level = ((attr1 >> 26) & 0x07) as u8;
        shape.heading_type = Some(para_level);
    }

    if data.len() >= 16 {
        shape.margin_left = i32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        shape.margin_right = i32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        shape.indent = i32::from_le_bytes([data[12], data[13], data[14], data[15]]);
    }

    if data.len() >= 24 {
        shape.line_spacing_type = data[16];
        shape.line_spacing = i32::from_le_bytes([data[20], data[21], data[22], data[23]]);
    }

    // Numbering ID is at bytes 26-27 (u16 LE) when the record is long enough.
    // tab_def_id occupies bytes 24-25, numbering_id follows at 26-27.
    if data.len() >= 28 {
        let nid = u16::from_le_bytes([data[26], data[27]]);
        if nid > 0 {
            shape.numbering_id = Some(nid);
        }
    }

    shape
}

pub(crate) fn parse_bin_data_entry(data: &[u8]) -> Option<BinDataEntry> {
    if data.len() < 4 {
        return None;
    }

    let type_val = u16::from_le_bytes([data[0], data[1]]);
    let mut entry = BinDataEntry {
        r#type: type_val,
        ..Default::default()
    };

    let storage_type = type_val & 0x0F;
    let mut offset = 2;

    match storage_type {
        0 => {
            let (path, new_offset) = read_utf16le_str(data, offset);
            entry.abs_path = Some(path);
            offset = new_offset;
            let (rpath, _) = read_utf16le_str(data, offset);
            entry.rel_path = Some(rpath);
        }
        1 => {
            let (path, new_offset) = read_utf16le_str(data, offset);
            entry.abs_path = Some(path);
            offset = new_offset;
            let (rpath, new_offset) = read_utf16le_str(data, offset);
            entry.rel_path = Some(rpath);
            offset = new_offset;
            if offset + 2 <= data.len() {
                entry.id = u16::from_le_bytes([data[offset], data[offset + 1]]);
            }
            if offset + 4 <= data.len() {
                let (ext, _) = read_utf16le_str(data, offset + 2);
                entry.extension = ext;
            }
        }
        2 => {
            if offset + 2 <= data.len() {
                entry.id = u16::from_le_bytes([data[offset], data[offset + 1]]);
            }
            if offset + 4 <= data.len() {
                let (ext, _) = read_utf16le_str(data, offset + 2);
                entry.extension = ext;
            }
        }
        _ => {}
    }

    Some(entry)
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // parse_para_shape — numbering_id field
    // -----------------------------------------------------------------------

    #[test]
    fn parse_para_shape_numbering_id_none_when_short_data() {
        // Data shorter than 28 bytes must leave numbering_id as None.
        let data = vec![0u8; 24];
        let shape = parse_para_shape(&data);
        assert!(shape.numbering_id.is_none());
    }

    #[test]
    fn parse_para_shape_numbering_id_none_when_zero() {
        // A zero value at bytes 26-27 must not set numbering_id (treated as absent).
        let mut data = vec![0u8; 28];
        data[26] = 0;
        data[27] = 0;
        let shape = parse_para_shape(&data);
        assert!(shape.numbering_id.is_none());
    }

    #[test]
    fn parse_para_shape_numbering_id_some_when_nonzero() {
        // A non-zero value at bytes 26-27 must set numbering_id = Some(value).
        let mut data = vec![0u8; 28];
        let nid: u16 = 3;
        data[26..28].copy_from_slice(&nid.to_le_bytes());
        let shape = parse_para_shape(&data);
        assert_eq!(shape.numbering_id, Some(3));
    }

    #[test]
    fn parse_para_shape_numbering_id_large_value() {
        // Values up to u16::MAX must be preserved faithfully.
        let mut data = vec![0u8; 28];
        let nid: u16 = 0xFFFF;
        data[26..28].copy_from_slice(&nid.to_le_bytes());
        let shape = parse_para_shape(&data);
        assert_eq!(shape.numbering_id, Some(0xFFFF));
    }

    #[test]
    fn parse_para_shape_numbering_id_does_not_affect_alignment() {
        // Ensure numbering_id parsing does not corrupt the alignment field at bytes 0-3.
        let mut data = vec![0u8; 28];
        // alignment bits 0-2 = 3 → Center
        data[0] = 3;
        let nid: u16 = 5;
        data[26..28].copy_from_slice(&nid.to_le_bytes());
        let shape = parse_para_shape(&data);
        assert_eq!(shape.alignment, Alignment::Center);
        assert_eq!(shape.numbering_id, Some(5));
    }

    #[test]
    fn para_shape_default_numbering_id_is_none() {
        let shape = ParaShape::default();
        assert!(shape.numbering_id.is_none());
    }
}
