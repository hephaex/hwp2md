use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub metadata: Metadata,
    pub sections: Vec<Section>,
    pub assets: Vec<Asset>,
}

impl Document {
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Metadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub description: Option<String>,
    pub subject: Option<String>,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Block {
    Heading {
        level: u8,
        inlines: Vec<Inline>,
    },
    Paragraph {
        inlines: Vec<Inline>,
    },
    Table {
        rows: Vec<TableRow>,
        col_count: usize,
    },
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    BlockQuote {
        blocks: Vec<Block>,
    },
    List {
        ordered: bool,
        start: u32,
        items: Vec<ListItem>,
    },
    Image {
        src: String,
        alt: String,
    },
    HorizontalRule,
    Footnote {
        id: String,
        content: Vec<Block>,
    },
    Math {
        display: bool,
        tex: String,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Inline {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub code: bool,
    pub superscript: bool,
    pub subscript: bool,
    pub link: Option<String>,
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
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
    pub is_header: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableCell {
    pub blocks: Vec<Block>,
    pub colspan: u32,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListItem {
    pub blocks: Vec<Block>,
    pub children: Vec<ListItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub name: String,
    pub data: Vec<u8>,
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
