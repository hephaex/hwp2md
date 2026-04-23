//! Lenient (fallback) HWP reader for damaged or truncated files.
//!
//! When the normal CFB-based parser fails, this module attempts a raw binary
//! scan of the file bytes looking for HWP record headers.  It only extracts
//! plain text from `HWPTAG_PARA_TEXT` records; tables, images, controls, and
//! other structures are silently skipped.
//!
//! The produced [`ir::Document`] carries `metadata.title = Some("(recovered)")`
//! so callers can distinguish partial recoveries from fully-parsed documents.
//!
//! # Limitations
//!
//! - Record detection is heuristic; garbage bytes between valid records are
//!   ignored via a forward scan.
//! - Compressed section streams embedded inside a valid CFB container will not
//!   be decompressed here.  This path is intended for files where CFB structure
//!   itself is damaged.
//! - No decompression is attempted; the scan targets uncompressed data only.

use std::path::Path;

use crate::error::Hwp2MdError;
use crate::hwp::reader::extract_paragraph_text;
use crate::hwp::record::{HWPTAG_BEGIN, HWPTAG_PARA_TEXT};
use crate::ir;

/// Upper bound on the tag IDs we consider plausible.  HWP 5.x defines tags up
/// to roughly `HWPTAG_BEGIN + 100`.  We add a generous margin to future-proof.
const HWPTAG_MAX_PLAUSIBLE: u16 = HWPTAG_BEGIN + 200;

/// Maximum record data size accepted during lenient scanning (4 MiB per record).
/// Large values almost certainly indicate misaligned reads.
const LENIENT_MAX_RECORD_BYTES: usize = 4 * 1024 * 1024;

/// Minimum bytes before the scan start offset.  The HWP file signature and CFB
/// header occupy the first 512 bytes; skipping them reduces false positives.
const SCAN_SKIP_BYTES: usize = 512;

/// Attempt to recover text from a damaged HWP file via raw binary scanning.
///
/// # Errors
///
/// Returns [`Hwp2MdError::Io`] when the file cannot be read.
/// Never returns an error for malformed/truncated content — such bytes are
/// silently skipped and a (possibly empty) document is returned.
pub(crate) fn try_lenient_read(path: &Path) -> Result<ir::Document, Hwp2MdError> {
    let data = std::fs::read(path)?;
    let records = scan_records(&data);

    let mut paragraphs: Vec<String> = Vec::new();
    for (tag_id, payload) in &records {
        if *tag_id == HWPTAG_PARA_TEXT {
            let text = extract_paragraph_text(payload);
            if !text.is_empty() {
                paragraphs.push(text);
            }
        }
    }

    let mut doc = ir::Document::new();
    doc.metadata.title = Some("(recovered)".into());

    if !paragraphs.is_empty() {
        let blocks: Vec<ir::Block> = paragraphs
            .into_iter()
            .map(|text| ir::Block::Paragraph {
                inlines: vec![ir::Inline::plain(text)],
            })
            .collect();
        doc.sections.push(ir::Section { blocks });
    }

    Ok(doc)
}

/// Scan raw `data` bytes for plausible HWP record headers and return a list of
/// `(tag_id, payload)` pairs.
///
/// The algorithm:
/// 1. Start at [`SCAN_SKIP_BYTES`] (skip CFB/signature header region).
/// 2. At each position, read a 4-byte little-endian word as a candidate header.
/// 3. Decode `tag_id` (bits 0-9), `level` (bits 10-19), `size_field` (bits 20-31).
/// 4. Accept the record only when `tag_id` is in `[HWPTAG_BEGIN, HWPTAG_MAX_PLAUSIBLE]`
///    and the claimed payload fits within the remaining bytes.
/// 5. On acceptance, advance by `4 + size`; on rejection, advance by 1 byte.
pub(crate) fn scan_records(data: &[u8]) -> Vec<(u16, Vec<u8>)> {
    let mut results: Vec<(u16, Vec<u8>)> = Vec::new();
    let len = data.len();

    if len < SCAN_SKIP_BYTES + 4 {
        return results;
    }

    let mut pos = SCAN_SKIP_BYTES;
    while pos + 4 <= len {
        let header = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);

        let tag_id = (header & 0x3FF) as u16;
        let size_field = ((header >> 20) & 0xFFF) as usize;

        // Filter on tag_id plausibility first — cheapest check.
        if !(HWPTAG_BEGIN..=HWPTAG_MAX_PLAUSIBLE).contains(&tag_id) {
            pos += 1;
            continue;
        }

        // Determine actual payload size.
        let (header_len, size) = if size_field == 0xFFF {
            // Extended size: next 4 bytes hold the real size.
            if pos + 8 > len {
                pos += 1;
                continue;
            }
            let ext =
                u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]])
                    as usize;
            (8, ext)
        } else {
            (4, size_field)
        };

        // Guard against absurdly large or out-of-bounds records.
        if size > LENIENT_MAX_RECORD_BYTES || pos + header_len + size > len {
            pos += 1;
            continue;
        }

        let payload = data[pos + header_len..pos + header_len + size].to_vec();
        results.push((tag_id, payload));
        pos += header_len + size;
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hwp::record::{HWPTAG_PARA_HEADER, HWPTAG_PARA_TEXT};

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Build a 4-byte record header word (tag_id, level, size_field).
    fn make_header(tag_id: u16, level: u16, size: u32) -> [u8; 4] {
        let size_field = if size < 0xFFF { size } else { 0xFFF };
        let word: u32 =
            (tag_id as u32 & 0x3FF) | ((level as u32 & 0x3FF) << 10) | (size_field << 20);
        word.to_le_bytes()
    }

    /// Build an extended-size record (size_field = 0xFFF + 4-byte actual size).
    fn make_ext_header(tag_id: u16, size: u32) -> Vec<u8> {
        let word: u32 = (tag_id as u32 & 0x3FF) | (0xFFF << 20);
        let mut v = word.to_le_bytes().to_vec();
        v.extend_from_slice(&size.to_le_bytes());
        v
    }

    /// Encode a slice of u16 values as little-endian bytes.
    fn encode_utf16le(s: &str) -> Vec<u8> {
        let units: Vec<u16> = s.encode_utf16().collect();
        let mut buf = Vec::with_capacity(units.len() * 2);
        for u in units {
            buf.extend_from_slice(&u.to_le_bytes());
        }
        buf
    }

    /// Build a minimal byte buffer with SCAN_SKIP_BYTES of padding followed by
    /// the provided records.
    fn padded(records: &[u8]) -> Vec<u8> {
        let mut buf = vec![0u8; SCAN_SKIP_BYTES];
        buf.extend_from_slice(records);
        buf
    }

    // -----------------------------------------------------------------------
    // scan_records tests
    // -----------------------------------------------------------------------

    #[test]
    fn scan_records_empty_data_returns_empty() {
        assert!(scan_records(&[]).is_empty());
    }

    #[test]
    fn scan_records_data_shorter_than_skip_plus_header_returns_empty() {
        let data = vec![0u8; SCAN_SKIP_BYTES + 2]; // only 2 bytes past skip
        assert!(scan_records(&data).is_empty());
    }

    #[test]
    fn scan_records_single_para_text_record() {
        let text_bytes = encode_utf16le("Hello");
        let size = text_bytes.len() as u32;
        let mut rec = make_header(HWPTAG_PARA_TEXT, 0, size).to_vec();
        rec.extend_from_slice(&text_bytes);
        let data = padded(&rec);

        let results = scan_records(&data);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, HWPTAG_PARA_TEXT);
        assert_eq!(results[0].1, text_bytes);
    }

    #[test]
    fn scan_records_two_consecutive_records() {
        let text_bytes = encode_utf16le("테스트");
        let size = text_bytes.len() as u32;

        let mut recs = make_header(HWPTAG_PARA_HEADER, 0, 0).to_vec();
        recs.extend_from_slice(&make_header(HWPTAG_PARA_TEXT, 1, size));
        recs.extend_from_slice(&text_bytes);
        let data = padded(&recs);

        let results = scan_records(&data);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, HWPTAG_PARA_HEADER);
        assert!(results[0].1.is_empty());
        assert_eq!(results[1].0, HWPTAG_PARA_TEXT);
        assert_eq!(results[1].1, text_bytes);
    }

    #[test]
    fn scan_records_garbage_bytes_skipped() {
        // Insert garbage before a valid record — should still find the record.
        let text_bytes = encode_utf16le("찾기");
        let size = text_bytes.len() as u32;
        let mut rec = make_header(HWPTAG_PARA_TEXT, 0, size).to_vec();
        rec.extend_from_slice(&text_bytes);

        let mut data = vec![0u8; SCAN_SKIP_BYTES];
        data.extend_from_slice(&[0xFF, 0xFF, 0x00, 0x00]); // garbage
        data.extend_from_slice(&rec);

        let results = scan_records(&data);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, HWPTAG_PARA_TEXT);
    }

    #[test]
    fn scan_records_out_of_range_tag_id_rejected() {
        // tag_id = 0x00 is below HWPTAG_BEGIN — must not appear in results.
        let word: u32 = 0x0000_0000; // tag_id=0, level=0, size=0
        let data = padded(&word.to_le_bytes());
        assert!(scan_records(&data).is_empty());
    }

    #[test]
    fn scan_records_extended_size_record() {
        let text_bytes = encode_utf16le("拡張サイズ");
        let size = text_bytes.len() as u32;
        let mut rec = make_ext_header(HWPTAG_PARA_TEXT, size);
        rec.extend_from_slice(&text_bytes);
        let data = padded(&rec);

        let results = scan_records(&data);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, HWPTAG_PARA_TEXT);
        assert_eq!(results[0].1, text_bytes);
    }

    #[test]
    fn scan_records_oversized_record_skipped() {
        // A header claiming more bytes than LENIENT_MAX_RECORD_BYTES.
        let tag_id = HWPTAG_PARA_TEXT;
        let huge: u32 = LENIENT_MAX_RECORD_BYTES as u32 + 1;
        let mut rec = make_ext_header(tag_id, huge);
        // Don't actually append that many bytes — simulate a truncated file.
        rec.extend_from_slice(&[0u8; 8]); // tiny actual payload
        let data = padded(&rec);

        // The oversized header must be skipped; we may still find nothing or
        // pick up something from scanning forward — as long as there is no panic.
        let _results = scan_records(&data);
    }

    #[test]
    fn scan_records_truncated_payload_skipped() {
        // Header claims 10-byte payload but only 4 bytes follow.
        let mut rec = make_header(HWPTAG_PARA_TEXT, 0, 10).to_vec();
        rec.extend_from_slice(&[0u8; 4]); // only 4 bytes, not 10
        let data = padded(&rec);

        // Must not panic; truncated record should be skipped.
        let _results = scan_records(&data);
    }

    // -----------------------------------------------------------------------
    // try_lenient_read integration tests
    // -----------------------------------------------------------------------

    #[test]
    fn try_lenient_read_nonexistent_file_returns_io_error() {
        let result = try_lenient_read(Path::new("/tmp/this_does_not_exist_hwp2md_test.hwp"));
        assert!(result.is_err());
    }

    #[test]
    fn try_lenient_read_empty_file_returns_empty_recovered_doc() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().expect("tempfile");
        // Write nothing — empty file.
        tmp.flush().unwrap();

        let doc = try_lenient_read(tmp.path()).expect("empty file should not fail");
        assert_eq!(doc.metadata.title.as_deref(), Some("(recovered)"));
        assert!(doc.sections.is_empty());
    }

    #[test]
    fn try_lenient_read_tiny_garbage_file_returns_recovered_doc_no_panic() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().expect("tempfile");
        tmp.write_all(&[0x00u8; 128]).unwrap();

        let doc = try_lenient_read(tmp.path()).expect("should not error");
        assert_eq!(doc.metadata.title.as_deref(), Some("(recovered)"));
    }

    #[test]
    fn try_lenient_read_extracts_para_text_from_minimal_byte_sequence() {
        use std::io::Write;

        let text_bytes = encode_utf16le("복구된 텍스트");
        let size = text_bytes.len() as u32;
        let mut rec = make_header(HWPTAG_PARA_TEXT, 0, size).to_vec();
        rec.extend_from_slice(&text_bytes);

        let mut file_bytes = vec![0u8; SCAN_SKIP_BYTES];
        file_bytes.extend_from_slice(&rec);

        let mut tmp = tempfile::NamedTempFile::new().expect("tempfile");
        tmp.write_all(&file_bytes).unwrap();

        let doc = try_lenient_read(tmp.path()).expect("should recover text");
        assert_eq!(doc.metadata.title.as_deref(), Some("(recovered)"));
        assert_eq!(doc.sections.len(), 1);
        assert_eq!(doc.sections[0].blocks.len(), 1);
        if let ir::Block::Paragraph { inlines } = &doc.sections[0].blocks[0] {
            assert_eq!(inlines[0].text, "복구된 텍스트");
        } else {
            panic!("expected Paragraph block");
        }
    }

    #[test]
    fn try_lenient_read_skips_non_para_text_records() {
        use std::io::Write;

        // PARA_HEADER (no text) followed by PARA_TEXT with content.
        let text_bytes = encode_utf16le("only this");
        let size = text_bytes.len() as u32;
        let mut recs = make_header(HWPTAG_PARA_HEADER, 0, 0).to_vec();
        recs.extend_from_slice(&make_header(HWPTAG_PARA_TEXT, 1, size));
        recs.extend_from_slice(&text_bytes);

        let mut file_bytes = vec![0u8; SCAN_SKIP_BYTES];
        file_bytes.extend_from_slice(&recs);

        let mut tmp = tempfile::NamedTempFile::new().expect("tempfile");
        tmp.write_all(&file_bytes).unwrap();

        let doc = try_lenient_read(tmp.path()).expect("should parse");
        assert_eq!(doc.sections[0].blocks.len(), 1);
        if let ir::Block::Paragraph { inlines } = &doc.sections[0].blocks[0] {
            assert_eq!(inlines[0].text, "only this");
        } else {
            panic!("expected single Paragraph block");
        }
    }

    #[test]
    fn try_lenient_read_multiple_paragraphs() {
        use std::io::Write;

        let lines = ["첫 번째", "두 번째", "세 번째"];
        let mut file_bytes = vec![0u8; SCAN_SKIP_BYTES];
        for line in lines {
            let text_bytes = encode_utf16le(line);
            let size = text_bytes.len() as u32;
            file_bytes.extend_from_slice(&make_header(HWPTAG_PARA_TEXT, 0, size));
            file_bytes.extend_from_slice(&text_bytes);
        }

        let mut tmp = tempfile::NamedTempFile::new().expect("tempfile");
        tmp.write_all(&file_bytes).unwrap();

        let doc = try_lenient_read(tmp.path()).expect("should parse multiple");
        assert_eq!(doc.sections[0].blocks.len(), 3);
    }

    #[test]
    fn try_lenient_read_empty_para_text_not_added_as_block() {
        use std::io::Write;

        // PARA_TEXT record with zero payload → empty string → not included.
        let mut file_bytes = vec![0u8; SCAN_SKIP_BYTES];
        file_bytes.extend_from_slice(&make_header(HWPTAG_PARA_TEXT, 0, 0));

        let mut tmp = tempfile::NamedTempFile::new().expect("tempfile");
        tmp.write_all(&file_bytes).unwrap();

        let doc = try_lenient_read(tmp.path()).expect("should parse");
        assert!(
            doc.sections.is_empty(),
            "empty PARA_TEXT must not add a section"
        );
    }
}
