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
    let data = encode_u16s(&[b'A' as u16, 0x000D, b'B' as u16]);
    assert_eq!(extract_paragraph_text(&data), "AB");
}

#[test]
fn extract_paragraph_text_control_chars_skip_14_bytes() {
    let mut units: Vec<u16> = vec![0x0003];
    units.extend_from_slice(&[0u16; 7]);
    units.push(b'X' as u16);
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
    let attr1 = ((head_type as u32) << 24) | ((para_level as u32) << 26);
    data[0..4].copy_from_slice(&attr1.to_le_bytes());
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

#[test]
fn size_limit_constants_are_reasonable() {
    // Both limits must be strictly positive and at most 1 GiB so that
    // allocation in tests and production code stays bounded.
    assert!(MAX_DECOMPRESSED > 0);
    assert!(MAX_DECOMPRESSED <= 1024 * 1024 * 1024);
    assert!(MAX_CFB_STREAM > 0);
    assert!(MAX_CFB_STREAM <= 1024 * 1024 * 1024);
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
