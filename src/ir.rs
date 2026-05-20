//! Intermediate representation (IR) types shared by all readers and writers.

use serde::{Deserialize, Serialize};

/// Top-level document produced by any reader and consumed by any writer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    /// Document-level metadata (title, author, dates, …).
    pub metadata: Metadata,
    /// Ordered list of content sections.
    pub sections: Vec<Section>,
    /// Embedded binary assets (images, fonts, …).
    pub assets: Vec<Asset>,
}

impl Document {
    /// Create an empty document with default metadata.
    #[must_use]
    pub fn new() -> Self {
        Self {
            metadata: Metadata::default(),
            sections: Vec::new(),
            assets: Vec::new(),
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

/// Document-level metadata extracted from HWP/HWPX summary streams.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
    /// Document title.
    pub title: Option<String>,
    /// Primary author.
    pub author: Option<String>,
    /// Creation timestamp (ISO 8601 string when available).
    pub created: Option<String>,
    /// Last-modified timestamp (ISO 8601 string when available).
    pub modified: Option<String>,
    /// Short description or abstract.
    pub description: Option<String>,
    /// Subject or category.
    pub subject: Option<String>,
    /// Keyword tags.
    pub keywords: Vec<String>,
}

/// Page layout metadata parsed from `<hp:secPr>` in HWPX section XML.
///
/// All dimension values use HWP units (1/7200 inch ≈ 0.00353 mm).
/// An A4 portrait page is approximately 59528 × 84188 HWP units.
///
/// This struct is stored on [`Section`] because HWPX allows each section to
/// have independent page layout settings via its own `<hp:secPr>` element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageLayout {
    /// Page width in HWP units (`<hp:pageSize width="…"/>`).
    pub width: Option<u32>,
    /// Page height in HWP units (`<hp:pageSize height="…"/>`).
    pub height: Option<u32>,
    /// `true` when the page is in landscape orientation
    /// (`<hp:pagePr landscape="true"/>`).
    pub landscape: bool,
    /// Left margin in HWP units (`<hp:margin left="…"/>`).
    pub margin_left: Option<u32>,
    /// Right margin in HWP units (`<hp:margin right="…"/>`).
    pub margin_right: Option<u32>,
    /// Top margin in HWP units (`<hp:margin top="…"/>`).
    pub margin_top: Option<u32>,
    /// Bottom margin in HWP units (`<hp:margin bottom="…"/>`).
    pub margin_bottom: Option<u32>,
}

impl PageLayout {
    /// Standard A4 portrait page layout with typical HWP margins.
    ///
    /// - Size: 210 × 297 mm (59528 × 84188 HWP units, 1 HWP unit = 1/7200 inch)
    /// - Margins: 20 mm header/footer, 30 mm left/right (5670 / 4252 HWP units)
    #[must_use]
    pub fn a4_portrait() -> Self {
        Self {
            width: Some(59528),
            height: Some(84188),
            landscape: false,
            margin_left: Some(5670),
            margin_right: Some(5670),
            margin_top: Some(4252),
            margin_bottom: Some(4252),
        }
    }
}

/// Scope of a `<hp:headerFooter>` element, from the OWPML `type` attribute.
///
/// The OWPML specification defines three known values; any other value that
/// appears in the wild is preserved via the [`Other`][HeaderFooterType::Other]
/// variant so that round-trip fidelity is not lost for non-standard documents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HeaderFooterType {
    /// Applies to all pages (`type="both"`).
    Both,
    /// Applies to even-numbered pages only (`type="even"`).
    Even,
    /// Applies to odd-numbered pages only (`type="odd"`).
    Odd,
    /// An unrecognised value preserved verbatim for round-trip fidelity.
    #[serde(untagged)]
    Other(String),
}

impl HeaderFooterType {
    /// Return the OWPML attribute string for this variant.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Both => "both",
            Self::Even => "even",
            Self::Odd => "odd",
            Self::Other(s) => s.as_str(),
        }
    }
}

impl From<&str> for HeaderFooterType {
    /// Construct a `HeaderFooterType` from a string slice, normalizing known values.
    ///
    /// Known OWPML values (`"both"`, `"even"`, `"odd"`) are converted to their
    /// corresponding enum variants. Any unrecognized value becomes [`Other`][Self::Other].
    /// This ensures that known values can only be constructed via their dedicated variants,
    /// maintaining serde round-trip consistency.
    fn from(s: &str) -> Self {
        match s {
            "both" => Self::Both,
            "even" => Self::Even,
            "odd" => Self::Odd,
            other => Self::Other(other.to_string()),
        }
    }
}

impl From<String> for HeaderFooterType {
    /// Construct a `HeaderFooterType` from an owned string, normalizing known values.
    ///
    /// This delegates to [`From<&str>`] to avoid code duplication.
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

/// Paragraph break-flow settings from OWPML `hh:breakSetting`.
///
/// Stored at the section level because HWPX encodes these values inside
/// `hh:paraPr` entries in `header.xml` rather than per-paragraph in
/// `section.xml`.  The default (`BreakSetting::default()`) corresponds to
/// all four flags being `false`, matching the OWPML schema default.
///
/// The four fields map directly to OWPML boolean attributes and are
/// semantically independent, so four `bool` fields is the correct
/// representation here.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BreakSetting {
    /// Prevent lone lines at the top or bottom of a page (widow/orphan control).
    pub widow_orphan: bool,
    /// Keep this paragraph on the same page as the following paragraph.
    pub keep_with_next: bool,
    /// Keep all lines of this paragraph on the same page.
    pub keep_lines: bool,
    /// Force a page break before this paragraph.
    pub page_break_before: bool,
}

/// A logical section of a document containing an ordered sequence of blocks.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Section {
    /// Content blocks in reading order.
    pub blocks: Vec<Block>,
    /// Page layout for this section, parsed from `<hp:secPr>` in HWPX.
    ///
    /// `None` when the source document did not include section properties
    /// (e.g. plain Markdown input or very minimal HWPX files).  Writers
    /// should fall back to [`PageLayout::a4_portrait`] defaults in that case.
    pub page_layout: Option<PageLayout>,
    /// Header text blocks (from HWPX `<hp:headerFooter>` → `<hp:header>`).
    ///
    /// `None` when the source document has no header definition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<Vec<Block>>,
    /// Footer text blocks (from HWPX `<hp:headerFooter>` → `<hp:footer>`).
    ///
    /// `None` when the source document has no footer definition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footer: Option<Vec<Block>>,
    /// Header/footer scope (from `<hp:headerFooter type="…">` attribute).
    ///
    /// `None` means no `type` attribute was present (document default).
    /// This specifies which pages the header/footer applies to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_footer_type: Option<HeaderFooterType>,
    /// Break-flow settings parsed from `hh:breakSetting` in header.xml.
    ///
    /// Defaults to all-`false` when header.xml is absent or contains no
    /// `hh:breakSetting` element (which matches the OWPML schema default).
    #[serde(default)]
    pub break_setting: BreakSetting,
}

/// A block-level content element within a section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Block {
    /// ATX heading with a level in `1..=6`.
    Heading {
        /// Heading level (1 = `#`, 6 = `######`).
        level: u8,
        /// Inline content of the heading.
        inlines: Vec<Inline>,
    },
    /// Plain paragraph of inline content.
    Paragraph {
        /// Inline runs in reading order.
        inlines: Vec<Inline>,
    },
    /// Table with a fixed column count.
    Table {
        /// Rows in top-to-bottom order.
        rows: Vec<TableRow>,
        /// Total number of columns (used for alignment row generation).
        col_count: usize,
        /// Optional inner margin override. `None` means use the writer default (141 HWP units).
        inner_margin: Option<TableInnerMargin>,
    },
    /// Fenced code block.
    CodeBlock {
        /// Optional language hint (e.g. `"rust"`).
        language: Option<String>,
        /// Raw source code.
        code: String,
    },
    /// Block quotation wrapping nested blocks.
    BlockQuote {
        /// Quoted blocks.
        blocks: Vec<Block>,
    },
    /// Ordered or unordered list.
    List {
        /// `true` for an ordered (`1.`) list, `false` for a bullet list.
        ordered: bool,
        /// Starting number for ordered lists.
        start: u32,
        /// List items in order.
        items: Vec<ListItem>,
    },
    /// Inline image.
    Image {
        /// Image source URI or data URL.
        src: String,
        /// Alternative text.
        alt: String,
    },
    /// Thematic break (`---`).
    HorizontalRule,
    /// Forced page break.
    ///
    /// Maps to OWPML `<hp:ctrl id="newPage"/>` on the HWPX side and to a
    /// `<!-- pagebreak -->` HTML comment on the Markdown side so that the
    /// Markdown renders as plain content while the marker survives a
    /// MD → HWPX → MD round-trip.
    PageBreak,
    /// Footnote definition collected from the source document.
    Footnote {
        /// Unique identifier matching the `footnote_ref` on the call-out inline.
        id: String,
        /// Block content of the footnote body.
        content: Vec<Block>,
    },
    /// Mathematical expression in TeX syntax.
    Math {
        /// `true` for a display (block) equation, `false` for inline.
        display: bool,
        /// TeX source.
        tex: String,
    },
}

/// Formatting attributes for an inline text run.
// Independent text decoration flags; each flag controls a distinct orthogonal property. Bitflags would add complexity without safety benefit.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct InlineFormat {
    /// Bold weight.
    pub bold: bool,
    /// Italic style.
    pub italic: bool,
    /// Underline decoration.
    pub underline: bool,
    /// Strikethrough decoration.
    pub strikethrough: bool,
    /// Superscript (`<sup>`).
    pub superscript: bool,
    /// Subscript (`<sub>`).
    pub subscript: bool,
    /// CSS hex color string (e.g. `"#FF0000"`) when text color is non-black.
    pub color: Option<String>,
}

/// A run of inline text with optional formatting and annotations.
// Bool fields represent format flags (bold, italic, underline, etc.) that are independent.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inline {
    /// The raw text of this run.
    pub text: String,
    /// Bold weight.
    pub bold: bool,
    /// Italic style.
    pub italic: bool,
    /// Underline decoration.
    pub underline: bool,
    /// Strikethrough decoration.
    pub strikethrough: bool,
    /// Monospace / inline-code style.
    pub code: bool,
    /// Superscript (`<sup>`).
    pub superscript: bool,
    /// Subscript (`<sub>`).
    pub subscript: bool,
    /// Hyperlink target URL when this run is a link.
    pub link: Option<String>,
    /// ID of the footnote definition this run references.
    pub footnote_ref: Option<String>,
    /// CSS hex color string (e.g. `"#FF0000"`) when text color is non-black.
    /// `None` means default/black text, which is not rendered in output.
    pub color: Option<String>,
    /// Font name resolved from the `DocInfo` `face_names` table.
    /// Not rendered in Markdown output; preserved for HWPX round-trip fidelity.
    pub font_name: Option<String>,
    /// Ruby annotation text.  When `Some`, the inline's `text` is the base
    /// character(s) and this field holds the small annotation above them.
    /// Rendered as `<ruby>base<rt>annotation</rt></ruby>` in Markdown output.
    pub ruby: Option<String>,
}

impl Inline {
    /// Create a plain, unformatted inline run from `text`.
    #[must_use]
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }

    /// Create a bold inline run from `text`.
    #[must_use]
    pub fn bold(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            bold: true,
            ..Self::default()
        }
    }

    /// Construct an inline with all formatting fields set from an [`InlineFormat`].
    ///
    /// The `link`, `footnote_ref`, `font_name`, `code`, and `ruby` fields are
    /// left at their defaults and can be set via chained builder methods
    /// (`with_ruby`, `with_link`, etc.).
    #[must_use]
    pub fn with_formatting(text: String, fmt: &InlineFormat) -> Self {
        Self {
            text,
            bold: fmt.bold,
            italic: fmt.italic,
            underline: fmt.underline,
            strikethrough: fmt.strikethrough,
            superscript: fmt.superscript,
            subscript: fmt.subscript,
            color: fmt.color.clone(),
            code: false,
            link: None,
            footnote_ref: None,
            font_name: None,
            ruby: None,
        }
    }

    /// Set the `link` field, returning the modified inline.
    #[must_use]
    pub fn with_link(mut self, link: Option<String>) -> Self {
        self.link = link;
        self
    }

    /// Set the `ruby` annotation field, returning the modified inline.
    #[must_use]
    pub fn with_ruby(mut self, ruby: Option<String>) -> Self {
        self.ruby = ruby;
        self
    }

    /// Set the `font_name` field, returning the modified inline.
    #[must_use]
    pub fn with_font_name(mut self, font_name: Option<String>) -> Self {
        self.font_name = font_name;
        self
    }

    /// Create a footnote-reference inline with no visible text or formatting.
    #[must_use]
    pub fn footnote_ref(id: impl Into<String>) -> Self {
        Self {
            footnote_ref: Some(id.into()),
            ..Self::default()
        }
    }
}

/// OWPML default inner-cell gap in HWP units when `<hp:inMargin>` axes are unspecified.
///
/// Used by the HWPX reader as the default for missing axes and cross-referenced by
/// `TABLE_INNER_MARGIN` in `hwpx::writer_section` (its `&str` counterpart for XML emission).
pub(crate) const DEFAULT_TABLE_INNER_MARGIN: u32 = 141;

/// Inner margin between cells in a `Block::Table`, in HWP units (1/7200 inch).
///
/// Corresponds to `<hp:tblPr><hp:inMargin>` in OWPML. When `None`, the writer
/// falls back to `DEFAULT_TABLE_INNER_MARGIN` (141 HWP units ≈ 1.4 mm).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableInnerMargin {
    /// Left inner margin in HWP units.
    pub left: u32,
    /// Right inner margin in HWP units.
    pub right: u32,
    /// Top inner margin in HWP units.
    pub top: u32,
    /// Bottom inner margin in HWP units.
    pub bottom: u32,
}

/// A single row in a [`Block::Table`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableRow {
    /// Cells in left-to-right order.
    pub cells: Vec<TableCell>,
    /// `true` when this row should be treated as a header row.
    pub is_header: bool,
}

/// A single cell in a [`TableRow`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableCell {
    /// Block content inside the cell.
    pub blocks: Vec<Block>,
    /// Number of columns this cell spans (≥ 1).
    pub colspan: u32,
    /// Number of rows this cell spans (≥ 1).
    pub rowspan: u32,
}

impl Default for TableCell {
    fn default() -> Self {
        Self {
            blocks: Vec::new(),
            colspan: 1,
            rowspan: 1,
        }
    }
}

/// A single item in a [`Block::List`], optionally containing nested sub-lists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListItem {
    /// Block content of this item (typically one `Paragraph`).
    pub blocks: Vec<Block>,
    /// Nested child list items for multi-level lists.
    pub children: Vec<ListItem>,
    /// GitHub-style task list checkbox state.
    ///
    /// - `None` — normal list item, no checkbox
    /// - `Some(false)` — unchecked `- [ ]`
    /// - `Some(true)` — checked `- [x]`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
}

impl ListItem {
    /// Create a new unchecked list item with the given blocks and children.
    #[must_use]
    pub fn new(blocks: Vec<Block>, children: Vec<ListItem>) -> Self {
        Self {
            blocks,
            children,
            checked: None,
        }
    }
}

/// A binary asset (image, font, …) embedded in the document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asset {
    /// Original file name as recorded in the source archive.
    pub name: String,
    /// Raw bytes of the asset.
    pub data: Vec<u8>,
    /// MIME type (e.g. `"image/png"`).
    pub mime_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_plain_sets_text_and_all_bools_false() {
        let i = Inline::plain("hello");
        assert_eq!(i.text, "hello");
        assert!(!i.bold);
        assert!(!i.italic);
        assert!(!i.underline);
        assert!(!i.strikethrough);
        assert!(!i.code);
        assert!(!i.superscript);
        assert!(!i.subscript);
        assert!(i.link.is_none());
        assert!(i.footnote_ref.is_none());
        assert!(i.color.is_none());
        assert!(i.font_name.is_none());
        assert!(i.ruby.is_none());
    }

    #[test]
    fn inline_bold_sets_text_and_bold_true_rest_false() {
        let i = Inline::bold("strong");
        assert_eq!(i.text, "strong");
        assert!(i.bold);
        assert!(!i.italic);
        assert!(!i.underline);
        assert!(!i.strikethrough);
        assert!(!i.code);
        assert!(!i.superscript);
        assert!(!i.subscript);
        assert!(i.link.is_none());
        assert!(i.footnote_ref.is_none());
        assert!(i.color.is_none());
        assert!(i.font_name.is_none());
        assert!(i.ruby.is_none());
    }

    #[test]
    fn inline_default_color_and_font_name_are_none() {
        let i = Inline::default();
        assert!(i.color.is_none());
        assert!(i.font_name.is_none());
        assert!(i.ruby.is_none());
    }

    #[test]
    fn inline_ruby_field_roundtrips() {
        let i = Inline {
            text: "漢字".into(),
            ruby: Some("한자".into()),
            ..Inline::default()
        };
        assert_eq!(i.text, "漢字");
        assert_eq!(i.ruby.as_deref(), Some("한자"));
    }

    #[test]
    fn document_new_has_empty_sections_and_assets() {
        let doc = Document::new();
        assert!(doc.sections.is_empty());
        assert!(doc.assets.is_empty());
    }

    #[test]
    fn document_default_equals_new() {
        assert_eq!(Document::new(), Document::default());
    }

    #[test]
    fn list_item_new_has_checked_none() {
        let item = ListItem::new(vec![], vec![]);
        assert!(item.checked.is_none());
    }

    #[test]
    fn list_item_checked_some_false() {
        let item = ListItem {
            blocks: vec![],
            children: vec![],
            checked: Some(false),
        };
        assert_eq!(item.checked, Some(false));
    }

    #[test]
    fn list_item_checked_some_true() {
        let item = ListItem {
            blocks: vec![],
            children: vec![],
            checked: Some(true),
        };
        assert_eq!(item.checked, Some(true));
    }

    #[test]
    fn list_item_checked_none_by_default_via_new() {
        let item = ListItem::new(vec![Block::Paragraph { inlines: vec![] }], vec![]);
        assert!(
            item.checked.is_none(),
            "ListItem::new must produce checked=None"
        );
    }

    #[test]
    fn list_item_clone_preserves_checked() {
        let item = ListItem {
            blocks: vec![],
            children: vec![],
            checked: Some(true),
        };
        let cloned = item.clone();
        assert_eq!(cloned.checked, Some(true));
    }

    #[test]
    fn header_footer_type_serde_roundtrip() {
        use serde_json;

        // Known variants serialize to their string and deserialize back via From<String>.
        let cases: &[(HeaderFooterType, &str)] = &[
            (HeaderFooterType::Both, r#""both""#),
            (HeaderFooterType::Even, r#""even""#),
            (HeaderFooterType::Odd, r#""odd""#),
            (HeaderFooterType::Other("custom".to_string()), r#""custom""#),
        ];

        for (variant, expected_json) in cases {
            let json = serde_json::to_string(variant)
                .unwrap_or_else(|e| panic!("serialize {variant:?} failed: {e}"));
            assert_eq!(
                json, *expected_json,
                "serialized form mismatch for {variant:?}"
            );

            let back: HeaderFooterType = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("deserialize {json:?} failed: {e}"));
            assert_eq!(
                back, *variant,
                "deserialized value mismatch for {variant:?}"
            );
        }
    }

    #[test]
    fn header_footer_type_from_string_normalizes_known_values() {
        // Known values normalize to their enum variants via From<String>.
        assert_eq!(
            HeaderFooterType::from("both".to_string()),
            HeaderFooterType::Both
        );
        assert_eq!(
            HeaderFooterType::from("even".to_string()),
            HeaderFooterType::Even
        );
        assert_eq!(
            HeaderFooterType::from("odd".to_string()),
            HeaderFooterType::Odd
        );
    }

    #[test]
    fn header_footer_type_from_string_unknown_becomes_other() {
        // Unknown values become Other variant.
        assert_eq!(
            HeaderFooterType::from("custom".to_string()),
            HeaderFooterType::Other("custom".to_string())
        );
        assert_eq!(
            HeaderFooterType::from("unknown".to_string()),
            HeaderFooterType::Other("unknown".to_string())
        );
    }

    #[test]
    fn header_footer_type_known_never_created_as_other() {
        // Verify that known values cannot be created as Other via From.
        // If someone tries to create Other("both"), it normalizes to Both.
        let other_both = HeaderFooterType::from("both".to_string());
        match other_both {
            HeaderFooterType::Both => {
                // Expected: normalizes to Both, not Other("both")
            }
            HeaderFooterType::Other(_) => {
                panic!("Known value 'both' should not be wrapped in Other variant");
            }
            _ => panic!("Unexpected variant"),
        }
    }

    #[test]
    fn header_footer_type_from_empty_string() {
        // Empty string should become Other(""), not panic or match a known variant.
        let result = HeaderFooterType::from("");
        assert_eq!(result, HeaderFooterType::Other(String::new()));
    }

    #[test]
    fn header_footer_type_from_whitespace() {
        // Whitespace is not trimmed — " both " is treated as unknown.
        let result = HeaderFooterType::from(" both ");
        assert_eq!(result, HeaderFooterType::Other(" both ".to_string()));
    }

    #[test]
    fn header_footer_type_from_capitalized() {
        // Matching is case-sensitive — "Both" is treated as unknown, not normalized.
        let result = HeaderFooterType::from("Both");
        assert_eq!(result, HeaderFooterType::Other("Both".to_string()));
    }

    #[test]
    fn header_footer_type_from_str_ref() {
        // Verify From<&str> works correctly with known values.
        assert_eq!(HeaderFooterType::from("both"), HeaderFooterType::Both);
        assert_eq!(HeaderFooterType::from("even"), HeaderFooterType::Even);
        assert_eq!(HeaderFooterType::from("odd"), HeaderFooterType::Odd);
    }
}
