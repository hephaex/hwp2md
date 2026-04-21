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
