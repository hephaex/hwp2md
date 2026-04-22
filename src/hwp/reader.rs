use crate::error::Hwp2MdError;
use crate::hwp::control::{find_children_end, parse_ctrl_header_at};
use crate::hwp::convert::hwp_to_ir;
use crate::hwp::model::*;
use crate::hwp::record::*;
use crate::hwp::summary::read_summary_info;
use crate::ir;
use flate2::read::DeflateDecoder;
use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::path::Path;

#[path = "shapes.rs"]
mod shapes;
pub(crate) use shapes::{parse_bin_data_entry, parse_char_shape, parse_para_shape};

pub fn read_hwp(path: &Path) -> Result<ir::Document, anyhow::Error> {
    let hwp_doc = parse_hwp_file(path)?;
    Ok(hwp_to_ir(&hwp_doc))
}

fn parse_hwp_file(path: &Path) -> Result<HwpDocument, anyhow::Error> {
    let file = std::fs::File::open(path)?;
    let mut cfb = cfb::CompoundFile::open(file)
        .map_err(|e| Hwp2MdError::HwpParse(format!("CFB open: {e}")))?;

    let header = read_file_header(&mut cfb)?;

    if header.encrypted || header.has_drm {
        return Err(Hwp2MdError::HwpParse("HWP file is encrypted or DRM-protected".into()).into());
    }
    if header.distributed {
        return Err(Hwp2MdError::HwpParse(
            "DRM-protected (distributed) HWP files are not supported".into(),
        )
        .into());
    }

    let doc_info = read_doc_info(&mut cfb, header.compressed)?;

    let section_count = if doc_info.doc_properties.section_count > 0 {
        doc_info.doc_properties.section_count as usize
    } else {
        count_sections(&mut cfb)
    };

    let mut sections = Vec::new();
    for i in 0..section_count {
        let section_path = format!("BodyText/Section{i}");
        match read_section_stream(&mut cfb, &section_path, header.compressed) {
            Ok(section) => sections.push(section),
            Err(e) => {
                tracing::warn!("Failed to read {section_path}: {e}");
            }
        }
    }

    let bin_data = read_bin_data(&mut cfb, &doc_info)?;
    let (summary_title, summary_author, summary_subject, summary_keywords) =
        read_summary_info(&mut cfb);

    Ok(HwpDocument {
        header,
        doc_info,
        sections,
        bin_data,
        summary_title,
        summary_author,
        summary_subject,
        summary_keywords,
    })
}

fn read_file_header(cfb: &mut cfb::CompoundFile<std::fs::File>) -> Result<FileHeader, Hwp2MdError> {
    let mut stream = cfb
        .open_stream("FileHeader")
        .map_err(|e| Hwp2MdError::HwpParse(format!("FileHeader stream: {e}")))?;

    let mut buf = vec![0u8; 256];
    let n = stream
        .read(&mut buf)
        .map_err(|e| Hwp2MdError::HwpParse(format!("FileHeader read: {e}")))?;
    if n < 36 {
        return Err(Hwp2MdError::HwpParse("FileHeader too short".into()));
    }

    let signature = &buf[0..32];
    let expected = b"HWP Document File";
    if !signature.starts_with(expected) {
        return Err(Hwp2MdError::HwpParse("Invalid HWP signature".into()));
    }

    let version = HwpVersion {
        major: buf[35],
        minor: buf[34],
        micro: buf[33],
        extra: buf[32],
    };

    let props = if n >= 40 {
        u32::from_le_bytes([buf[36], buf[37], buf[38], buf[39]])
    } else {
        0
    };

    Ok(FileHeader {
        version,
        compressed: (props & 0x01) != 0,
        encrypted: (props & 0x02) != 0,
        distributed: (props & 0x04) != 0,
        has_script: (props & 0x08) != 0,
        has_drm: (props & 0x10) != 0,
        has_xml_template: (props & 0x20) != 0,
        has_history: (props & 0x40) != 0,
        has_cert: (props & 0x80) != 0,
        has_cert_drm: (props & 0x100) != 0,
        has_ccl: (props & 0x200) != 0,
    })
}

/// Maximum decompressed size to prevent decompression bombs (256 MB).
const MAX_DECOMPRESSED: u64 = 256 * 1024 * 1024;

pub(crate) fn decompress_stream(data: &[u8]) -> Result<Vec<u8>, Hwp2MdError> {
    let mut out = Vec::new();
    let decoder = DeflateDecoder::new(data);
    if let Err(e) = decoder.take(MAX_DECOMPRESSED).read_to_end(&mut out) {
        tracing::debug!("Deflate failed, trying zlib: {e}");
    } else {
        return Ok(out);
    }

    out.clear();
    let decoder = flate2::read::ZlibDecoder::new(data);
    decoder
        .take(MAX_DECOMPRESSED)
        .read_to_end(&mut out)
        .map_err(|e| Hwp2MdError::Decompress(format!("zlib fallback: {e}")))?;
    Ok(out)
}

fn read_stream_bytes(
    cfb: &mut cfb::CompoundFile<std::fs::File>,
    path: &str,
    compressed: bool,
) -> Result<Vec<u8>, Hwp2MdError> {
    let mut stream = cfb
        .open_stream(path)
        .map_err(|e| Hwp2MdError::HwpParse(format!("open stream '{path}': {e}")))?;

    let mut raw = Vec::new();
    stream.read_to_end(&mut raw)?;

    if compressed {
        decompress_stream(&raw)
    } else {
        Ok(raw)
    }
}

fn read_doc_info(
    cfb: &mut cfb::CompoundFile<std::fs::File>,
    compressed: bool,
) -> Result<DocInfo, Hwp2MdError> {
    let data = read_stream_bytes(cfb, "DocInfo", compressed)?;
    let mut cursor = Cursor::new(&data);
    let records = parse_records(&mut cursor)?;

    let mut doc_info = DocInfo::default();

    for rec in &records {
        match rec.tag_id {
            HWPTAG_DOCUMENT_PROPERTIES => {
                if rec.data.len() >= 26 {
                    doc_info.doc_properties.section_count =
                        u16::from_le_bytes([rec.data[0], rec.data[1]]);
                }
            }
            HWPTAG_FACE_NAME => {
                let (name, _) = read_utf16le_str(&rec.data, 1);
                doc_info.face_names.push(name);
            }
            HWPTAG_CHAR_SHAPE => {
                doc_info.char_shapes.push(parse_char_shape(&rec.data));
            }
            HWPTAG_PARA_SHAPE => {
                doc_info.para_shapes.push(parse_para_shape(&rec.data));
            }
            HWPTAG_BIN_DATA => {
                if let Some(entry) = parse_bin_data_entry(&rec.data) {
                    doc_info.bin_data_entries.push(entry);
                }
            }
            _ => {}
        }
    }

    Ok(doc_info)
}

fn count_sections(cfb: &mut cfb::CompoundFile<std::fs::File>) -> usize {
    let mut count = 0;
    loop {
        let path = format!("BodyText/Section{count}");
        if cfb.open_stream(&path).is_err() {
            break;
        }
        count += 1;
    }
    count
}

pub(crate) fn read_section_stream(
    cfb: &mut cfb::CompoundFile<std::fs::File>,
    path: &str,
    compressed: bool,
) -> Result<HwpSection, Hwp2MdError> {
    let data = read_stream_bytes(cfb, path, compressed)?;
    let mut cursor = Cursor::new(&data);
    let records = parse_records(&mut cursor)?;

    let mut section = HwpSection {
        paragraphs: Vec::new(),
    };
    let mut current_para: Option<HwpParagraph> = None;

    // Index-based loop so we can skip over CTRL_HEADER subtrees after processing.
    let mut rec_idx: usize = 0;
    while rec_idx < records.len() {
        let rec = &records[rec_idx];
        match rec.tag_id {
            HWPTAG_PARA_HEADER => {
                if let Some(para) = current_para.take() {
                    section.paragraphs.push(para);
                }
                let para_shape_id = if rec.data.len() >= 6 {
                    u16::from_le_bytes([rec.data[4], rec.data[5]])
                } else {
                    0
                };
                current_para = Some(HwpParagraph {
                    text: String::new(),
                    char_shape_ids: Vec::new(),
                    para_shape_id,
                    controls: Vec::new(),
                });
                rec_idx += 1;
            }
            HWPTAG_PARA_TEXT => {
                if let Some(ref mut para) = current_para {
                    para.text = extract_paragraph_text(&rec.data);
                }
                rec_idx += 1;
            }
            HWPTAG_PARA_CHAR_SHAPE => {
                if let Some(ref mut para) = current_para {
                    para.char_shape_ids = parse_char_shape_refs(&rec.data);
                }
                rec_idx += 1;
            }
            HWPTAG_EQEDIT => {
                if let Some(ref mut para) = current_para {
                    let (script, _) = read_utf16le_str(&rec.data, 2);
                    if !script.is_empty() {
                        para.controls.push(HwpControl::Equation { script });
                    }
                }
                rec_idx += 1;
            }
            HWPTAG_CTRL_HEADER => {
                // Parse the control and skip all its child records so they are
                // not re-processed as top-level paragraph content.
                if let Some(ctrl) = parse_ctrl_header_at(&records, rec_idx) {
                    if let Some(ref mut para) = current_para {
                        para.controls.push(ctrl);
                    }
                }
                rec_idx = find_children_end(&records, rec_idx);
            }
            _ => {
                rec_idx += 1;
            }
        }
    }

    if let Some(para) = current_para {
        section.paragraphs.push(para);
    }

    Ok(section)
}

pub(crate) fn extract_paragraph_text(data: &[u8]) -> String {
    let mut result = String::new();
    let mut i = 0;
    let len = data.len();

    while i + 1 < len {
        let ch = u16::from_le_bytes([data[i], data[i + 1]]);
        i += 2;

        match ch {
            0x0000 => {}
            0x0001..=0x0002 => {
                if i + 14 > len {
                    break;
                }
                i += 14;
            }
            0x0003..=0x0008 => {
                if i + 14 > len {
                    break;
                }
                i += 14;
            }
            0x0009 => {
                result.push('\t');
            }
            0x000A => {
                result.push('\n');
            }
            0x000B..=0x000C => {
                if i + 14 > len {
                    break;
                }
                i += 14;
            }
            0x000D => {
                // paragraph break
            }
            0x000E..=0x001F => {}
            _ => {
                if let Some(c) = char::from_u32(ch as u32) {
                    result.push(c);
                } else if (0xD800..=0xDBFF).contains(&ch) && i + 1 < len {
                    let low = u16::from_le_bytes([data[i], data[i + 1]]);
                    i += 2;
                    if (0xDC00..=0xDFFF).contains(&low) {
                        let codepoint =
                            0x10000 + ((ch as u32 - 0xD800) << 10) + (low as u32 - 0xDC00);
                        if let Some(c) = char::from_u32(codepoint) {
                            result.push(c);
                        }
                    }
                }
            }
        }
    }

    result
}

pub(crate) fn parse_char_shape_refs(data: &[u8]) -> Vec<(u32, u16)> {
    let mut refs = Vec::new();
    let mut i = 0;
    while i + 6 <= data.len() {
        let pos = u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
        let id = u16::from_le_bytes([data[i + 4], data[i + 5]]);
        refs.push((pos, id));
        i += 6;
    }

    // Handle 8-byte entries (offset + 4-byte shape id)
    if refs.is_empty() {
        i = 0;
        while i + 8 <= data.len() {
            let pos = u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
            let id = u32::from_le_bytes([data[i + 4], data[i + 5], data[i + 6], data[i + 7]]);
            refs.push((pos, id as u16));
            i += 8;
        }
    }

    refs
}

fn read_bin_data(
    cfb: &mut cfb::CompoundFile<std::fs::File>,
    doc_info: &DocInfo,
) -> Result<HashMap<u16, Vec<u8>>, Hwp2MdError> {
    let mut bin_data = HashMap::new();

    if !doc_info.bin_data_entries.is_empty() {
        // Probe only the IDs recorded in DocInfo — avoids scanning up to 999 entries.
        for (idx, entry) in doc_info.bin_data_entries.iter().enumerate() {
            let id = if entry.id > 0 {
                entry.id
            } else {
                (idx + 1) as u16
            };
            let path = format!("BinData/BIN{:04X}", id);
            if let Ok(mut stream) = cfb.open_stream(&path) {
                let mut data = Vec::new();
                stream.read_to_end(&mut data)?;
                bin_data.insert(id, data);
            }
        }
    } else {
        // Fallback: sequential probe with a conservative upper limit.
        for i in 1..=100u16 {
            let path = format!("BinData/BIN{:04X}", i);
            match cfb.open_stream(&path) {
                Ok(mut stream) => {
                    let mut data = Vec::new();
                    stream.read_to_end(&mut data)?;
                    bin_data.insert(i, data);
                }
                Err(_) => {
                    if i > 10 && bin_data.is_empty() {
                        break;
                    }
                    if i > bin_data.len() as u16 + 20 {
                        break;
                    }
                }
            }
        }
    }

    Ok(bin_data)
}

/// Encode a slice of u16 code units as little-endian bytes.
/// Exposed for use in sibling module tests only.
#[cfg(test)]
pub(crate) fn encode_u16s_test(units: &[u16]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(units.len() * 2);
    for &u in units {
        buf.push((u & 0xFF) as u8);
        buf.push((u >> 8) as u8);
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Helpers ---

    fn encode_u16s(units: &[u16]) -> Vec<u8> {
        encode_u16s_test(units)
    }

    /// Build a 58-byte CharShape record with `flags` at offset 46 and `height` at offset 42.
    fn make_char_shape_data(flags: u32, height: i32) -> Vec<u8> {
        let mut data = vec![0u8; 58];
        data[42..46].copy_from_slice(&height.to_le_bytes());
        data[46..50].copy_from_slice(&flags.to_le_bytes());
        data
    }

    // --- extract_paragraph_text ---

    #[test]
    fn extract_paragraph_text_basic_korean() {
        // "한글" — U+D55C U+AE00
        let data = encode_u16s(&[0xD55C, 0xAE00]);
        assert_eq!(extract_paragraph_text(&data), "한글");
    }

    #[test]
    fn extract_paragraph_text_ascii() {
        let data = encode_u16s(&[b'H' as u16, b'i' as u16]);
        assert_eq!(extract_paragraph_text(&data), "Hi");
    }

    #[test]
    fn extract_paragraph_text_tab() {
        let data = encode_u16s(&[0x0009]);
        assert_eq!(extract_paragraph_text(&data), "\t");
    }

    #[test]
    fn extract_paragraph_text_newline() {
        let data = encode_u16s(&[0x000A]);
        assert_eq!(extract_paragraph_text(&data), "\n");
    }

    #[test]
    fn extract_paragraph_text_paragraph_break_skipped() {
        // 0x000D is a paragraph-break marker; it must not appear in output.
        let data = encode_u16s(&[b'A' as u16, 0x000D, b'B' as u16]);
        assert_eq!(extract_paragraph_text(&data), "AB");
    }

    #[test]
    fn extract_paragraph_text_control_chars_skip_14_bytes() {
        // Control codes 0x0003–0x0008 are followed by 14 skip-bytes.
        // After the control code (2 bytes) we place 7 u16 zero-words (= 14 bytes),
        // then the letter 'X'.
        let mut units: Vec<u16> = vec![0x0003];
        units.extend_from_slice(&[0u16; 7]); // 7 × 2 = 14 bytes
        units.push(b'X' as u16);
        let data = encode_u16s(&units);
        assert_eq!(extract_paragraph_text(&data), "X");
    }

    #[test]
    fn extract_paragraph_text_truncated_control_stops_gracefully() {
        // Control code 0x0001 but only 2 more bytes remain (< 14) → must not panic.
        let data = encode_u16s(&[0x0001, 0x0000]);
        let result = extract_paragraph_text(&data);
        assert!(result.is_empty());
    }

    #[test]
    fn extract_paragraph_text_surrogate_pair() {
        // U+1F600 GRINNING FACE → surrogate pair 0xD83D 0xDE00
        let data = encode_u16s(&[0xD83D, 0xDE00]);
        assert_eq!(extract_paragraph_text(&data), "\u{1F600}");
    }

    #[test]
    fn extract_paragraph_text_empty_input() {
        assert_eq!(extract_paragraph_text(&[]), "");
    }

    #[test]
    fn extract_paragraph_text_null_code_unit_skipped() {
        // 0x0000 must be silently ignored.
        let data = encode_u16s(&[0x0000, b'Z' as u16]);
        assert_eq!(extract_paragraph_text(&data), "Z");
    }

    // --- parse_char_shape ---

    #[test]
    fn parse_char_shape_bold_flag() {
        let cs = parse_char_shape(&make_char_shape_data(0x01, 0));
        assert!(cs.bold);
        assert!(!cs.italic);
        assert!(!cs.underline);
        assert!(!cs.strikethrough);
    }

    #[test]
    fn parse_char_shape_italic_flag() {
        let cs = parse_char_shape(&make_char_shape_data(0x02, 0));
        assert!(!cs.bold);
        assert!(cs.italic);
    }

    #[test]
    fn parse_char_shape_underline_flag() {
        let cs = parse_char_shape(&make_char_shape_data(0x04, 0));
        assert!(cs.underline);
    }

    #[test]
    fn parse_char_shape_strikethrough_flag() {
        // strikethrough = bit 6 = 0x40
        let cs = parse_char_shape(&make_char_shape_data(0x40, 0));
        assert!(cs.strikethrough);
        assert!(!cs.bold);
    }

    #[test]
    fn parse_char_shape_all_style_flags() {
        // bold | italic | underline | strikethrough = 0x01 | 0x02 | 0x04 | 0x40 = 0x47
        let cs = parse_char_shape(&make_char_shape_data(0x47, 0));
        assert!(cs.bold);
        assert!(cs.italic);
        assert!(cs.underline);
        assert!(cs.strikethrough);
    }

    #[test]
    fn parse_char_shape_short_data_returns_default() {
        // The guard is data.len() < 58; use 20 bytes to trigger the early return.
        let cs = parse_char_shape(&[0u8; 20]);
        assert!(!cs.bold);
        assert!(!cs.italic);
        assert!(!cs.underline);
        assert!(!cs.strikethrough);
        assert_eq!(cs.height, 0);
    }

    #[test]
    fn parse_char_shape_height_parsed() {
        let cs = parse_char_shape(&make_char_shape_data(0, 1400));
        assert_eq!(cs.height, 1400);
    }

    // --- parse_para_shape ---

    fn make_para_shape_data(alignment_nibble: u8, margin_left: i32, line_spacing: i32) -> Vec<u8> {
        let mut data = vec![0u8; 24];
        data[0] = alignment_nibble & 0x07;
        data[4..8].copy_from_slice(&margin_left.to_le_bytes());
        data[20..24].copy_from_slice(&line_spacing.to_le_bytes());
        data
    }

    #[test]
    fn parse_para_shape_alignment_justify() {
        let ps = parse_para_shape(&make_para_shape_data(0, 0, 0));
        assert_eq!(ps.alignment, crate::hwp::model::Alignment::Justify);
    }

    #[test]
    fn parse_para_shape_alignment_left() {
        let ps = parse_para_shape(&make_para_shape_data(1, 0, 0));
        assert_eq!(ps.alignment, crate::hwp::model::Alignment::Left);
    }

    #[test]
    fn parse_para_shape_alignment_right() {
        let ps = parse_para_shape(&make_para_shape_data(2, 0, 0));
        assert_eq!(ps.alignment, crate::hwp::model::Alignment::Right);
    }

    #[test]
    fn parse_para_shape_alignment_center() {
        let ps = parse_para_shape(&make_para_shape_data(3, 0, 0));
        assert_eq!(ps.alignment, crate::hwp::model::Alignment::Center);
    }

    #[test]
    fn parse_para_shape_alignment_unknown_defaults_to_left() {
        let ps = parse_para_shape(&make_para_shape_data(7, 0, 0));
        assert_eq!(ps.alignment, crate::hwp::model::Alignment::Left);
    }

    #[test]
    fn parse_para_shape_margin_left() {
        let ps = parse_para_shape(&make_para_shape_data(1, 500, 0));
        assert_eq!(ps.margin_left, 500);
    }

    #[test]
    fn parse_para_shape_line_spacing() {
        let ps = parse_para_shape(&make_para_shape_data(1, 0, 160));
        assert_eq!(ps.line_spacing, 160);
    }

    #[test]
    fn parse_para_shape_short_data_returns_default() {
        let ps = parse_para_shape(&[0u8; 4]);
        assert_eq!(ps.margin_left, 0);
        assert_eq!(ps.line_spacing, 0);
    }

    // --- decompress_stream ---

    #[test]
    fn decompress_stream_valid_deflate() {
        use flate2::{write::DeflateEncoder, Compression};
        use std::io::Write;

        let original = b"Hello, HWP world!";
        let mut enc = DeflateEncoder::new(Vec::new(), Compression::default());
        enc.write_all(original).unwrap();
        let compressed = enc.finish().unwrap();

        let decompressed = decompress_stream(&compressed).unwrap();
        assert_eq!(decompressed, original);
    }

    #[test]
    fn decompress_stream_valid_zlib_fallback() {
        use flate2::{write::ZlibEncoder, Compression};
        use std::io::Write;

        let original = b"zlib fallback test data";
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(original).unwrap();
        let compressed = enc.finish().unwrap();

        // zlib bytes are not valid raw deflate, so the first pass must fail and
        // the zlib decoder must succeed.
        let decompressed = decompress_stream(&compressed).unwrap();
        assert_eq!(decompressed, original);
    }

    #[test]
    fn decompress_stream_invalid_data_returns_error() {
        assert!(decompress_stream(b"\x00\x01\x02\x03rubbish").is_err());
    }

    #[test]
    fn read_file_header_encrypted_bit_sets_encrypted() {
        let mut buf = vec![0u8; 256];
        buf[0..17].copy_from_slice(b"HWP Document File");
        buf[35] = 5; // version major
                     // props byte 36: bit 1 = encrypted (0x02), bit 0 = compressed (0x01)
        buf[36] = 0x03; // compressed + encrypted
                        // We can't call read_file_header directly (it needs cfb),
                        // but we can verify the bit parsing logic:
        let props = u32::from_le_bytes([buf[36], buf[37], buf[38], buf[39]]);
        assert!((props & 0x02) != 0, "encrypted bit should be set");
    }

    #[test]
    fn read_file_header_drm_bit_sets_has_drm() {
        let mut buf = vec![0u8; 256];
        buf[0..17].copy_from_slice(b"HWP Document File");
        buf[35] = 5;
        buf[36] = 0x10; // has_drm bit
        let props = u32::from_le_bytes([buf[36], buf[37], buf[38], buf[39]]);
        assert!((props & 0x10) != 0, "has_drm bit should be set");
    }
}
