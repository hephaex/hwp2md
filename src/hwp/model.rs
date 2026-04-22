use std::collections::HashMap;

#[derive(Debug)]
pub struct HwpDocument {
    pub header: FileHeader,
    pub doc_info: DocInfo,
    pub sections: Vec<HwpSection>,
    pub bin_data: HashMap<u16, Vec<u8>>,
    pub summary_title: Option<String>,
    pub summary_author: Option<String>,
    pub summary_subject: Option<String>,
    pub summary_keywords: Vec<String>,
}

#[derive(Debug, Default)]
pub struct FileHeader {
    pub version: HwpVersion,
    pub compressed: bool,
    pub encrypted: bool,
    pub distributed: bool,
    pub has_script: bool,
    pub has_drm: bool,
    pub has_xml_template: bool,
    pub has_history: bool,
    pub has_cert: bool,
    pub has_cert_drm: bool,
    pub has_ccl: bool,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct HwpVersion {
    pub major: u8,
    pub minor: u8,
    pub micro: u8,
    pub extra: u8,
}

impl std::fmt::Display for HwpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}",
            self.major, self.minor, self.micro, self.extra
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hwp_version_display_formats_correctly() {
        let v = HwpVersion {
            major: 5,
            minor: 1,
            micro: 0,
            extra: 0,
        };
        assert_eq!(format!("{v}"), "5.1.0.0");
    }

    #[test]
    fn hwp_version_default_is_all_zeros() {
        let v = HwpVersion::default();
        assert_eq!(format!("{v}"), "0.0.0.0");
    }

    #[test]
    fn hwp_version_display_all_fields() {
        let v = HwpVersion {
            major: 5,
            minor: 3,
            micro: 2,
            extra: 1,
        };
        assert_eq!(format!("{v}"), "5.3.2.1");
    }
}

#[derive(Debug, Default)]
pub struct DocInfo {
    pub char_shapes: Vec<CharShape>,
    pub para_shapes: Vec<ParaShape>,
    pub face_names: Vec<String>,
    pub bin_data_entries: Vec<BinDataEntry>,
    pub doc_properties: DocProperties,
}

#[derive(Debug, Default)]
pub struct DocProperties {
    pub section_count: u16,
}

#[derive(Debug, Default, Clone)]
pub struct CharShape {
    pub face_id: u16,
    pub height: u32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub superscript: bool,
    pub subscript: bool,
    pub color: u32,
}

#[derive(Debug, Default, Clone)]
pub struct ParaShape {
    pub alignment: Alignment,
    pub heading_type: Option<u8>,
    pub indent: i32,
    pub margin_left: i32,
    pub margin_right: i32,
    pub line_spacing: i32,
    pub line_spacing_type: u8,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, Default)]
pub struct BinDataEntry {
    pub r#type: u16,
    pub abs_path: Option<String>,
    pub rel_path: Option<String>,
    pub id: u16,
    pub extension: String,
}

#[derive(Debug)]
pub struct HwpSection {
    pub paragraphs: Vec<HwpParagraph>,
}

#[derive(Debug)]
pub struct HwpParagraph {
    pub text: String,
    pub char_shape_ids: Vec<(u32, u16)>,
    pub para_shape_id: u16,
    pub controls: Vec<HwpControl>,
}

#[derive(Debug)]
pub enum HwpControl {
    Table {
        row_count: u16,
        col_count: u16,
        cells: Vec<HwpTableCell>,
    },
    Equation {
        script: String,
    },
    Image {
        bin_data_id: u16,
        width: u32,
        height: u32,
    },
    Hyperlink {
        url: String,
    },
    FootnoteEndnote {
        is_endnote: bool,
        paragraphs: Vec<HwpParagraph>,
    },
    PageBreak,
    SectionBreak,
    ColumnBreak,
}

#[derive(Debug)]
pub struct HwpTableCell {
    pub row: u16,
    pub col: u16,
    pub row_span: u16,
    pub col_span: u16,
    pub paragraphs: Vec<HwpParagraph>,
}
