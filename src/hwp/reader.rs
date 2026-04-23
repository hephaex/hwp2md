use crate::error::Hwp2MdError;
use crate::hwp::control::extract_paragraphs_from_range;
use crate::hwp::convert::hwp_to_ir;
use crate::hwp::crypto::{
    decrypt_seed, decrypt_viewtext, extract_aes_key, HWPTAG_DISTRIBUTE_DOC_DATA,
};
use crate::hwp::lenient;
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

/// Read an HWP file, returning an IR [`ir::Document`].
///
/// If normal CFB-based parsing fails (e.g. the file is corrupted or truncated),
/// a lenient raw-scan fallback is attempted automatically.  The fallback
/// document carries `metadata.title = Some("(recovered)")` to signal partial
/// content.
pub fn read_hwp(path: &Path) -> Result<ir::Document, Hwp2MdError> {
    match parse_hwp_file(path) {
        Ok(hwp_doc) => Ok(hwp_to_ir(&hwp_doc)),
        Err(e) => {
            tracing::warn!("Normal HWP parse failed ({e}), attempting lenient recovery");
            lenient::try_lenient_read(path)
        }
    }
}

fn parse_hwp_file(path: &Path) -> Result<HwpDocument, Hwp2MdError> {
    let file = std::fs::File::open(path)?;
    let mut cfb = cfb::CompoundFile::open(file)
        .map_err(|e| Hwp2MdError::HwpParse(format!("CFB open: {e}")))?;

    let header = read_file_header(&mut cfb)?;

    if header.encrypted || header.has_drm {
        return Err(Hwp2MdError::HwpParse(
            "HWP file is encrypted or DRM-protected".into(),
        ));
    }

    let doc_info = read_doc_info(&mut cfb, header.compressed)?;

    let section_count = if doc_info.doc_properties.section_count > 0 {
        doc_info.doc_properties.section_count as usize
    } else if header.distributed {
        count_viewtext_sections(&mut cfb)
    } else {
        count_sections(&mut cfb)
    };

    let mut sections = Vec::new();

    if header.distributed {
        // Distribution documents store body text in encrypted ViewText streams.
        // Locate the DISTRIBUTE_DOC_DATA seed in DocInfo and derive the AES key.
        let aes_key = match doc_info.distribute_seed.as_ref() {
            Some(seed_data) => {
                let decrypted = decrypt_seed(seed_data).map_err(|e| {
                    Hwp2MdError::HwpParse(format!("distribute seed decryption failed: {e}"))
                })?;
                extract_aes_key(&decrypted)
                    .map_err(|e| Hwp2MdError::HwpParse(format!("AES key extraction failed: {e}")))?
            }
            None => {
                return Err(Hwp2MdError::HwpParse(
                    "distributed HWP has no DISTRIBUTE_DOC_DATA record".into(),
                ));
            }
        };

        for i in 0..section_count {
            let section_path = format!("ViewText/Section{i}");
            match read_distributed_section(&mut cfb, &section_path, &aes_key) {
                Ok(section) => sections.push(section),
                Err(e) => {
                    tracing::warn!("Failed to read distributed {section_path}: {e}");
                }
            }
        }
    } else {
        for i in 0..section_count {
            let section_path = format!("BodyText/Section{i}");
            match read_section_stream(&mut cfb, &section_path, header.compressed) {
                Ok(section) => sections.push(section),
                Err(e) => {
                    tracing::warn!("Failed to read {section_path}: {e}");
                }
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

/// Maximum size for a raw CFB stream read from untrusted HWP input (256 MB).
pub(crate) const MAX_CFB_STREAM: u64 = 256 * 1024 * 1024;

pub(crate) fn decompress_stream(data: &[u8]) -> Result<Vec<u8>, Hwp2MdError> {
    decompress_stream_limited(data, MAX_DECOMPRESSED)
}

/// Decompress `data` (deflate with zlib fallback), rejecting output that
/// exceeds `limit` bytes.
///
/// Reading `limit + 1` bytes via [`Read::take`] lets us distinguish a
/// legitimately-sized stream (output < limit) from one that was cut off at
/// the limit (output == limit + 1, meaning more data remains).
fn decompress_stream_limited(data: &[u8], limit: u64) -> Result<Vec<u8>, Hwp2MdError> {
    let mut out = Vec::new();
    let decoder = DeflateDecoder::new(data);
    let deflate_err = match decoder.take(limit + 1).read_to_end(&mut out) {
        Ok(_) => {
            if out.len() as u64 > limit {
                return Err(Hwp2MdError::DecompressionBomb(limit));
            }
            return Ok(out);
        }
        Err(e) => {
            tracing::trace!("Deflate failed, trying zlib: {e}");
            e
        }
    };

    out.clear();
    let decoder = flate2::read::ZlibDecoder::new(data);
    match decoder.take(limit + 1).read_to_end(&mut out) {
        Ok(_) => {
            if out.len() as u64 > limit {
                return Err(Hwp2MdError::DecompressionBomb(limit));
            }
            Ok(out)
        }
        Err(zlib_err) => Err(Hwp2MdError::Decompress(format!(
            "deflate failed: {deflate_err}; zlib also failed: {zlib_err}"
        ))),
    }
}

fn read_stream_bytes(
    cfb: &mut cfb::CompoundFile<std::fs::File>,
    path: &str,
    compressed: bool,
) -> Result<Vec<u8>, Hwp2MdError> {
    let stream = cfb
        .open_stream(path)
        .map_err(|e| Hwp2MdError::HwpParse(format!("open stream '{path}': {e}")))?;

    let mut raw = Vec::new();
    stream.take(MAX_CFB_STREAM).read_to_end(&mut raw)?;

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
            HWPTAG_DOCUMENT_PROPERTIES if rec.data.len() >= 26 => {
                doc_info.doc_properties.section_count =
                    u16::from_le_bytes([rec.data[0], rec.data[1]]);
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
            HWPTAG_DISTRIBUTE_DOC_DATA if rec.data.len() >= 4 => {
                doc_info.distribute_seed = Some(rec.data[4..].to_vec());
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

fn count_viewtext_sections(cfb: &mut cfb::CompoundFile<std::fs::File>) -> usize {
    let mut count = 0;
    loop {
        let path = format!("ViewText/Section{count}");
        if cfb.open_stream(&path).is_err() {
            break;
        }
        count += 1;
    }
    count
}

/// Read and decrypt a `ViewText/Section{N}` stream from a distribution document.
///
/// The stream is stored as AES-128 ECB encrypted data without any compression
/// header.  After decryption the bytes are identical in format to a normal
/// `BodyText/Section{N}` stream and are deflate-decompressed before parsing.
fn read_distributed_section(
    cfb: &mut cfb::CompoundFile<std::fs::File>,
    path: &str,
    aes_key: &[u8; 16],
) -> Result<HwpSection, Hwp2MdError> {
    let stream = cfb
        .open_stream(path)
        .map_err(|e| Hwp2MdError::HwpParse(format!("open distributed stream '{path}': {e}")))?;

    let mut raw = Vec::new();
    stream.take(MAX_CFB_STREAM).read_to_end(&mut raw)?;

    // AES-128 ECB decrypt.
    let decrypted = decrypt_viewtext(&raw, aes_key)
        .map_err(|e| Hwp2MdError::HwpParse(format!("AES decrypt '{path}': {e}")))?;

    // Deflate-decompress (same as normal BodyText).
    let decompressed = decompress_stream(&decrypted)?;

    let mut cursor = Cursor::new(&decompressed);
    let records = parse_records(&mut cursor)?;
    Ok(parse_section_from_records(&records))
}

fn parse_section_from_records(records: &[Record]) -> HwpSection {
    let paragraphs = extract_paragraphs_from_range(records, 0, records.len());
    HwpSection { paragraphs }
}

pub(crate) fn read_section_stream(
    cfb: &mut cfb::CompoundFile<std::fs::File>,
    path: &str,
    compressed: bool,
) -> Result<HwpSection, Hwp2MdError> {
    let data = read_stream_bytes(cfb, path, compressed)?;
    let mut cursor = Cursor::new(&data);
    let records = parse_records(&mut cursor)?;
    Ok(parse_section_from_records(&records))
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
            if let Ok(stream) = cfb.open_stream(&path) {
                let mut data = Vec::new();
                stream.take(MAX_CFB_STREAM).read_to_end(&mut data)?;
                bin_data.insert(id, data);
            }
        }
    } else {
        // Fallback: sequential probe with a conservative upper limit.
        for i in 1..=100u16 {
            let path = format!("BinData/BIN{:04X}", i);
            match cfb.open_stream(&path) {
                Ok(stream) => {
                    let mut data = Vec::new();
                    stream.take(MAX_CFB_STREAM).read_to_end(&mut data)?;
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
#[path = "reader_tests.rs"]
mod tests;
