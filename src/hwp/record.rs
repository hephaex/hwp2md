#![allow(dead_code)]

use crate::error::Hwp2MdError;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Read;

pub const HWPTAG_BEGIN: u16 = 0x0010;

pub const HWPTAG_DOCUMENT_PROPERTIES: u16 = HWPTAG_BEGIN;
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

pub const CTRL_SECTION_DEF: u32 = ctrl_id(b"secd");
pub const CTRL_TABLE: u32 = ctrl_id(b"tbl ");
pub const CTRL_EQUATION: u32 = ctrl_id(b"eqed");
pub const CTRL_GSHAPE: u32 = ctrl_id(b"gso ");
pub const CTRL_HEADER: u32 = ctrl_id(b"daeh");
pub const CTRL_FOOTER: u32 = ctrl_id(b"toof");
pub const CTRL_FOOTNOTE: u32 = ctrl_id(b"fn  ");
pub const CTRL_ENDNOTE: u32 = ctrl_id(b"en  ");
pub const CTRL_PAGE_BREAK: u32 = ctrl_id(b"pgbk");
pub const CTRL_COL_BREAK: u32 = ctrl_id(b"clbk");
pub const CTRL_HYPERLINK: u32 = ctrl_id(b"hyln");

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
    (s, offset + 2 + count * 2)
}
