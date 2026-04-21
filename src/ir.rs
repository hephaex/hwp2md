use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub metadata: Metadata,
    pub sections: Vec<Section>,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Metadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Block {
    Heading { level: u8, text: Vec<Inline> },
    Paragraph { text: Vec<Inline> },
    Table { rows: Vec<TableRow> },
    CodeBlock { language: Option<String>, code: String },
    BlockQuote { blocks: Vec<Block> },
    List { ordered: bool, items: Vec<ListItem> },
    Image { src: String, alt: String },
    HorizontalRule,
    Footnote { id: String, content: Vec<Block> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inline {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub strikethrough: bool,
    pub code: bool,
    pub link: Option<String>,
    pub superscript: bool,
    pub subscript: bool,
}

impl Default for Inline {
    fn default() -> Self {
        Self {
            text: String::new(),
            bold: false,
            italic: false,
            strikethrough: false,
            code: false,
            link: None,
            superscript: false,
            subscript: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableCell {
    pub content: Vec<Inline>,
    pub colspan: u32,
    pub rowspan: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListItem {
    pub content: Vec<Inline>,
    pub children: Vec<ListItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub name: String,
    pub data: Vec<u8>,
    pub mime_type: String,
}
