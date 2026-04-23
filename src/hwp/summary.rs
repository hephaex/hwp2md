use std::io::Read;

/// Maximum size for a raw CFB stream read from untrusted HWP input (256 MB).
const MAX_CFB_STREAM: u64 = 256 * 1024 * 1024;

const PROP_TITLE: u32 = 0x02;
const PROP_SUBJECT: u32 = 0x03;
const PROP_AUTHOR: u32 = 0x04;
const PROP_KEYWORDS: u32 = 0x06;

const VT_LPSTR: u32 = 0x1E;

pub(crate) fn read_summary_info(
    cfb: &mut cfb::CompoundFile<std::fs::File>,
) -> (Option<String>, Option<String>, Option<String>, Vec<String>) {
    let stream_name = "\x05SummaryInformation";
    let mut raw = Vec::new();
    match cfb.open_stream(stream_name) {
        Ok(s) => {
            if s.take(MAX_CFB_STREAM).read_to_end(&mut raw).is_err() {
                tracing::debug!("SummaryInformation: read failed");
                return (None, None, None, Vec::new());
            }
        }
        Err(e) => {
            tracing::debug!("SummaryInformation stream not found: {e}");
            return (None, None, None, Vec::new());
        }
    }
    parse_summary_bytes(&raw)
}

pub(crate) fn parse_summary_bytes(
    raw: &[u8],
) -> (Option<String>, Option<String>, Option<String>, Vec<String>) {
    let empty = || (None, None, None, Vec::new());

    if raw.len() < 48 {
        tracing::debug!("SummaryInformation: stream too short ({} bytes)", raw.len());
        return empty();
    }

    if raw[0] != 0xFE || raw[1] != 0xFF {
        tracing::debug!("SummaryInformation: unexpected byte-order mark");
        return empty();
    }

    let sec_offset = u32::from_le_bytes([raw[44], raw[45], raw[46], raw[47]]) as usize;
    if sec_offset + 8 > raw.len() {
        tracing::debug!("SummaryInformation: section offset out of range");
        return empty();
    }

    let prop_count = u32::from_le_bytes([
        raw[sec_offset + 4],
        raw[sec_offset + 5],
        raw[sec_offset + 6],
        raw[sec_offset + 7],
    ]) as usize;

    let dir_start = sec_offset + 8;
    let dir_end = prop_count
        .checked_mul(8)
        .and_then(|n| dir_start.checked_add(n));
    if dir_end.map_or(true, |e| e > raw.len()) {
        tracing::debug!("SummaryInformation: property directory truncated");
        return empty();
    }

    let read_lpstr = |prop_offset: usize| -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn build_summary_bytes(props: &[(u32, &str)]) -> Vec<u8> {
        let mut buf = Vec::new();

        // Header (28 bytes)
        buf.extend_from_slice(&[0xFE, 0xFF]); // BOM
        buf.extend_from_slice(&[0x00, 0x00]); // version
        buf.extend_from_slice(&[0x02, 0x00, 0x00, 0x00]); // OS
        buf.extend_from_slice(&[0u8; 16]); // CLSID
        buf.extend_from_slice(&1u32.to_le_bytes()); // section count

        // Section entry: FMTID (16 bytes) + offset (4 bytes)
        buf.extend_from_slice(&[0u8; 16]); // FMTID
        let sec_offset: u32 = 48;
        buf.extend_from_slice(&sec_offset.to_le_bytes());

        // Section header: byte-count (placeholder) + prop_count
        let sec_start = buf.len();
        buf.extend_from_slice(&0u32.to_le_bytes()); // byte-count placeholder
        buf.extend_from_slice(&(props.len() as u32).to_le_bytes());

        // Property directory (8 bytes each)
        let dir_size = props.len() * 8;
        let mut prop_data: Vec<u8> = Vec::new();
        let data_base_offset = 8 + dir_size; // relative to sec_offset

        for (i, (prop_id, value)) in props.iter().enumerate() {
            let entry_offset = 8 + i * 8;
            let _ = entry_offset;
            buf.extend_from_slice(&prop_id.to_le_bytes());
            let prop_offset = data_base_offset + prop_data.len();
            buf.extend_from_slice(&(prop_offset as u32).to_le_bytes());

            // VT_LPSTR value: type (4) + size (4) + data (padded to 4-byte)
            prop_data.extend_from_slice(&VT_LPSTR.to_le_bytes());
            let bytes = value.as_bytes();
            let size = bytes.len() + 1; // include NUL
            prop_data.extend_from_slice(&(size as u32).to_le_bytes());
            prop_data.extend_from_slice(bytes);
            prop_data.push(0); // NUL
                               // Pad to 4-byte alignment
            while prop_data.len() % 4 != 0 {
                prop_data.push(0);
            }
        }

        buf.extend_from_slice(&prop_data);

        // Patch section byte-count
        let sec_size = (buf.len() - sec_start) as u32;
        buf[sec_start..sec_start + 4].copy_from_slice(&sec_size.to_le_bytes());

        buf
    }

    #[test]
    fn parse_summary_bytes_valid_title_and_author() {
        let raw = build_summary_bytes(&[(PROP_TITLE, "Test Doc"), (PROP_AUTHOR, "Author Name")]);
        let (title, author, subject, keywords) = parse_summary_bytes(&raw);
        assert_eq!(title.as_deref(), Some("Test Doc"));
        assert_eq!(author.as_deref(), Some("Author Name"));
        assert!(subject.is_none());
        assert!(keywords.is_empty());
    }

    #[test]
    fn parse_summary_bytes_empty_data_returns_all_none() {
        let (t, a, s, k) = parse_summary_bytes(&[]);
        assert!(t.is_none());
        assert!(a.is_none());
        assert!(s.is_none());
        assert!(k.is_empty());
    }

    #[test]
    fn parse_summary_bytes_bad_bom_returns_all_none() {
        let mut raw = build_summary_bytes(&[(PROP_TITLE, "X")]);
        raw[0] = 0x00; // corrupt BOM
        let (t, a, s, k) = parse_summary_bytes(&raw);
        assert!(t.is_none());
        assert!(a.is_none());
        assert!(s.is_none());
        assert!(k.is_empty());
    }

    #[test]
    fn parse_summary_bytes_truncated_returns_all_none() {
        let raw = build_summary_bytes(&[(PROP_TITLE, "Hello")]);
        let truncated = &raw[..30]; // cut in header
        let (t, a, _, _) = parse_summary_bytes(truncated);
        assert!(t.is_none());
        assert!(a.is_none());
    }

    #[test]
    fn parse_summary_bytes_subject_field() {
        let raw = build_summary_bytes(&[
            (PROP_TITLE, "Doc Title"),
            (PROP_SUBJECT, "This is the subject"),
        ]);
        let (title, _author, subject, _keywords) = parse_summary_bytes(&raw);
        assert_eq!(title.as_deref(), Some("Doc Title"));
        assert_eq!(subject.as_deref(), Some("This is the subject"));
    }

    #[test]
    fn parse_summary_bytes_keywords_comma_separated() {
        let raw = build_summary_bytes(&[(PROP_KEYWORDS, "rust,hwp,converter")]);
        let (_t, _a, _s, keywords) = parse_summary_bytes(&raw);
        assert!(keywords.contains(&"rust".to_string()));
        assert!(keywords.contains(&"hwp".to_string()));
        assert!(keywords.contains(&"converter".to_string()));
    }

    #[test]
    fn parse_summary_bytes_keywords_semicolon_separated() {
        let raw = build_summary_bytes(&[(PROP_KEYWORDS, "alpha;beta;gamma")]);
        let (_t, _a, _s, keywords) = parse_summary_bytes(&raw);
        assert!(keywords.contains(&"alpha".to_string()));
        assert!(keywords.contains(&"beta".to_string()));
        assert!(keywords.contains(&"gamma".to_string()));
    }

    #[test]
    fn parse_summary_bytes_keywords_space_separated() {
        let raw = build_summary_bytes(&[(PROP_KEYWORDS, "one two three")]);
        let (_t, _a, _s, keywords) = parse_summary_bytes(&raw);
        assert!(keywords.contains(&"one".to_string()));
        assert!(keywords.contains(&"two".to_string()));
        assert!(keywords.contains(&"three".to_string()));
    }

    #[test]
    fn parse_summary_bytes_all_fields() {
        let raw = build_summary_bytes(&[
            (PROP_TITLE, "Full Doc"),
            (PROP_AUTHOR, "Full Author"),
            (PROP_SUBJECT, "Full Subject"),
            (PROP_KEYWORDS, "kw1,kw2"),
        ]);
        let (title, author, subject, keywords) = parse_summary_bytes(&raw);
        assert_eq!(title.as_deref(), Some("Full Doc"));
        assert_eq!(author.as_deref(), Some("Full Author"));
        assert_eq!(subject.as_deref(), Some("Full Subject"));
        assert_eq!(keywords, vec!["kw1", "kw2"]);
    }

    #[test]
    fn parse_summary_bytes_section_offset_out_of_range_returns_none() {
        // Build 48 bytes with valid BOM but section offset pointing beyond buffer.
        let mut raw = vec![0u8; 48];
        raw[0] = 0xFE;
        raw[1] = 0xFF;
        // Set section offset to 1000 (way beyond buf end).
        let offset: u32 = 1000;
        raw[44..48].copy_from_slice(&offset.to_le_bytes());
        let (t, a, s, k) = parse_summary_bytes(&raw);
        assert!(t.is_none());
        assert!(a.is_none());
        assert!(s.is_none());
        assert!(k.is_empty());
    }

    #[test]
    fn parse_summary_bytes_prop_directory_truncated_returns_none() {
        // Build a buffer where prop_count is large but directory would exceed buffer.
        let mut raw = vec![0u8; 60];
        raw[0] = 0xFE;
        raw[1] = 0xFF;
        let sec_offset: u32 = 44;
        raw[44..48].copy_from_slice(&sec_offset.to_le_bytes());
        // sec_offset+4 = prop_count = 1000 (huge)
        let prop_count: u32 = 1000;
        raw[48..52].copy_from_slice(&prop_count.to_le_bytes());
        let (t, a, s, k) = parse_summary_bytes(&raw);
        assert!(t.is_none());
        assert!(a.is_none());
        assert!(s.is_none());
        assert!(k.is_empty());
    }

    #[test]
    fn parse_summary_bytes_unknown_prop_id_ignored() {
        // An unknown property ID (not TITLE/AUTHOR/SUBJECT/KEYWORDS) should be skipped.
        let raw = build_summary_bytes(&[(0xFF, "ignored value"), (PROP_TITLE, "Known Title")]);
        let (title, _author, _subject, _keywords) = parse_summary_bytes(&raw);
        assert_eq!(title.as_deref(), Some("Known Title"));
    }
}
