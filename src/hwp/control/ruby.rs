use crate::hwp::record::{read_utf16le_str, Record};

/// Parse a `ruby` CTRL_HEADER record into `(base_text, ruby_text)`.
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
        // 4 bytes ctrl_id + 2 bytes length=0 → empty ruby text, returns Some("").
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
}
