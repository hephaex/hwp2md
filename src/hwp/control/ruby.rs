use crate::hwp::model::{HwpControl, HwpParagraph};
use crate::hwp::record::{read_utf16le_str, Record};

/// Control characters in HWP PARA_TEXT that occupy a 2-byte code unit plus a
/// 14-byte inline parameter block.  The range 0x0001..=0x001F covers all such
/// extended control code units.
const CTRL_CHAR_LOW: u16 = 0x0001;
const CTRL_CHAR_HIGH: u16 = 0x001F;

/// The specific code unit that marks an extended control (Ruby, Table, ...) in
/// the paragraph text stream.
const CTRL_MARKER: u16 = 0x0003;

/// Byte count of the inline parameter block that immediately follows each
/// extended control code unit in the PARA_TEXT stream.
const CTRL_PARAM_BYTES: usize = 14;

/// Fix up the `base_text` field of every `Ruby` control inside `para`.
///
/// HWP stores the base text -- the characters that the ruby annotation covers --
/// in the PARA_TEXT byte stream, immediately before the `0x0003` control
/// marker.  The `base_text` field is left empty by the initial parser because
/// that information lives outside the CTRL_HEADER record.  This function
/// reconstructs it from the raw UTF-16LE bytes stored in `para.raw_para_text`.
///
/// The algorithm walks the raw byte stream and collects an ordered list of
/// (start_byte_index, end_byte_index_exclusive) ranges that hold the normal
/// text characters preceding each `0x0003` marker.  Those ranges are decoded
/// and matched -- in order -- to the Ruby controls in `para.controls`.
///
/// If `raw_para_text` is `None` (e.g. the paragraph has no PARA_TEXT record)
/// the function returns immediately without modification.
pub(crate) fn fixup_ruby_base_text(para: &mut HwpParagraph) {
    let raw = match para.raw_para_text.as_deref() {
        Some(r) if !r.is_empty() => r,
        _ => return,
    };

    // Collect the byte ranges of "text runs before each 0x0003 marker".
    // Each entry is (run_start, run_end_exclusive) in byte-index terms.
    let mut base_ranges: Vec<(usize, usize)> = Vec::new();

    let len = raw.len();
    let mut i = 0usize;
    // Start of the current plain-text run.
    let mut run_start = 0usize;

    while i + 1 < len {
        let ch = u16::from_le_bytes([raw[i], raw[i + 1]]);

        if ch == CTRL_MARKER {
            // Record the plain-text run that ends just before this marker.
            base_ranges.push((run_start, i));
            // Skip the 2-byte code unit plus the 14-byte parameter block.
            i += 2 + CTRL_PARAM_BYTES;
            // The next run starts after the parameter block.
            run_start = i;
        } else if (CTRL_CHAR_LOW..=CTRL_CHAR_HIGH).contains(&ch) {
            // Other extended control characters: skip 2-byte code unit + 14-byte block.
            i += 2 + CTRL_PARAM_BYTES;
            // These characters are not text, so they break the run.  Start a
            // new run after the parameter block so we do not accidentally
            // include the following bytes in the previous base_text.
            run_start = i;
        } else {
            // Normal character (includes surrogates -- we advance 2 bytes at a
            // time to stay aligned; the decode below handles surrogate pairs).
            i += 2;
        }
    }

    // Now match the collected ranges to Ruby controls in order.
    let mut ruby_iter = base_ranges.into_iter();

    for ctrl in para.controls.iter_mut() {
        if let HwpControl::Ruby { base_text, .. } = ctrl {
            if let Some((start, end)) = ruby_iter.next() {
                if start < end && end <= raw.len() {
                    *base_text = decode_utf16le_text_run(&raw[start..end]);
                }
            }
        }
    }
}

/// Decode a slice of raw UTF-16LE bytes into a `String`.
///
/// Uses lossy decoding to match HWP's permissive behaviour with unpaired
/// surrogates.
fn decode_utf16le_text_run(bytes: &[u8]) -> String {
    let mut units: Vec<u16> = Vec::with_capacity(bytes.len() / 2);
    let mut i = 0;
    while i + 1 < bytes.len() {
        units.push(u16::from_le_bytes([bytes[i], bytes[i + 1]]));
        i += 2;
    }
    String::from_utf16_lossy(&units).to_owned()
}

/// Parse a `ruby` CTRL_HEADER record into the ruby annotation text.
///
/// Ruby control data layout (after the 4-byte ctrl_id at offset 0):
/// - Bytes 4..6: ruby text length as u16 LE (count of UTF-16 code units)
/// - Bytes 6..:  ruby text in UTF-16LE
///
/// The base text is carried in the paragraph text that immediately precedes
/// the control character.  This function only extracts the annotation.
/// An empty `ruby_text` is valid (the control exists but has no annotation).
/// Returns `None` only when the record data is too short to contain even the
/// length field.
pub(crate) fn parse_ruby_ctrl(rec: &Record) -> Option<String> {
    // Need at least 4 bytes for ctrl_id + 2 bytes for the length field.
    if rec.data.len() < 6 {
        return None;
    }
    let (ruby_text, _) = read_utf16le_str(&rec.data, 4);
    Some(ruby_text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hwp::record::{CTRL_RUBY, HWPTAG_CTRL_HEADER};

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn make_ruby_record(ruby_text: &str) -> Record {
        let chars: Vec<u16> = ruby_text.encode_utf16().collect();
        let mut data = CTRL_RUBY.to_le_bytes().to_vec();
        data.extend_from_slice(&(chars.len() as u16).to_le_bytes());
        for ch in &chars {
            data.extend_from_slice(&ch.to_le_bytes());
        }
        Record {
            tag_id: HWPTAG_CTRL_HEADER,
            level: 0,
            data,
        }
    }

    /// Encode a slice of UTF-16 code units as little-endian bytes.
    fn encode_u16s(units: &[u16]) -> Vec<u8> {
        let mut buf = Vec::with_capacity(units.len() * 2);
        for &u in units {
            buf.push((u & 0xFF) as u8);
            buf.push((u >> 8) as u8);
        }
        buf
    }

    /// Build a `HwpParagraph` with one Ruby control and raw_para_text set to
    /// the given bytes.
    fn make_para_with_ruby(raw: Vec<u8>, ruby_text: &str) -> HwpParagraph {
        HwpParagraph {
            text: String::new(),
            char_shape_ids: Vec::new(),
            para_shape_id: 0,
            controls: vec![HwpControl::Ruby {
                base_text: String::new(),
                ruby_text: ruby_text.to_string(),
            }],
            raw_para_text: Some(raw),
        }
    }

    // -----------------------------------------------------------------------
    // parse_ruby_ctrl
    // -----------------------------------------------------------------------

    #[test]
    fn parse_ruby_ctrl_korean_annotation() {
        let rec = make_ruby_record("한자");
        let result = parse_ruby_ctrl(&rec).expect("should return Some");
        assert_eq!(result, "한자");
    }

    #[test]
    fn parse_ruby_ctrl_ascii_annotation() {
        let rec = make_ruby_record("kanji");
        let result = parse_ruby_ctrl(&rec).expect("should return Some");
        assert_eq!(result, "kanji");
    }

    #[test]
    fn parse_ruby_ctrl_empty_annotation() {
        let rec = make_ruby_record("");
        let result = parse_ruby_ctrl(&rec).expect("should return Some for empty annotation");
        assert_eq!(result, "");
    }

    #[test]
    fn parse_ruby_ctrl_too_short_returns_none() {
        // Only 4 bytes (ctrl_id only, no length field).
        let data = CTRL_RUBY.to_le_bytes().to_vec();
        let rec = Record {
            tag_id: HWPTAG_CTRL_HEADER,
            level: 0,
            data,
        };
        assert!(parse_ruby_ctrl(&rec).is_none());
    }

    #[test]
    fn parse_ruby_ctrl_exactly_six_bytes_empty_annotation() {
        // 4 bytes ctrl_id + 2 bytes length=0 -> empty ruby text, returns Some("").
        let mut data = CTRL_RUBY.to_le_bytes().to_vec();
        data.extend_from_slice(&0u16.to_le_bytes());
        let rec = Record {
            tag_id: HWPTAG_CTRL_HEADER,
            level: 0,
            data,
        };
        let result = parse_ruby_ctrl(&rec).expect("6-byte record must return Some");
        assert!(result.is_empty());
    }

    // -----------------------------------------------------------------------
    // fixup_ruby_base_text
    // -----------------------------------------------------------------------

    /// Build a PARA_TEXT byte stream: `base_chars` in UTF-16LE, then one
    /// CTRL_MARKER (0x0003) code unit, then 14 zero bytes for the parameter
    /// block.
    fn make_raw_with_base(base_chars: &str) -> Vec<u8> {
        let units: Vec<u16> = base_chars.encode_utf16().collect();
        let mut raw = encode_u16s(&units);
        // Append the 0x0003 marker.
        raw.extend_from_slice(&CTRL_MARKER.to_le_bytes());
        // Append the 14-byte parameter block.
        raw.extend_from_slice(&[0u8; CTRL_PARAM_BYTES]);
        raw
    }

    #[test]
    fn fixup_ruby_base_text_sets_base_text_from_raw() {
        let raw = make_raw_with_base("漢字");
        let mut para = make_para_with_ruby(raw, "한자");
        fixup_ruby_base_text(&mut para);
        if let HwpControl::Ruby {
            base_text,
            ruby_text,
        } = &para.controls[0]
        {
            assert_eq!(base_text, "漢字");
            assert_eq!(ruby_text, "한자");
        } else {
            panic!("expected Ruby control");
        }
    }

    #[test]
    fn fixup_ruby_base_text_empty_base_at_paragraph_start() {
        // 0x0003 is the very first code unit -- base_text should be empty.
        let mut raw = CTRL_MARKER.to_le_bytes().to_vec();
        raw.extend_from_slice(&[0u8; CTRL_PARAM_BYTES]);
        let mut para = make_para_with_ruby(raw, "ルビ");
        fixup_ruby_base_text(&mut para);
        if let HwpControl::Ruby { base_text, .. } = &para.controls[0] {
            assert!(
                base_text.is_empty(),
                "base_text at paragraph start must be empty, got {base_text:?}"
            );
        } else {
            panic!("expected Ruby control");
        }
    }

    #[test]
    fn fixup_ruby_base_text_multiple_ruby_controls() {
        // Two ruby controls in one paragraph: "AB" before first, "CD" before second.
        let units_ab: Vec<u16> = "AB".encode_utf16().collect();
        let units_cd: Vec<u16> = "CD".encode_utf16().collect();
        let mut raw = encode_u16s(&units_ab);
        raw.extend_from_slice(&CTRL_MARKER.to_le_bytes());
        raw.extend_from_slice(&[0u8; CTRL_PARAM_BYTES]);
        raw.extend_from_slice(&encode_u16s(&units_cd));
        raw.extend_from_slice(&CTRL_MARKER.to_le_bytes());
        raw.extend_from_slice(&[0u8; CTRL_PARAM_BYTES]);

        let mut para = HwpParagraph {
            text: String::new(),
            char_shape_ids: Vec::new(),
            para_shape_id: 0,
            controls: vec![
                HwpControl::Ruby {
                    base_text: String::new(),
                    ruby_text: "ann1".to_string(),
                },
                HwpControl::Ruby {
                    base_text: String::new(),
                    ruby_text: "ann2".to_string(),
                },
            ],
            raw_para_text: Some(raw),
        };
        fixup_ruby_base_text(&mut para);

        let bases: Vec<&str> = para
            .controls
            .iter()
            .filter_map(|c| {
                if let HwpControl::Ruby { base_text, .. } = c {
                    Some(base_text.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(bases, vec!["AB", "CD"]);
    }

    #[test]
    fn fixup_ruby_base_text_no_raw_does_nothing() {
        let mut para = HwpParagraph {
            text: String::new(),
            char_shape_ids: Vec::new(),
            para_shape_id: 0,
            controls: vec![HwpControl::Ruby {
                base_text: String::new(),
                ruby_text: "ann".to_string(),
            }],
            raw_para_text: None,
        };
        fixup_ruby_base_text(&mut para);
        if let HwpControl::Ruby { base_text, .. } = &para.controls[0] {
            assert!(base_text.is_empty(), "should remain empty when no raw data");
        }
    }

    #[test]
    fn fixup_ruby_base_text_other_ctrl_char_breaks_run() {
        // A non-0x0003 control char (0x0001) should reset the run so the base
        // text is empty for the Ruby that follows directly.
        let mut raw = Vec::new();
        // Text before the non-ruby control.
        let units_hi: Vec<u16> = "Hi".encode_utf16().collect();
        raw.extend_from_slice(&encode_u16s(&units_hi));
        // 0x0001 control char + 14-byte param block.
        raw.extend_from_slice(&0x0001u16.to_le_bytes());
        raw.extend_from_slice(&[0u8; CTRL_PARAM_BYTES]);
        // Immediately followed by 0x0003 (no normal text between them).
        raw.extend_from_slice(&CTRL_MARKER.to_le_bytes());
        raw.extend_from_slice(&[0u8; CTRL_PARAM_BYTES]);

        let mut para = make_para_with_ruby(raw, "ann");
        fixup_ruby_base_text(&mut para);
        if let HwpControl::Ruby { base_text, .. } = &para.controls[0] {
            assert!(
                base_text.is_empty(),
                "run must be broken by other ctrl char; got {base_text:?}"
            );
        } else {
            panic!("expected Ruby control");
        }
    }
}
