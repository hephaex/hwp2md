use crate::error::Hwp2MdError;
use crate::hwp::model::*;
use crate::hwp::record::*;
use crate::ir;
use byteorder::{LittleEndian, ReadBytesExt};
use flate2::read::DeflateDecoder;
use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::path::Path;

pub fn read_hwp(path: &Path) -> Result<ir::Document, anyhow::Error> {
    let hwp_doc = parse_hwp_file(path)?;
    Ok(hwp_to_ir(&hwp_doc))
}

fn parse_hwp_file(path: &Path) -> Result<HwpDocument, anyhow::Error> {
    let file = std::fs::File::open(path)?;
    let mut cfb = cfb::CompoundFile::open(file)
        .map_err(|e| Hwp2MdError::HwpParse(format!("CFB open: {e}")))?;

    let header = read_file_header(&mut cfb)?;

    if header.encrypted {
        anyhow::bail!("Encrypted HWP files are not supported");
    }
    if header.distributed {
        anyhow::bail!("DRM-protected (distributed) HWP files are not supported");
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

    let bin_data = read_bin_data(&mut cfb)?;

    Ok(HwpDocument {
        header,
        doc_info,
        sections,
        bin_data,
    })
}

fn read_file_header(cfb: &mut cfb::CompoundFile<std::fs::File>) -> Result<FileHeader, Hwp2MdError> {
    let mut stream = cfb
        .open_stream("FileHeader")
        .map_err(|e| Hwp2MdError::HwpParse(format!("FileHeader stream: {e}")))?;

    let mut buf = vec![0u8; 256];
    let n = stream.read(&mut buf).map_err(|e| Hwp2MdError::HwpParse(format!("FileHeader read: {e}")))?;
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

/// HWP font height is in 1/100 point units (HWP internal unit).
/// e.g. 1600 = 16pt, 1400 = 14pt, 1200 = 12pt.
const HEADING1_MIN_HEIGHT: u32 = 1600; // 16pt
const HEADING2_MIN_HEIGHT: u32 = 1400; // 14pt
const HEADING3_MIN_HEIGHT: u32 = 1200; // 12pt

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

    let alignment_val = data[0] & 0x07;
    shape.alignment = match alignment_val {
        0 => Alignment::Justify,
        1 => Alignment::Left,
        2 => Alignment::Right,
        3 => Alignment::Center,
        _ => Alignment::Left,
    };

    if data.len() >= 16 {
        shape.margin_left = i32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        shape.margin_right = i32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        shape.indent = i32::from_le_bytes([data[12], data[13], data[14], data[15]]);
    }

    if data.len() >= 24 {
        shape.line_spacing_type = data[16];
        shape.line_spacing = i32::from_le_bytes([data[20], data[21], data[22], data[23]]);
    }

    shape
}

fn parse_bin_data_entry(data: &[u8]) -> Option<BinDataEntry> {
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

fn read_section_stream(
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

    for rec in &records {
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
            }
            HWPTAG_PARA_TEXT => {
                if let Some(ref mut para) = current_para {
                    para.text = extract_paragraph_text(&rec.data);
                }
            }
            HWPTAG_PARA_CHAR_SHAPE => {
                if let Some(ref mut para) = current_para {
                    para.char_shape_ids = parse_char_shape_refs(&rec.data);
                }
            }
            HWPTAG_EQEDIT => {
                if let Some(ref mut para) = current_para {
                    let (script, _) = read_utf16le_str(&rec.data, 2);
                    if !script.is_empty() {
                        para.controls.push(HwpControl::Equation { script });
                    }
                }
            }
            _ => {}
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

fn parse_char_shape_refs(data: &[u8]) -> Vec<(u32, u16)> {
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

fn read_bin_data(cfb: &mut cfb::CompoundFile<std::fs::File>) -> Result<HashMap<u16, Vec<u8>>, Hwp2MdError> {
    let mut bin_data = HashMap::new();

    for i in 1..=999u16 {
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

    Ok(bin_data)
}

fn hwp_to_ir(hwp: &HwpDocument) -> ir::Document {
    let mut doc = ir::Document::new();

    doc.metadata.title = None;
    doc.metadata.author = None;

    for section in &hwp.sections {
        let mut ir_section = ir::Section {
            blocks: Vec::new(),
        };

        for para in &section.paragraphs {
            let blocks = paragraph_to_blocks(para, &hwp.doc_info);
            ir_section.blocks.extend(blocks);
        }

        doc.sections.push(ir_section);
    }

    for (id, data) in &hwp.bin_data {
        let mime = guess_mime(data);
        let ext = mime_to_ext(&mime);
        doc.assets.push(ir::Asset {
            name: format!("image_{id}.{ext}"),
            data: data.clone(),
            mime_type: mime,
        });
    }

    doc
}

fn paragraph_to_blocks(para: &HwpParagraph, doc_info: &DocInfo) -> Vec<ir::Block> {
    let text = para.text.trim();
    if text.is_empty() {
        return Vec::new();
    }

    let heading_level = detect_heading_level(para, doc_info);

    let inlines = build_inlines(para, doc_info);

    if inlines.is_empty() {
        return Vec::new();
    }

    if let Some(level) = heading_level {
        vec![ir::Block::Heading { level, inlines }]
    } else {
        vec![ir::Block::Paragraph { inlines }]
    }
}

fn detect_heading_level(para: &HwpParagraph, doc_info: &DocInfo) -> Option<u8> {
    let ps_id = para.para_shape_id as usize;
    if ps_id < doc_info.para_shapes.len() {
        if let Some(level) = doc_info.para_shapes[ps_id].heading_type {
            if level < 7 {
                return Some(level + 1);
            }
        }
    }

    let text = para.text.trim();
    if text.len() < 100 {
        if let Some(first_cs) = para.char_shape_ids.first() {
            let cs_id = first_cs.1 as usize;
            if cs_id < doc_info.char_shapes.len() {
                let cs = &doc_info.char_shapes[cs_id];
                if cs.height >= HEADING1_MIN_HEIGHT && cs.bold {
                    return Some(1);
                }
                if cs.height >= HEADING2_MIN_HEIGHT && cs.bold {
                    return Some(2);
                }
                if cs.height >= HEADING3_MIN_HEIGHT && cs.bold {
                    return Some(3);
                }
            }
        }
    }

    None
}

fn build_inlines(para: &HwpParagraph, doc_info: &DocInfo) -> Vec<ir::Inline> {
    let text = &para.text;
    if text.is_empty() {
        return Vec::new();
    }

    if para.char_shape_ids.is_empty() {
        return vec![ir::Inline::plain(text.clone())];
    }

    let chars: Vec<char> = text.chars().collect();
    let mut inlines = Vec::new();
    let char_refs = &para.char_shape_ids;

    for (idx, &(pos, cs_id)) in char_refs.iter().enumerate() {
        let start = pos as usize;
        let end = if idx + 1 < char_refs.len() {
            char_refs[idx + 1].0 as usize
        } else {
            chars.len()
        };

        if start >= chars.len() {
            break;
        }
        let end = end.min(chars.len());
        let segment: String = chars[start..end].iter().collect();
        let segment = segment.trim_end_matches('\r').to_string();

        if segment.is_empty() {
            continue;
        }

        let cs_idx = cs_id as usize;
        let inline = if cs_idx < doc_info.char_shapes.len() {
            let cs = &doc_info.char_shapes[cs_idx];
            ir::Inline {
                text: segment,
                bold: cs.bold,
                italic: cs.italic,
                underline: cs.underline,
                strikethrough: cs.strikethrough,
                superscript: cs.superscript,
                subscript: cs.subscript,
                ..ir::Inline::default()
            }
        } else {
            ir::Inline::plain(segment)
        };

        inlines.push(inline);
    }

    inlines
}

fn guess_mime(data: &[u8]) -> String {
    if data.len() < 4 {
        return "application/octet-stream".to_string();
    }
    match &data[..4] {
        [0x89, b'P', b'N', b'G'] => "image/png".to_string(),
        [0xFF, 0xD8, 0xFF, _] => "image/jpeg".to_string(),
        [b'G', b'I', b'F', b'8'] => "image/gif".to_string(),
        [b'B', b'M', _, _] => "image/bmp".to_string(),
        _ => {
            if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
                "image/webp".to_string()
            } else {
                "application/octet-stream".to_string()
            }
        }
    }
}

fn mime_to_ext(mime: &str) -> &'static str {
    match mime {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/bmp" => "bmp",
        "image/webp" => "webp",
        _ => "bin",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Encode a slice of u16 code units as little-endian bytes (the wire format
    /// used by HWPTAG_PARA_TEXT records).
    fn encode_u16s(units: &[u16]) -> Vec<u8> {
        let mut buf = Vec::with_capacity(units.len() * 2);
        for &u in units {
            buf.push((u & 0xFF) as u8);
            buf.push((u >> 8) as u8);
        }
        buf
    }

    /// Build a CharShape record matching the current layout in `parse_char_shape`:
    ///   bytes  0-13: face_id array (7 × u16)
    ///   bytes 14-41: ratio/spacing/rel_size/offset arrays
    ///   bytes 42-45: height (i32 LE)
    ///   bytes 46-49: attribute flags (u32 LE) — bold=0x01, italic=0x02, underline=0x04, strike=0x40
    ///   bytes 50-53: shadow space
    ///   bytes 54-57: color (u32)
    /// The minimum size checked is 58 bytes.
    fn make_char_shape_data(flags: u32, height: i32) -> Vec<u8> {
        let mut data = vec![0u8; 58];
        let hb = height.to_le_bytes();
        data[42] = hb[0];
        data[43] = hb[1];
        data[44] = hb[2];
        data[45] = hb[3];
        let fb = flags.to_le_bytes();
        data[46] = fb[0];
        data[47] = fb[1];
        data[48] = fb[2];
        data[49] = fb[3];
        data
    }

    // -----------------------------------------------------------------------
    // extract_paragraph_text
    // -----------------------------------------------------------------------

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

    // -----------------------------------------------------------------------
    // parse_char_shape
    // -----------------------------------------------------------------------

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

    // -----------------------------------------------------------------------
    // parse_para_shape
    // -----------------------------------------------------------------------

    fn make_para_shape_data(alignment_nibble: u8, margin_left: i32, line_spacing: i32) -> Vec<u8> {
        let mut data = vec![0u8; 24];
        data[0] = alignment_nibble & 0x07;
        let ml = margin_left.to_le_bytes();
        data[4] = ml[0];
        data[5] = ml[1];
        data[6] = ml[2];
        data[7] = ml[3];
        let ls = line_spacing.to_le_bytes();
        data[20] = ls[0];
        data[21] = ls[1];
        data[22] = ls[2];
        data[23] = ls[3];
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

    // -----------------------------------------------------------------------
    // decompress_stream
    // -----------------------------------------------------------------------

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
}
