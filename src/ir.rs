//! Intermediate representation (IR) types shared by all readers and writers.

use serde::{Deserialize, Serialize};

/// Top-level document produced by any reader and consumed by any writer.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// A logical section of a document containing an ordered sequence of blocks.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Section {
    /// Content blocks in reading order.
    pub blocks: Vec<Block>,
    /// Page layout for this section, parsed from `<hp:secPr>` in HWPX.
    ///
    /// `None` when the source document did not include section properties
    /// (e.g. plain Markdown input or very minimal HWPX files).  Writers
    /// should fall back to [`PageLayout::a4_portrait`] defaults in that case.
    pub page_layout: Option<PageLayout>,
}

/// A block-level content element within a section.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// A run of inline text with optional formatting and annotations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    /// Font name resolved from the DocInfo face_names table.
    /// Not rendered in Markdown output; preserved for HWPX round-trip fidelity.
    pub font_name: Option<String>,
    /// Ruby annotation text.  When `Some`, the inline's `text` is the base
    /// character(s) and this field holds the small annotation above them.
    /// Rendered as `<ruby>base<rt>annotation</rt></ruby>` in Markdown output.
    pub ruby: Option<String>,
}

impl Inline {
    /// Create a plain, unformatted inline run from `text`.
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }

    /// Create a bold inline run from `text`.
    pub fn bold(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            bold: true,
            ..Self::default()
        }
    }

    /// Construct an inline with all formatting fields set explicitly.
    ///
    /// This avoids the `..Default::default()` pattern which silently drops
    /// newly-added fields.  The `link`, `footnote_ref`, `font_name`, `code`,
    /// and `ruby` fields are left at their defaults and can be set via chained
    /// builder methods (`with_ruby`, etc.).
    #[allow(clippy::too_many_arguments)]
    pub fn with_formatting(
        text: String,
        bold: bool,
        italic: bool,
        underline: bool,
        strikethrough: bool,
        superscript: bool,
        subscript: bool,
        color: Option<String>,
    ) -> Self {
        Self {
            text,
            bold,
            italic,
            underline,
            strikethrough,
            superscript,
            subscript,
            color,
            code: false,
            link: None,
            footnote_ref: None,
            font_name: None,
            ruby: None,
        }
    }

    /// Set the `link` field, returning the modified inline.
    pub fn with_link(mut self, link: Option<String>) -> Self {
        self.link = link;
        self
    }

    /// Set the `ruby` annotation field, returning the modified inline.
    pub fn with_ruby(mut self, ruby: Option<String>) -> Self {
        self.ruby = ruby;
        self
    }

    /// Set the `font_name` field, returning the modified inline.
    pub fn with_font_name(mut self, font_name: Option<String>) -> Self {
        self.font_name = font_name;
        self
    }

    /// Create a footnote-reference inline with no visible text or formatting.
    pub fn footnote_ref(id: impl Into<String>) -> Self {
        Self {
            footnote_ref: Some(id.into()),
            ..Self::default()
        }
    }
}

/// A single row in a [`Block::Table`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRow {
    /// Cells in left-to-right order.
    pub cells: Vec<TableCell>,
    /// `true` when this row should be treated as a header row.
    pub is_header: bool,
}

/// A single cell in a [`TableRow`].
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListItem {
    /// Block content of this item (typically one `Paragraph`).
    pub blocks: Vec<Block>,
    /// Nested child list items for multi-level lists.
    pub children: Vec<ListItem>,
}

/// A binary asset (image, font, …) embedded in the document.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
        let a = Document::new();
        let b = Document::default();
        // Both must be empty — compare structurally via their serialized form.
        assert_eq!(a.sections.len(), b.sections.len());
        assert_eq!(a.assets.len(), b.assets.len());
    }
}
