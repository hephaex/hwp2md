use crate::error::Hwp2MdError;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Read;

pub const HWPTAG_BEGIN: u16 = 0x0010;

pub const HWPTAG_DOCUMENT_PROPERTIES: u16 = HWPTAG_BEGIN;
#[allow(dead_code)]
pub const HWPTAG_ID_MAPPINGS: u16 = HWPTAG_BEGIN + 1;
pub const HWPTAG_BIN_DATA: u16 = HWPTAG_BEGIN + 2;
pub const HWPTAG_FACE_NAME: u16 = HWPTAG_BEGIN + 3;
pub const HWPTAG_CHAR_SHAPE: u16 = HWPTAG_BEGIN + 8;
pub const HWPTAG_PARA_SHAPE: u16 = HWPTAG_BEGIN + 14;

pub const HWPTAG_PARA_HEADER: u16 = HWPTAG_BEGIN + 50;
pub const HWPTAG_PARA_TEXT: u16 = HWPTAG_BEGIN + 51;
pub const HWPTAG_PARA_CHAR_SHAPE: u16 = HWPTAG_BEGIN + 52;
pub const HWPTAG_CTRL_HEADER: u16 = HWPTAG_BEGIN + 54;
pub const HWPTAG_LIST_HEADER: u16 = HWPTAG_BEGIN + 55;
pub const HWPTAG_TABLE: u16 = HWPTAG_BEGIN + 57;
pub const HWPTAG_EQEDIT: u16 = HWPTAG_BEGIN + 71;
/// GSOType record — contains picture/OLE shape specifics including BinData reference.
pub const HWPTAG_GSOTYPE: u16 = HWPTAG_BEGIN + 67;

pub const CTRL_TABLE: u32 = ctrl_id(b"tbl ");
#[allow(dead_code)]
pub const CTRL_EQUATION: u32 = ctrl_id(b"eqed");
pub const CTRL_GSHAPE: u32 = ctrl_id(b"gso ");
#[allow(dead_code)]
pub const CTRL_HEADER: u32 = ctrl_id(b"daeh");
#[allow(dead_code)]
pub const CTRL_FOOTER: u32 = ctrl_id(b"toof");
pub const CTRL_FOOTNOTE: u32 = ctrl_id(b"fn  ");
pub const CTRL_ENDNOTE: u32 = ctrl_id(b"en  ");
pub const CTRL_PAGE_BREAK: u32 = ctrl_id(b"pgbk");
pub const CTRL_COL_BREAK: u32 = ctrl_id(b"clbk");
pub const CTRL_HYPERLINK: u32 = ctrl_id(b"hyln");
pub const CTRL_RUBY: u32 = ctrl_id(b"ruby");

const fn ctrl_id(b: &[u8; 4]) -> u32 {
    (b[0] as u32) | ((b[1] as u32) << 8) | ((b[2] as u32) << 16) | ((b[3] as u32) << 24)
}

#[derive(Debug, Clone)]
pub struct Record {
    pub tag_id: u16,
    pub level: u16,
    pub data: Vec<u8>,
}

impl Record {
    #[allow(dead_code)]
    pub fn tag_name(&self) -> &'static str {
        match self.tag_id {
            HWPTAG_DOCUMENT_PROPERTIES => "DOCUMENT_PROPERTIES",
            HWPTAG_ID_MAPPINGS => "ID_MAPPINGS",
            HWPTAG_BIN_DATA => "BIN_DATA",
            HWPTAG_FACE_NAME => "FACE_NAME",
            HWPTAG_CHAR_SHAPE => "CHAR_SHAPE",
            HWPTAG_PARA_SHAPE => "PARA_SHAPE",
            HWPTAG_PARA_HEADER => "PARA_HEADER",
            HWPTAG_PARA_TEXT => "PARA_TEXT",
            HWPTAG_PARA_CHAR_SHAPE => "PARA_CHAR_SHAPE",
            HWPTAG_CTRL_HEADER => "CTRL_HEADER",
            HWPTAG_LIST_HEADER => "LIST_HEADER",
            HWPTAG_TABLE => "TABLE",
            HWPTAG_EQEDIT => "EQEDIT",
            HWPTAG_GSOTYPE => "GSOTYPE",
            _ => "UNKNOWN",
        }
    }
}

/// Maximum record data size to prevent unbounded allocation (64 MB).
const MAX_RECORD_SIZE: usize = 64 * 1024 * 1024;

pub fn parse_records<R: Read>(reader: &mut R) -> Result<Vec<Record>, Hwp2MdError> {
    let mut records = Vec::new();
    loop {
        let header = match reader.read_u32::<LittleEndian>() {
            Ok(h) => h,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(Hwp2MdError::Io(e)),
        };

        let tag_id = (header & 0x3FF) as u16;
        let level = ((header >> 10) & 0x3FF) as u16;
        let size_field = (header >> 20) & 0xFFF;

        let size = if size_field == 0xFFF {
            reader
                .read_u32::<LittleEndian>()
                .map_err(|e| Hwp2MdError::InvalidRecord(format!("extended size read: {e}")))?
                as usize
        } else {
            size_field as usize
        };

        if size > MAX_RECORD_SIZE {
            return Err(Hwp2MdError::InvalidRecord(format!(
                "record size {} exceeds maximum allowed {} (tag={})",
                size, MAX_RECORD_SIZE, tag_id
            )));
        }

        let mut data = vec![0u8; size];
        reader.read_exact(&mut data).map_err(|e| {
            Hwp2MdError::InvalidRecord(format!(
                "record data read (tag={}, size={}): {}",
                tag_id, size, e
            ))
        })?;

        records.push(Record {
            tag_id,
            level,
            data,
        });
    }
    Ok(records)
}

pub fn read_utf16le(data: &[u8], offset: usize, count: usize) -> String {
    let mut chars = Vec::with_capacity(count);
    for i in 0..count {
        let pos = offset + i * 2;
        if pos + 1 >= data.len() {
            break;
        }
        let code = u16::from_le_bytes([data[pos], data[pos + 1]]);
        chars.push(code);
    }
    String::from_utf16_lossy(&chars)
}

pub fn read_utf16le_str(data: &[u8], offset: usize) -> (String, usize) {
    if offset + 2 > data.len() {
        return (String::new(), offset);
    }
    let count = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
    let s = read_utf16le(data, offset + 2, count);
    (s, (offset + 2 + count * 2).min(data.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    // -----------------------------------------------------------------------
    // ctrl_id constant derivation
    // -----------------------------------------------------------------------

    #[test]
    fn ctrl_id_tbl_equals_constant() {
        assert_eq!(CTRL_TABLE, ctrl_id(b"tbl "));
    }

    #[test]
    fn ctrl_id_fn_equals_constant() {
        assert_eq!(CTRL_FOOTNOTE, ctrl_id(b"fn  "));
    }

    // -----------------------------------------------------------------------
    // Record::tag_name
    // -----------------------------------------------------------------------

    fn make_rec(tag_id: u16) -> Record {
        Record {
            tag_id,
            level: 0,
            data: Vec::new(),
        }
    }

    #[test]
    fn tag_name_document_properties() {
        assert_eq!(
            make_rec(HWPTAG_DOCUMENT_PROPERTIES).tag_name(),
            "DOCUMENT_PROPERTIES"
        );
    }

    #[test]
    fn tag_name_id_mappings() {
        assert_eq!(make_rec(HWPTAG_ID_MAPPINGS).tag_name(), "ID_MAPPINGS");
    }

    #[test]
    fn tag_name_bin_data() {
        assert_eq!(make_rec(HWPTAG_BIN_DATA).tag_name(), "BIN_DATA");
    }

    #[test]
    fn tag_name_face_name() {
        assert_eq!(make_rec(HWPTAG_FACE_NAME).tag_name(), "FACE_NAME");
    }

    #[test]
    fn tag_name_char_shape() {
        assert_eq!(make_rec(HWPTAG_CHAR_SHAPE).tag_name(), "CHAR_SHAPE");
    }

    #[test]
    fn tag_name_para_shape() {
        assert_eq!(make_rec(HWPTAG_PARA_SHAPE).tag_name(), "PARA_SHAPE");
    }

    #[test]
    fn tag_name_para_header() {
        assert_eq!(make_rec(HWPTAG_PARA_HEADER).tag_name(), "PARA_HEADER");
    }

    #[test]
    fn tag_name_para_text() {
        assert_eq!(make_rec(HWPTAG_PARA_TEXT).tag_name(), "PARA_TEXT");
    }

    #[test]
    fn tag_name_para_char_shape() {
        assert_eq!(
            make_rec(HWPTAG_PARA_CHAR_SHAPE).tag_name(),
            "PARA_CHAR_SHAPE"
        );
    }

    #[test]
    fn tag_name_ctrl_header() {
        assert_eq!(make_rec(HWPTAG_CTRL_HEADER).tag_name(), "CTRL_HEADER");
    }

    #[test]
    fn tag_name_list_header() {
        assert_eq!(make_rec(HWPTAG_LIST_HEADER).tag_name(), "LIST_HEADER");
    }

    #[test]
    fn tag_name_table() {
        assert_eq!(make_rec(HWPTAG_TABLE).tag_name(), "TABLE");
    }

    #[test]
    fn tag_name_eqedit() {
        assert_eq!(make_rec(HWPTAG_EQEDIT).tag_name(), "EQEDIT");
    }

    #[test]
    fn tag_name_gsotype() {
        assert_eq!(make_rec(HWPTAG_GSOTYPE).tag_name(), "GSOTYPE");
    }

    #[test]
    fn tag_name_unknown() {
        assert_eq!(make_rec(0x0000).tag_name(), "UNKNOWN");
    }

    // -----------------------------------------------------------------------
    // parse_records
    // -----------------------------------------------------------------------

    /// Build a minimal 4-byte record header with no payload.
    fn header_bytes(tag_id: u16, level: u16, size: u32) -> Vec<u8> {
        // header = tag_id (10 bits) | level (10 bits) | size_field (12 bits)
        // If size < 0xFFF, size_field == size; otherwise 0xFFF + extended u32.
        let size_field = if size < 0xFFF { size } else { 0xFFF };
        let raw: u32 =
            (tag_id as u32 & 0x3FF) | ((level as u32 & 0x3FF) << 10) | (size_field << 20);
        raw.to_le_bytes().to_vec()
    }

    #[test]
    fn parse_records_empty_input_returns_empty_vec() {
        let mut cursor = Cursor::new(vec![]);
        let records = parse_records(&mut cursor).unwrap();
        assert!(records.is_empty());
    }

    #[test]
    fn parse_records_single_zero_size_record() {
        let mut data = header_bytes(HWPTAG_PARA_HEADER, 0, 0);
        // No payload bytes.
        let mut cursor = Cursor::new(&mut data);
        let records = parse_records(&mut cursor).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].tag_id, HWPTAG_PARA_HEADER);
        assert_eq!(records[0].level, 0);
        assert!(records[0].data.is_empty());
    }

    #[test]
    fn parse_records_single_record_with_payload() {
        let payload = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let mut data = header_bytes(HWPTAG_PARA_TEXT, 1, 4);
        data.extend_from_slice(&payload);
        let mut cursor = Cursor::new(&data);
        let records = parse_records(&mut cursor).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].tag_id, HWPTAG_PARA_TEXT);
        assert_eq!(records[0].level, 1);
        assert_eq!(records[0].data, payload);
    }

    #[test]
    fn parse_records_multiple_records() {
        let mut data = header_bytes(HWPTAG_PARA_HEADER, 0, 0);
        data.extend_from_slice(&header_bytes(HWPTAG_PARA_TEXT, 1, 2));
        data.extend_from_slice(&[0x01, 0x02]); // payload for second record
        let mut cursor = Cursor::new(&data);
        let records = parse_records(&mut cursor).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].tag_id, HWPTAG_PARA_HEADER);
        assert_eq!(records[1].tag_id, HWPTAG_PARA_TEXT);
        assert_eq!(records[1].data, vec![0x01, 0x02]);
    }

    #[test]
    fn parse_records_extended_size_field() {
        // size_field == 0xFFF means the next u32 is the actual size.
        let size: u32 = 8;
        // Build header with size_field = 0xFFF.
        let raw: u32 = (HWPTAG_PARA_HEADER as u32 & 0x3FF)
            | (0u32 << 10) // level = 0
            | (0xFFF << 20);
        let mut data = raw.to_le_bytes().to_vec();
        data.extend_from_slice(&size.to_le_bytes()); // extended size
        data.extend_from_slice(&[0u8; 8]); // 8-byte payload
        let mut cursor = Cursor::new(&data);
        let records = parse_records(&mut cursor).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].data.len(), 8);
    }

    #[test]
    fn parse_records_truncated_payload_returns_error() {
        // Header says 4-byte payload but only 2 bytes follow.
        let mut data = header_bytes(HWPTAG_PARA_TEXT, 0, 4);
        data.extend_from_slice(&[0x01, 0x02]); // only 2 bytes
        let mut cursor = Cursor::new(&data);
        assert!(parse_records(&mut cursor).is_err());
    }

    // -----------------------------------------------------------------------
    // read_utf16le
    // -----------------------------------------------------------------------

    fn encode_utf16le(s: &str) -> Vec<u8> {
        let units: Vec<u16> = s.encode_utf16().collect();
        let mut buf = Vec::with_capacity(units.len() * 2);
        for u in units {
            buf.push((u & 0xFF) as u8);
            buf.push((u >> 8) as u8);
        }
        buf
    }

    #[test]
    fn read_utf16le_basic_ascii() {
        let data = encode_utf16le("Hello");
        let result = read_utf16le(&data, 0, 5);
        assert_eq!(result, "Hello");
    }

    #[test]
    fn read_utf16le_korean() {
        let data = encode_utf16le("한글");
        let result = read_utf16le(&data, 0, 2);
        assert_eq!(result, "한글");
    }

    #[test]
    fn read_utf16le_truncated_stops_gracefully() {
        // Only 2 bytes (one character) but count=5 requested — should return "A".
        let data = vec![b'A', 0x00]; // 'A' in UTF-16LE
        let result = read_utf16le(&data, 0, 5);
        assert_eq!(result, "A");
    }

    #[test]
    fn read_utf16le_with_offset() {
        // Prefix 2 bytes, then "Hi"
        let mut data = vec![0xFF, 0xFF]; // prefix junk
        data.extend_from_slice(&encode_utf16le("Hi"));
        let result = read_utf16le(&data, 2, 2);
        assert_eq!(result, "Hi");
    }

    #[test]
    fn read_utf16le_empty_count() {
        let data = encode_utf16le("abc");
        let result = read_utf16le(&data, 0, 0);
        assert!(result.is_empty());
    }

    // -----------------------------------------------------------------------
    // read_utf16le_str
    // -----------------------------------------------------------------------

    fn build_utf16le_str(s: &str) -> Vec<u8> {
        let units: Vec<u16> = s.encode_utf16().collect();
        let mut buf = Vec::new();
        buf.push((units.len() as u16 & 0xFF) as u8);
        buf.push((units.len() as u16 >> 8) as u8);
        for u in &units {
            buf.push((*u & 0xFF) as u8);
            buf.push((*u >> 8) as u8);
        }
        buf
    }

    #[test]
    fn read_utf16le_str_basic() {
        let data = build_utf16le_str("Test");
        let (s, new_offset) = read_utf16le_str(&data, 0);
        assert_eq!(s, "Test");
        assert_eq!(new_offset, data.len());
    }

    #[test]
    fn read_utf16le_str_empty_string() {
        // count = 0 → empty string, offset advances by 2
        let data = vec![0x00, 0x00];
        let (s, new_offset) = read_utf16le_str(&data, 0);
        assert!(s.is_empty());
        assert_eq!(new_offset, 2);
    }

    #[test]
    fn read_utf16le_str_offset_too_large_returns_empty() {
        let data = vec![0x01, 0x00, b'A', 0x00];
        let (s, new_offset) = read_utf16le_str(&data, 100); // way past end
        assert!(s.is_empty());
        assert_eq!(new_offset, 100); // offset unchanged
    }

    #[test]
    fn read_utf16le_str_at_nonzero_offset() {
        // Place a length-prefixed string starting at byte 4.
        let mut data = vec![0xFF, 0xFF, 0xFF, 0xFF]; // prefix junk
        data.extend_from_slice(&build_utf16le_str("OK"));
        let (s, new_offset) = read_utf16le_str(&data, 4);
        assert_eq!(s, "OK");
        assert_eq!(new_offset, data.len());
    }
}
