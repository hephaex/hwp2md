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
fn extract_paragraphs_from_range(
    records: &[Record],
    start: usize,
    end: usize,
) -> Vec<HwpParagraph> {
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

/// Parse a `CTRL_TABLE` subtree starting at `ctrl_idx` in `records`.
///
/// Returns `(row_count, col_count, cells)`.
fn parse_table_ctrl(records: &[Record], ctrl_idx: usize) -> (u16, u16, Vec<HwpTableCell>) {
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
                    if v == 0 {
                        1
                    } else {
                        v
                    }
                } else {
                    1
                };
                let row_span = if rec.data.len() >= 10 {
                    let v = u16::from_le_bytes([rec.data[8], rec.data[9]]);
                    if v == 0 {
                        1
                    } else {
                        v
                    }
                } else {
                    1
                };

                let cell_end = find_children_end(records, idx);
                let paragraphs = extract_paragraphs_from_range(records, idx + 1, cell_end);

                cells.push(HwpTableCell {
                    row,
                    col,
                    row_span,
                    col_span,
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
/// GSOTYPE layout for a picture (GSOType == 0):
///   bytes  0- 3: GSOType kind (u32) — 0 = picture
///   bytes  4- 7: color fill (u32)
///   ...varies by kind...
/// For pictures (kind 0), the bin data ID is at offset 80 (u16) in the GSOTYPE body.
/// In practice this offset varies; we probe for it defensively.
fn parse_gshape_ctrl(records: &[Record], ctrl_idx: usize) -> (u16, u32, u32) {
    let rec = &records[ctrl_idx];

    // Extract width and height from the CTRL_HEADER data itself.
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

    // Search child records for HWPTAG_GSOTYPE which carries the bin data reference.
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
fn find_gsotype_bin_id(records: &[Record], start: usize, end: usize) -> u16 {
    for rec in records.iter().skip(start).take(end.saturating_sub(start)) {
        if rec.tag_id == HWPTAG_GSOTYPE {
            // GSOTYPE record for a picture:
            //   bytes 0-3: kind (0 = picture)
            //   bytes 4-7: fill color (u32)
            //   ...
            // The embedded BinData ID for pictures is at byte offset 2 of a
            // child "picSub" structure.  Empirically it sits at offset 0 of
            // a sub-record (tag HWPTAG_BEGIN+68), but we also check at offset
            // 2 and 4 within this record itself when there are no children.
            if rec.data.len() >= 4 {
                let kind = u32::from_le_bytes([rec.data[0], rec.data[1], rec.data[2], rec.data[3]]);
                if kind == 0 && rec.data.len() >= 6 {
                    // Candidate at offset 4 (u16).
                    let candidate = u16::from_le_bytes([rec.data[4], rec.data[5]]);
                    if candidate > 0 {
                        return candidate;
                    }
                }
            }
        }
    }
    0
}

/// Parse the `CTRL_HEADER` at `ctrl_idx` and return the corresponding
/// `HwpControl` variant, or `None` if the control type is unknown/malformed.
fn parse_ctrl_header_at(records: &[Record], ctrl_idx: usize) -> Option<HwpControl> {
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
        CTRL_PAGE_BREAK => Some(HwpControl::PageBreak),
        CTRL_COL_BREAK => Some(HwpControl::ColumnBreak),
        _ => {
            tracing::debug!("CTRL_HEADER at index {ctrl_idx}: unhandled ctrl_id=0x{ctrl_id:08X}");
            None
        }
    }
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

/// Property IDs for the OLE2 SummaryInformation stream.
const PROP_TITLE: u32 = 0x02;
const PROP_SUBJECT: u32 = 0x03;
const PROP_AUTHOR: u32 = 0x04;
const PROP_KEYWORDS: u32 = 0x06;

/// VT_LPSTR type tag in a property set.
const VT_LPSTR: u32 = 0x1E;

/// Read the OLE2 `\x05SummaryInformation` stream and extract title, author,
/// subject, and keywords.  Returns `(title, author, subject, keywords)`.
///
/// Gracefully returns all-`None`/empty on any parse failure so callers are
/// never disrupted.
fn read_summary_info(
    cfb: &mut cfb::CompoundFile<std::fs::File>,
) -> (Option<String>, Option<String>, Option<String>, Vec<String>) {
    let empty = || (None, None, None, Vec::new());

    // The stream name begins with the literal byte 0x05.
    let stream_name = "\x05SummaryInformation";
    let mut raw = Vec::new();
    match cfb.open_stream(stream_name) {
        Ok(mut s) => {
            if s.read_to_end(&mut raw).is_err() {
                tracing::debug!("SummaryInformation: read failed");
                return empty();
            }
        }
        Err(e) => {
            tracing::debug!("SummaryInformation stream not found: {e}");
            return empty();
        }
    }

    // Minimum header: 28 bytes (byte-order + version + OS + reserved) +
    // 20 bytes for the first section entry (16-byte FMTID + 4-byte offset).
    if raw.len() < 48 {
        tracing::debug!("SummaryInformation: stream too short ({} bytes)", raw.len());
        return empty();
    }

    // Validate little-endian byte-order mark (bytes 0-1 = 0xFE 0xFF).
    if raw[0] != 0xFE || raw[1] != 0xFF {
        tracing::debug!("SummaryInformation: unexpected byte-order mark");
        return empty();
    }

    // Section offset is at bytes 44-47 (after 28-byte header + 16-byte FMTID).
    let sec_offset = u32::from_le_bytes([raw[44], raw[45], raw[46], raw[47]]) as usize;
    if sec_offset + 8 > raw.len() {
        tracing::debug!("SummaryInformation: section offset out of range");
        return empty();
    }

    // Section header: byte-count (4) then property-count (4).
    let prop_count = u32::from_le_bytes([
        raw[sec_offset + 4],
        raw[sec_offset + 5],
        raw[sec_offset + 6],
        raw[sec_offset + 7],
    ]) as usize;

    // Property directory starts at sec_offset + 8.
    // Each entry is 8 bytes: property_id (u32) + offset_from_sec_start (u32).
    let dir_start = sec_offset + 8;
    if dir_start + prop_count * 8 > raw.len() {
        tracing::debug!("SummaryInformation: property directory truncated");
        return empty();
    }

    let read_lpstr = |prop_offset: usize| -> Option<String> {
        // prop_offset is relative to sec_offset.
        let abs = sec_offset + prop_offset;
        if abs + 8 > raw.len() {
            return None;
        }
        let type_id = u32::from_le_bytes([raw[abs], raw[abs + 1], raw[abs + 2], raw[abs + 3]]);
        if type_id != VT_LPSTR {
            return None;
        }
        let size =
            u32::from_le_bytes([raw[abs + 4], raw[abs + 5], raw[abs + 6], raw[abs + 7]]) as usize;
        let data_start = abs + 8;
        if data_start + size > raw.len() {
            return None;
        }
        // Trim trailing NUL bytes, then decode as UTF-8 (lossy).
        let bytes: &[u8] = raw[data_start..data_start + size]
            .split(|&b| b == 0)
            .next()
            .unwrap_or(&[]);
        if bytes.is_empty() {
            return None;
        }
        Some(String::from_utf8_lossy(bytes).into_owned())
    };

    let mut title = None;
    let mut author = None;
    let mut subject = None;
    let mut keywords: Vec<String> = Vec::new();

    for i in 0..prop_count {
        let entry = dir_start + i * 8;
        let prop_id =
            u32::from_le_bytes([raw[entry], raw[entry + 1], raw[entry + 2], raw[entry + 3]]);
        let prop_offset = u32::from_le_bytes([
            raw[entry + 4],
            raw[entry + 5],
            raw[entry + 6],
            raw[entry + 7],
        ]) as usize;

        match prop_id {
            PROP_TITLE => title = read_lpstr(prop_offset),
            PROP_AUTHOR => author = read_lpstr(prop_offset),
            PROP_SUBJECT => subject = read_lpstr(prop_offset),
            PROP_KEYWORDS => {
                if let Some(kw) = read_lpstr(prop_offset) {
                    keywords = kw
                        .split([',', ';', ' '])
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned)
                        .collect();
                }
            }
            _ => {}
        }
    }

    tracing::debug!(
        "SummaryInformation parsed: title={:?} author={:?} subject={:?} keywords={:?}",
        title,
        author,
        subject,
        keywords
    );

    (title, author, subject, keywords)
}

fn hwp_to_ir(hwp: &HwpDocument) -> ir::Document {
    let mut doc = ir::Document::new();

    doc.metadata.title = hwp.summary_title.clone();
    doc.metadata.author = hwp.summary_author.clone();
    doc.metadata.subject = hwp.summary_subject.clone();
    doc.metadata.keywords = hwp.summary_keywords.clone();

    for section in &hwp.sections {
        let mut ir_section = ir::Section { blocks: Vec::new() };

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
    let mut blocks: Vec<ir::Block> = Vec::new();

    // Emit IR blocks for each embedded control first.  Controls are independent
    // of the paragraph text (a paragraph may contain *only* a table, for example).
    for ctrl in &para.controls {
        if let Some(block) = control_to_block(ctrl, doc_info) {
            blocks.push(block);
        }
    }

    // Emit the text content of the paragraph, if any.
    let text = para.text.trim();
    if !text.is_empty() {
        let heading_level = detect_heading_level(para, doc_info);
        let inlines = build_inlines(para, doc_info);
        if !inlines.is_empty() {
            if let Some(level) = heading_level {
                blocks.push(ir::Block::Heading { level, inlines });
            } else {
                blocks.push(ir::Block::Paragraph { inlines });
            }
        }
    }

    blocks
}

/// Convert a single `HwpControl` to an `ir::Block`.  Returns `None` for
/// controls that have no direct IR representation (e.g. page-break hints).
fn control_to_block(ctrl: &HwpControl, doc_info: &DocInfo) -> Option<ir::Block> {
    match ctrl {
        HwpControl::Table {
            row_count,
            col_count,
            cells,
        } => {
            // Group cells by row index, then sort each row by col index.
            let n_rows = *row_count as usize;
            let n_cols = *col_count as usize;
            let effective_cols = if n_cols > 0 {
                n_cols
            } else {
                cells.iter().map(|c| c.col as usize + 1).max().unwrap_or(1)
            };

            let mut rows: Vec<Vec<&HwpTableCell>> = vec![Vec::new(); n_rows.max(1)];
            for cell in cells {
                let row_idx = cell.row as usize;
                if row_idx < rows.len() {
                    rows[row_idx].push(cell);
                } else {
                    // Gracefully extend for malformed row indices.
                    rows.resize(row_idx + 1, Vec::new());
                    rows[row_idx].push(cell);
                }
            }

            let ir_rows: Vec<ir::TableRow> = rows
                .into_iter()
                .enumerate()
                .map(|(row_idx, row_cells)| {
                    let mut sorted = row_cells;
                    sorted.sort_by_key(|c| c.col);
                    let ir_cells: Vec<ir::TableCell> = sorted
                        .into_iter()
                        .map(|cell| ir::TableCell {
                            blocks: cell
                                .paragraphs
                                .iter()
                                .flat_map(|p| paragraph_to_blocks(p, doc_info))
                                .collect(),
                            colspan: cell.col_span as u32,
                            rowspan: cell.row_span as u32,
                        })
                        .collect();
                    ir::TableRow {
                        cells: ir_cells,
                        is_header: row_idx == 0,
                    }
                })
                .collect();

            Some(ir::Block::Table {
                rows: ir_rows,
                col_count: effective_cols,
            })
        }
        HwpControl::Image { bin_data_id, .. } => {
            let src = format!("image_{bin_data_id}.bin");
            Some(ir::Block::Image {
                src,
                alt: String::new(),
            })
        }
        HwpControl::Equation { script } => {
            // Already captured as Math block in `paragraph_to_blocks` via EQEDIT.
            // But if it surfaces here via controls, emit it.
            Some(ir::Block::Math {
                display: false,
                tex: script.clone(),
            })
        }
        HwpControl::FootnoteEndnote {
            is_endnote,
            paragraphs,
        } => {
            let content: Vec<ir::Block> = paragraphs
                .iter()
                .flat_map(|p| paragraph_to_blocks(p, doc_info))
                .collect();
            let id = if *is_endnote {
                "endnote".to_string()
            } else {
                "footnote".to_string()
            };
            Some(ir::Block::Footnote { id, content })
        }
        HwpControl::Hyperlink { .. }
        | HwpControl::PageBreak
        | HwpControl::SectionBreak
        | HwpControl::ColumnBreak => None,
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

    // -----------------------------------------------------------------------
    // Phase 2: find_children_end
    // -----------------------------------------------------------------------

    /// Build a minimal Record for testing purposes.
    fn make_record(tag_id: u16, level: u16) -> Record {
        Record {
            tag_id,
            level,
            data: Vec::new(),
        }
    }

    /// Build a Record with a specific data payload.
    fn make_record_with_data(tag_id: u16, level: u16, data: Vec<u8>) -> Record {
        Record {
            tag_id,
            level,
            data,
        }
    }

    #[test]
    fn find_children_end_no_children() {
        // Parent at level 0, immediately followed by a sibling at the same level.
        let records = vec![
            make_record(HWPTAG_CTRL_HEADER, 0),
            make_record(HWPTAG_PARA_HEADER, 0), // sibling, not child
        ];
        assert_eq!(find_children_end(&records, 0), 1);
    }

    #[test]
    fn find_children_end_with_children() {
        // Parent at level 1, two children at level 2, then a sibling at level 1.
        let records = vec![
            make_record(HWPTAG_CTRL_HEADER, 1), // index 0 (parent)
            make_record(HWPTAG_TABLE, 2),       // index 1 (child)
            make_record(HWPTAG_LIST_HEADER, 2), // index 2 (child)
            make_record(HWPTAG_PARA_HEADER, 1), // index 3 (sibling — stops here)
        ];
        assert_eq!(find_children_end(&records, 0), 3);
    }

    #[test]
    fn find_children_end_deeply_nested() {
        // Parent at 0, child at 1, grandchild at 2 — all are "descendants" of 0.
        let records = vec![
            make_record(HWPTAG_CTRL_HEADER, 0), // index 0
            make_record(HWPTAG_TABLE, 1),       // index 1 (child)
            make_record(HWPTAG_LIST_HEADER, 2), // index 2 (grandchild)
            make_record(HWPTAG_PARA_HEADER, 3), // index 3 (great-grandchild)
            make_record(HWPTAG_PARA_HEADER, 0), // index 4 (sibling)
        ];
        assert_eq!(find_children_end(&records, 0), 4);
    }

    #[test]
    fn find_children_end_at_last_record() {
        // Parent is the last element — no children, end == len.
        let records = vec![make_record(HWPTAG_CTRL_HEADER, 0)];
        assert_eq!(find_children_end(&records, 0), 1);
    }

    // -----------------------------------------------------------------------
    // Phase 2: parse_table_ctrl
    // -----------------------------------------------------------------------

    /// Build a TABLE record with given row_count and col_count.
    fn make_table_record(level: u16, row_count: u16, col_count: u16) -> Record {
        let mut data = vec![0u8; 8];
        // bytes 0-3: properties (zeroed)
        data[4..6].copy_from_slice(&row_count.to_le_bytes());
        data[6..8].copy_from_slice(&col_count.to_le_bytes());
        make_record_with_data(HWPTAG_TABLE, level, data)
    }

    /// Build a LIST_HEADER record describing one table cell at (row, col).
    fn make_list_header_record(
        level: u16,
        col: u16,
        row: u16,
        col_span: u16,
        row_span: u16,
    ) -> Record {
        let mut data = vec![0u8; 10];
        // bytes 0-1: properties
        data[2..4].copy_from_slice(&col.to_le_bytes());
        data[4..6].copy_from_slice(&row.to_le_bytes());
        data[6..8].copy_from_slice(&col_span.to_le_bytes());
        data[8..10].copy_from_slice(&row_span.to_le_bytes());
        make_record_with_data(HWPTAG_LIST_HEADER, level, data)
    }

    /// Build a CTRL_HEADER record with ctrl_id `tbl `.
    fn make_ctrl_header_table(level: u16) -> Record {
        make_record_with_data(HWPTAG_CTRL_HEADER, level, CTRL_TABLE.to_le_bytes().to_vec())
    }

    #[test]
    fn parse_table_ctrl_dimensions() {
        // Flat record sequence:
        //   [0] CTRL_HEADER(tbl )  level=0
        //   [1] TABLE              level=1  (2 rows × 3 cols)
        let records = vec![make_ctrl_header_table(0), make_table_record(1, 2, 3)];
        let (rows, cols, cells) = parse_table_ctrl(&records, 0);
        assert_eq!(rows, 2);
        assert_eq!(cols, 3);
        assert!(cells.is_empty(), "no LIST_HEADERs so no cells expected");
    }

    #[test]
    fn parse_table_ctrl_with_cells() {
        // One 1×2 table (1 row, 2 cols) with two cells each containing one paragraph.
        //
        // Record sequence (level notation: CH=0, TABLE/LH=1, PH=2, PT=3):
        //   [0] CTRL_HEADER(tbl)   level=0
        //   [1] TABLE(1×2)         level=1
        //   [2] LIST_HEADER(r=0,c=0, span 1×1) level=1
        //   [3] PARA_HEADER        level=2
        //   [4] PARA_TEXT("A")     level=3   ← inside cell (0,0)
        //   [5] LIST_HEADER(r=0,c=1, span 1×1) level=1
        //   [6] PARA_HEADER        level=2
        //   [7] PARA_TEXT("B")     level=3   ← inside cell (0,1)
        let text_a = encode_u16s(&[b'A' as u16]);
        let text_b = encode_u16s(&[b'B' as u16]);

        let mut para_header_data = vec![0u8; 6]; // 6 bytes minimum
        para_header_data[4] = 0; // para_shape_id = 0
        para_header_data[5] = 0;

        let records = vec![
            make_ctrl_header_table(0),              // [0]
            make_table_record(1, 1, 2),             // [1]
            make_list_header_record(1, 0, 0, 1, 1), // [2]
            make_record_with_data(HWPTAG_PARA_HEADER, 2, para_header_data.clone()), // [3]
            make_record_with_data(HWPTAG_PARA_TEXT, 3, text_a), // [4]
            make_list_header_record(1, 1, 0, 1, 1), // [5]
            make_record_with_data(HWPTAG_PARA_HEADER, 2, para_header_data.clone()), // [6]
            make_record_with_data(HWPTAG_PARA_TEXT, 3, text_b), // [7]
        ];

        let (rows, cols, cells) = parse_table_ctrl(&records, 0);
        assert_eq!(rows, 1);
        assert_eq!(cols, 2);
        assert_eq!(cells.len(), 2);

        // Cell (0,0) should have text "A"
        let cell_00 = cells
            .iter()
            .find(|c| c.row == 0 && c.col == 0)
            .expect("cell (0,0)");
        assert_eq!(cell_00.paragraphs.len(), 1);
        assert_eq!(cell_00.paragraphs[0].text, "A");

        // Cell (0,1) should have text "B"
        let cell_01 = cells
            .iter()
            .find(|c| c.row == 0 && c.col == 1)
            .expect("cell (0,1)");
        assert_eq!(cell_01.paragraphs.len(), 1);
        assert_eq!(cell_01.paragraphs[0].text, "B");
    }

    #[test]
    fn parse_table_ctrl_cell_spans() {
        // A cell with row_span=2, col_span=2 must be recorded faithfully.
        let records = vec![
            make_ctrl_header_table(0),
            make_table_record(1, 2, 2),
            make_list_header_record(1, 0, 0, 2, 2), // merged cell spanning 2×2
        ];
        let (_rows, _cols, cells) = parse_table_ctrl(&records, 0);
        assert_eq!(cells.len(), 1);
        assert_eq!(cells[0].col_span, 2);
        assert_eq!(cells[0].row_span, 2);
    }

    #[test]
    fn parse_table_ctrl_malformed_table_record_short_data() {
        // TABLE record with only 4 bytes — row/col count cannot be read.
        // Must not panic; dimensions default to 0.
        let mut short_data = vec![0u8; 4];
        short_data[0..4].copy_from_slice(&0u32.to_le_bytes()); // properties only
        let records = vec![
            make_ctrl_header_table(0),
            make_record_with_data(HWPTAG_TABLE, 1, short_data),
        ];
        let (rows, cols, cells) = parse_table_ctrl(&records, 0);
        assert_eq!(rows, 0);
        assert_eq!(cols, 0);
        assert!(cells.is_empty());
    }

    // -----------------------------------------------------------------------
    // Phase 2: parse_ctrl_header_at
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
        // CTRL_HEADER with fewer than 4 bytes of data → cannot read ctrl_id.
        let records = vec![make_record_with_data(HWPTAG_CTRL_HEADER, 0, vec![0u8; 2])];
        assert!(parse_ctrl_header_at(&records, 0).is_none());
    }

    #[test]
    fn parse_ctrl_header_at_unknown_ctrl_id_returns_none() {
        // An unrecognised ctrl_id must return None gracefully.
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

    // -----------------------------------------------------------------------
    // Phase 2: extract_paragraphs_from_range
    // -----------------------------------------------------------------------

    #[test]
    fn extract_paragraphs_from_range_empty_range() {
        let records: Vec<Record> = Vec::new();
        let paras = extract_paragraphs_from_range(&records, 0, 0);
        assert!(paras.is_empty());
    }

    #[test]
    fn extract_paragraphs_from_range_single_paragraph() {
        let text_data = encode_u16s(&[b'H' as u16, b'i' as u16]);
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
        let text_a = encode_u16s(&[b'A' as u16]);
        let text_b = encode_u16s(&[b'B' as u16]);
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

    // -----------------------------------------------------------------------
    // Phase 2: control_to_block (IR conversion)
    // -----------------------------------------------------------------------

    #[test]
    fn control_to_block_image_produces_image_block() {
        let ctrl = HwpControl::Image {
            bin_data_id: 7,
            width: 100,
            height: 200,
        };
        let doc_info = DocInfo::default();
        let block = control_to_block(&ctrl, &doc_info).expect("Some");
        assert!(
            matches!(block, ir::Block::Image { ref src, .. } if src == "image_7.bin"),
            "expected Image block with src=image_7.bin, got {block:?}"
        );
    }

    #[test]
    fn control_to_block_empty_table_produces_table_block() {
        let ctrl = HwpControl::Table {
            row_count: 2,
            col_count: 3,
            cells: Vec::new(),
        };
        let doc_info = DocInfo::default();
        let block = control_to_block(&ctrl, &doc_info).expect("Some");
        assert!(matches!(block, ir::Block::Table { col_count: 3, .. }));
    }

    #[test]
    fn control_to_block_footnote_produces_footnote_block() {
        let ctrl = HwpControl::FootnoteEndnote {
            is_endnote: false,
            paragraphs: Vec::new(),
        };
        let doc_info = DocInfo::default();
        let block = control_to_block(&ctrl, &doc_info).expect("Some");
        assert!(matches!(block, ir::Block::Footnote { .. }));
    }

    #[test]
    fn control_to_block_page_break_returns_none() {
        let ctrl = HwpControl::PageBreak;
        let doc_info = DocInfo::default();
        assert!(control_to_block(&ctrl, &doc_info).is_none());
    }

    #[test]
    fn control_to_block_table_groups_cells_into_rows() {
        // 2×2 table with 4 cells.
        let make_cell = |row: u16, col: u16, text: &str| HwpTableCell {
            row,
            col,
            row_span: 1,
            col_span: 1,
            paragraphs: vec![HwpParagraph {
                text: text.to_string(),
                char_shape_ids: Vec::new(),
                para_shape_id: 0,
                controls: Vec::new(),
            }],
        };
        let ctrl = HwpControl::Table {
            row_count: 2,
            col_count: 2,
            cells: vec![
                make_cell(0, 0, "r0c0"),
                make_cell(0, 1, "r0c1"),
                make_cell(1, 0, "r1c0"),
                make_cell(1, 1, "r1c1"),
            ],
        };
        let doc_info = DocInfo::default();
        let block = control_to_block(&ctrl, &doc_info).expect("Some");
        if let ir::Block::Table { rows, col_count } = block {
            assert_eq!(col_count, 2);
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0].cells.len(), 2);
            assert_eq!(rows[1].cells.len(), 2);
            // First row is marked as header.
            assert!(rows[0].is_header);
            assert!(!rows[1].is_header);
        } else {
            panic!("Expected Table block");
        }
    }
}
