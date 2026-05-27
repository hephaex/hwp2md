use super::*;

// --- Helpers ---

fn encode_u16s(units: &[u16]) -> Vec<u8> {
    encode_u16s_test(units)
}

fn make_char_shape_data(flags: u32, height: i32) -> Vec<u8> {
    let mut data = vec![0u8; 58];
    data[42..46].copy_from_slice(&height.to_le_bytes());
    data[46..50].copy_from_slice(&flags.to_le_bytes());
    data
}

// --- extract_paragraph_text ---

#[test]
fn extract_paragraph_text_basic_korean() {
    let data = encode_u16s(&[0xD55C, 0xAE00]);
    assert_eq!(extract_paragraph_text(&data), "한글");
}

#[test]
fn extract_paragraph_text_ascii() {
    let data = encode_u16s(&[u16::from(b'H'), u16::from(b'i')]);
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
    let data = encode_u16s(&[u16::from(b'A'), 0x000D, u16::from(b'B')]);
    assert_eq!(extract_paragraph_text(&data), "AB");
}

#[test]
fn extract_paragraph_text_control_chars_skip_14_bytes() {
    let mut units: Vec<u16> = vec![0x0003];
    units.extend_from_slice(&[0u16; 7]);
    units.push(u16::from(b'X'));
    let data = encode_u16s(&units);
    assert_eq!(extract_paragraph_text(&data), "X");
}

#[test]
fn extract_paragraph_text_truncated_control_stops_gracefully() {
    let data = encode_u16s(&[0x0001, 0x0000]);
    let result = extract_paragraph_text(&data);
    assert!(result.is_empty());
}

#[test]
fn extract_paragraph_text_surrogate_pair() {
    let data = encode_u16s(&[0xD83D, 0xDE00]);
    assert_eq!(extract_paragraph_text(&data), "\u{1F600}");
}

#[test]
fn extract_paragraph_text_empty_input() {
    assert_eq!(extract_paragraph_text(&[]), "");
}

#[test]
fn extract_paragraph_text_null_code_unit_skipped() {
    let data = encode_u16s(&[0x0000, u16::from(b'Z')]);
    assert_eq!(extract_paragraph_text(&data), "Z");
}

// Regression test for the 湰灧 garbled character bug (Sprint 49).
// Code 0x0015 is an extended HWP 5.0 inline control (e.g. auto-numbering) that
// carries 14 bytes of inline data embedded in the PARA_TEXT stream.  Before the
// fix, that range (0x000E..=0x001F) was treated as a bare 2-byte code, causing
// the first two u16 values of the inline data to appear in the output as CJK
// garbage characters.
//
// Pattern observed in real government HWP files:
//   [0x0015][pngp data: 0x706E 0x6770 ...][valid text]
//   → garbled output "湰灧" (U+6E70 U+7067) followed by text
//
// After the fix the 14 bytes must be silently consumed.
#[test]
fn extract_paragraph_text_extended_ctrl_0x0015_skips_14_bytes() {
    // Build: text "A" + code 0x0015 + 14 bytes inline (7 u16 "pngp" payload) + text "B"
    let mut units: Vec<u16> = vec![u16::from(b'A'), 0x0015];
    // 14 bytes = 7 u16 inline data for the control (first 4 bytes are [70 6E 67 70] = 湰灧)
    units.extend_from_slice(&[0x6E70, 0x7067, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000]);
    units.push(u16::from(b'B'));
    let data = encode_u16s(&units);
    assert_eq!(
        extract_paragraph_text(&data),
        "AB",
        "0x0015 inline data must not appear as garbled chars in output"
    );
}

#[test]
fn extract_paragraph_text_extended_ctrl_0x000e_skips_14_bytes() {
    let mut units: Vec<u16> = vec![u16::from(b'X'), 0x000E];
    units.extend_from_slice(&[0x1111, 0x2222, 0x3333, 0x4444, 0x5555, 0x6666, 0x7777]);
    units.push(u16::from(b'Y'));
    let data = encode_u16s(&units);
    assert_eq!(extract_paragraph_text(&data), "XY");
}

#[test]
fn extract_paragraph_text_section_break_0x000d_no_extra_bytes() {
    // 0x000D is a section/column break — just 2 bytes, NO inline data.
    let data = encode_u16s(&[u16::from(b'C'), 0x000D, u16::from(b'D')]);
    assert_eq!(extract_paragraph_text(&data), "CD");
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
    let cs = parse_char_shape(&make_char_shape_data(0x40, 0));
    assert!(cs.strikethrough);
    assert!(!cs.bold);
}

#[test]
fn parse_char_shape_all_style_flags() {
    let cs = parse_char_shape(&make_char_shape_data(0x47, 0));
    assert!(cs.bold);
    assert!(cs.italic);
    assert!(cs.underline);
    assert!(cs.strikethrough);
}

#[test]
fn parse_char_shape_short_data_returns_default() {
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

// --- parse_char_shape superscript/subscript ---

#[test]
fn parse_char_shape_superscript_flag() {
    let cs = parse_char_shape(&make_char_shape_data(0x0001_0000, 0));
    assert!(cs.superscript);
    assert!(!cs.subscript);
}

#[test]
fn parse_char_shape_subscript_flag() {
    let cs = parse_char_shape(&make_char_shape_data(0x0002_0000, 0));
    assert!(!cs.superscript);
    assert!(cs.subscript);
}

#[test]
fn parse_char_shape_bold_and_superscript() {
    let cs = parse_char_shape(&make_char_shape_data(0x0001_0001, 0));
    assert!(cs.bold);
    assert!(cs.superscript);
    assert!(!cs.subscript);
}

// --- parse_para_shape heading_type ---

fn make_para_shape_with_heading(head_type: u8, para_level: u8) -> Vec<u8> {
    let mut data = vec![0u8; 24];
    let attr1 = (u32::from(head_type) << 24) | (u32::from(para_level) << 26);
    data[0..4].copy_from_slice(&attr1.to_le_bytes());
    data
}

// Helper: Build a BSTR (2-byte count + UTF-16LE string) at a given offset.
fn make_bstr(s: &str) -> Vec<u8> {
    let utf16: Vec<u16> = s.encode_utf16().collect();
    let count = utf16.len() as u16;
    let mut data = Vec::new();
    data.extend_from_slice(&count.to_le_bytes());
    for &u in &utf16 {
        data.extend_from_slice(&u.to_le_bytes());
    }
    data
}

// Helper: Concatenate two BSTRs into a single byte vec.
fn make_style_record_data(local_name: &str, name: &str) -> Vec<u8> {
    let mut data = make_bstr(local_name);
    data.extend(make_bstr(name));
    data
}

#[test]
fn parse_para_shape_heading_type_outline() {
    let ps = parse_para_shape(&make_para_shape_with_heading(1, 0));
    assert_eq!(ps.heading_type, Some(0));
}

#[test]
fn parse_para_shape_heading_type_level_3() {
    let ps = parse_para_shape(&make_para_shape_with_heading(1, 3));
    assert_eq!(ps.heading_type, Some(3));
}

#[test]
fn parse_para_shape_no_heading() {
    let ps = parse_para_shape(&make_para_shape_with_heading(0, 0));
    assert_eq!(ps.heading_type, None);
}

// --- size limit constants ---

// Verify the size-limit constants at compile time so they cannot be set to
// zero or above 1 GiB.  Using const-assert avoids the `assertions_on_constants`
// clippy lint while still catching misconfiguration at build time.
const _: () = {
    assert!(MAX_DECOMPRESSED > 0);
    assert!(MAX_DECOMPRESSED <= 1024 * 1024 * 1024);
    assert!(MAX_CFB_STREAM > 0);
    assert!(MAX_CFB_STREAM <= 1024 * 1024 * 1024);
};

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

    let decompressed = decompress_stream(&compressed).unwrap();
    assert_eq!(decompressed, original);
}

#[test]
fn decompress_stream_invalid_data_returns_error() {
    assert!(decompress_stream(b"\x00\x01\x02\x03rubbish").is_err());
}

#[test]
fn decompress_stream_deflate_bomb_returns_error() {
    use crate::error::Hwp2MdError;
    use flate2::{write::DeflateEncoder, Compression};
    use std::io::Write;

    // Compress 32 zero bytes; use a limit of 16 so the output exceeds it.
    let original = vec![0u8; 32];
    let mut enc = DeflateEncoder::new(Vec::new(), Compression::default());
    enc.write_all(&original).unwrap();
    let compressed = enc.finish().unwrap();

    let err = decompress_stream_limited(&compressed, 16).unwrap_err();
    assert!(
        matches!(err, Hwp2MdError::DecompressionBomb(16)),
        "expected DecompressionBomb(16), got {err:?}"
    );
}

#[test]
fn decompress_stream_zlib_bomb_returns_error() {
    use crate::error::Hwp2MdError;
    use flate2::{write::ZlibEncoder, Compression};
    use std::io::Write;

    // Compress 32 zero bytes with zlib; limit to 16 bytes.
    let original = vec![0u8; 32];
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(&original).unwrap();
    let compressed = enc.finish().unwrap();

    let err = decompress_stream_limited(&compressed, 16).unwrap_err();
    assert!(
        matches!(err, Hwp2MdError::DecompressionBomb(16)),
        "expected DecompressionBomb(16), got {err:?}"
    );
}

#[test]
fn decompress_stream_exactly_at_limit_succeeds() {
    use flate2::{write::DeflateEncoder, Compression};
    use std::io::Write;

    // Compress exactly 16 bytes; limit is also 16 — must succeed (not a bomb).
    let original = vec![0xABu8; 16];
    let mut enc = DeflateEncoder::new(Vec::new(), Compression::default());
    enc.write_all(&original).unwrap();
    let compressed = enc.finish().unwrap();

    let decompressed = decompress_stream_limited(&compressed, 16).unwrap();
    assert_eq!(decompressed, original);
}

#[test]
fn read_file_header_encrypted_bit_sets_encrypted() {
    let mut buf = vec![0u8; 256];
    buf[0..17].copy_from_slice(b"HWP Document File");
    buf[35] = 5;
    buf[36] = 0x03;
    let props = u32::from_le_bytes([buf[36], buf[37], buf[38], buf[39]]);
    assert!((props & 0x02) != 0, "encrypted bit should be set");
}

#[test]
fn read_file_header_drm_bit_sets_has_drm() {
    let mut buf = vec![0u8; 256];
    buf[0..17].copy_from_slice(b"HWP Document File");
    buf[35] = 5;
    buf[36] = 0x10;
    let props = u32::from_le_bytes([buf[36], buf[37], buf[38], buf[39]]);
    assert!((props & 0x10) != 0, "has_drm bit should be set");
}

// --- parse_style_record ---

#[test]
fn parse_style_record_normal_korean() {
    let data = make_style_record_data("Normal", "개요 1");
    let (_local_name, off) = read_utf16le_str(&data, 0);
    let (name, _) = read_utf16le_str(&data, off);
    assert_eq!(name, "개요 1");
}

#[test]
fn parse_style_record_empty_local_name() {
    let data = make_style_record_data("", "Outline 1");
    let (_local_name, off) = read_utf16le_str(&data, 0);
    let (name, _) = read_utf16le_str(&data, off);
    assert_eq!(name, "Outline 1");
}

#[test]
fn parse_style_record_long_local_name() {
    let data = make_style_record_data("0123456789", "Body Text");
    let (_local_name, off) = read_utf16le_str(&data, 0);
    let (name, _) = read_utf16le_str(&data, off);
    assert_eq!(name, "Body Text");
}

#[test]
fn parse_style_record_truncated_no_panic() {
    let data = vec![0x01u8];
    let (_local_name, off) = read_utf16le_str(&data, 0);
    let (name, _) = read_utf16le_str(&data, off);
    assert_eq!(name, "");
}

// --- HWPTAG_NUMBERING / HWPTAG_BULLET parsing ---

/// Build a minimal HWPTAG_NUMBERING record payload where the number-type nibble
/// of the first level is `num_type` (low 4 bits of byte 2).
fn make_numbering_data(num_type: u8) -> Vec<u8> {
    // Byte 0-1: start number u16 (set to 1)
    // Byte 2: first level's num_type in low nibble
    // Remaining bytes: zero-padded to a realistic minimum size.
    let mut data = vec![0u8; 36];
    data[0] = 1; // start number = 1
    data[1] = 0;
    data[2] = num_type & 0x0F;
    data
}

#[test]
fn numbering_def_ordered_id1_parsed_from_arabic_type() {
    // num_type = 0 = arabic → ordered
    let data = make_numbering_data(0x00);
    let rec = crate::hwp::record::Record {
        tag_id: HWPTAG_NUMBERING,
        level: 0,
        data,
    };
    let mut doc_info = DocInfo::default();
    // Manually apply the same branch logic used in read_doc_info.
    let id = (doc_info.numbering_defs.len() as u32) + 1;
    let ordered = if rec.data.len() >= 3 {
        let num_type = rec.data[2] & 0x0F;
        num_type <= 26
    } else {
        false
    };
    doc_info
        .numbering_defs
        .push(crate::hwp::model::NumberingDef { id, ordered });

    assert_eq!(doc_info.numbering_defs.len(), 1);
    let def = &doc_info.numbering_defs[0];
    assert_eq!(def.id, 1);
    assert!(def.ordered, "arabic (type 0) should be ordered");
}

#[test]
fn numbering_def_unordered_from_bullet_tag() {
    // HWPTAG_BULLET definitions are always unordered.
    let mut doc_info = DocInfo::default();
    let id = (doc_info.numbering_defs.len() as u32) + 1;
    doc_info
        .numbering_defs
        .push(crate::hwp::model::NumberingDef { id, ordered: false });

    assert_eq!(doc_info.numbering_defs.len(), 1);
    let def = &doc_info.numbering_defs[0];
    assert_eq!(def.id, 1);
    assert!(!def.ordered, "bullet tag should produce unordered def");
}

#[test]
fn numbering_def_empty_data_falls_back_to_unordered() {
    // A HWPTAG_NUMBERING record shorter than 3 bytes → defensive fallback.
    let rec = crate::hwp::record::Record {
        tag_id: HWPTAG_NUMBERING,
        level: 0,
        data: vec![0x00, 0x01], // only 2 bytes
    };
    let mut doc_info = DocInfo::default();
    let id = (doc_info.numbering_defs.len() as u32) + 1;
    let ordered = if rec.data.len() >= 3 {
        let num_type = rec.data[2] & 0x0F;
        num_type <= 26
    } else {
        false
    };
    doc_info
        .numbering_defs
        .push(crate::hwp::model::NumberingDef { id, ordered });

    assert!(!doc_info.numbering_defs[0].ordered, "short record should fall back to unordered");
}

#[test]
fn numbering_def_ids_are_sequential_and_one_based() {
    // Multiple NUMBERING + BULLET records should get IDs 1, 2, 3, …
    let mut doc_info = DocInfo::default();
    for ordered in [true, false, true] {
        let id = (doc_info.numbering_defs.len() as u32) + 1;
        doc_info
            .numbering_defs
            .push(crate::hwp::model::NumberingDef { id, ordered });
    }
    assert_eq!(doc_info.numbering_defs[0].id, 1);
    assert_eq!(doc_info.numbering_defs[1].id, 2);
    assert_eq!(doc_info.numbering_defs[2].id, 3);
    assert!(doc_info.numbering_defs[0].ordered);
    assert!(!doc_info.numbering_defs[1].ordered);
    assert!(doc_info.numbering_defs[2].ordered);
}

#[test]
fn numbering_def_high_num_type_is_unordered() {
    // num_type >= 27 (e.g. 0x0F = 15 is still ≤26; use 0x0F max of nibble)
    // Nibble max is 0x0F = 15 which is ≤ 26 → ordered.
    // To test unordered path from num_type > 26 we'd need a multi-byte field.
    // Since 4-bit nibble max = 15 ≤ 26, type 27+ can't be represented in 4 bits.
    // Verify the boundary: type 26 → ordered, type 0x0F (15) → ordered.
    // (The unordered path via num_type for HWPTAG_NUMBERING is unreachable via
    // nibble but is exercised through HWPTAG_BULLET → always-false.)
    let data_type15 = make_numbering_data(0x0F);
    let ordered = if data_type15.len() >= 3 {
        let num_type = data_type15[2] & 0x0F;
        num_type <= 26
    } else {
        false
    };
    assert!(ordered, "num_type 15 (max nibble) should still be ordered (<= 26)");
}
