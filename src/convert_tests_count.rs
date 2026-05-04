use super::*;
use crate::ir::{Block, Inline, ListItem, TableCell, TableRow};

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

fn plain(t: &str) -> Inline {
    Inline::plain(t)
}

fn para(text: &str) -> Block {
    Block::Paragraph {
        inlines: vec![plain(text)],
    }
}

// -----------------------------------------------------------------------
// count_chars — Paragraph
// -----------------------------------------------------------------------

#[test]
fn count_chars_empty_paragraph_returns_zero() {
    let block = Block::Paragraph { inlines: vec![] };
    assert_eq!(count_chars(&block), 0);
}

#[test]
fn count_chars_paragraph_single_inline() {
    let block = Block::Paragraph {
        inlines: vec![plain("hello")],
    };
    assert_eq!(count_chars(&block), 5);
}

#[test]
fn count_chars_paragraph_multiple_inlines() {
    let block = Block::Paragraph {
        inlines: vec![plain("foo"), plain("bar")],
    };
    assert_eq!(count_chars(&block), 6);
}

#[test]
fn count_chars_paragraph_multibyte_chars() {
    // Korean characters: each is one char but multiple bytes.
    let block = Block::Paragraph {
        inlines: vec![plain("안녕하세요")],
    };
    assert_eq!(count_chars(&block), 5);
}

// -----------------------------------------------------------------------
// count_chars — Heading
// -----------------------------------------------------------------------

#[test]
fn count_chars_heading_counts_inline_chars() {
    let block = Block::Heading {
        level: 1,
        inlines: vec![plain("Introduction")],
    };
    assert_eq!(count_chars(&block), 12);
}

#[test]
fn count_chars_heading_empty_returns_zero() {
    let block = Block::Heading {
        level: 2,
        inlines: vec![],
    };
    assert_eq!(count_chars(&block), 0);
}

// -----------------------------------------------------------------------
// count_chars — CodeBlock
// -----------------------------------------------------------------------

#[test]
fn count_chars_code_block_counts_chars() {
    let block = Block::CodeBlock {
        language: Some("rust".into()),
        code: "let x = 1;".into(),
    };
    assert_eq!(count_chars(&block), 10);
}

#[test]
fn count_chars_code_block_empty_returns_zero() {
    let block = Block::CodeBlock {
        language: None,
        code: String::new(),
    };
    assert_eq!(count_chars(&block), 0);
}

// -----------------------------------------------------------------------
// count_chars — Math
// -----------------------------------------------------------------------

#[test]
fn count_chars_math_counts_latex_chars() {
    let block = Block::Math {
        display: true,
        tex: r"\frac{a}{b}".into(),
    };
    assert_eq!(count_chars(&block), r"\frac{a}{b}".len());
}

#[test]
fn count_chars_math_empty_returns_zero() {
    let block = Block::Math {
        display: false,
        tex: String::new(),
    };
    assert_eq!(count_chars(&block), 0);
}

// -----------------------------------------------------------------------
// count_chars — Table
// -----------------------------------------------------------------------

#[test]
fn count_chars_table_sums_all_cells() {
    let rows = vec![
        TableRow {
            cells: vec![
                TableCell {
                    blocks: vec![para("Col1")],
                    ..Default::default()
                },
                TableCell {
                    blocks: vec![para("Col2")],
                    ..Default::default()
                },
            ],
            is_header: true,
        },
        TableRow {
            cells: vec![
                TableCell {
                    blocks: vec![para("val")],
                    ..Default::default()
                },
                TableCell {
                    blocks: vec![para("x")],
                    ..Default::default()
                },
            ],
            is_header: false,
        },
    ];
    let block = Block::Table { rows, col_count: 2 };
    // "Col1"=4, "Col2"=4, "val"=3, "x"=1 → 12
    assert_eq!(count_chars(&block), 12);
}

#[test]
fn count_chars_table_empty_returns_zero() {
    let block = Block::Table {
        rows: vec![],
        col_count: 0,
    };
    assert_eq!(count_chars(&block), 0);
}

// -----------------------------------------------------------------------
// count_chars — List
// -----------------------------------------------------------------------

#[test]
fn count_chars_list_sums_item_blocks() {
    let block = Block::List {
        ordered: false,
        start: 1,
        items: vec![
            ListItem {
                blocks: vec![para("alpha")],
                children: vec![],
                checked: None,
            },
            ListItem {
                blocks: vec![para("beta")],
                children: vec![],
                checked: None,
            },
        ],
    };
    // "alpha"=5, "beta"=4 → 9
    assert_eq!(count_chars(&block), 9);
}

#[test]
fn count_chars_list_empty_returns_zero() {
    let block = Block::List {
        ordered: true,
        start: 1,
        items: vec![],
    };
    assert_eq!(count_chars(&block), 0);
}

// -----------------------------------------------------------------------
// count_chars — BlockQuote
// -----------------------------------------------------------------------

#[test]
fn count_chars_blockquote_recurses_into_blocks() {
    let block = Block::BlockQuote {
        blocks: vec![para("quoted text")],
    };
    assert_eq!(count_chars(&block), 11);
}

// -----------------------------------------------------------------------
// count_chars — Footnote (was previously caught by `_ => 0`)
// -----------------------------------------------------------------------

#[test]
fn count_chars_footnote_recurses_into_content() {
    let block = Block::Footnote {
        id: "fn1".into(),
        content: vec![para("footnote body")],
    };
    // "footnote body" = 13
    assert_eq!(count_chars(&block), 13);
}

#[test]
fn count_chars_footnote_empty_content_returns_zero() {
    let block = Block::Footnote {
        id: "fn2".into(),
        content: vec![],
    };
    assert_eq!(count_chars(&block), 0);
}

// -----------------------------------------------------------------------
// count_chars — Image and HorizontalRule (zero-char blocks)
// -----------------------------------------------------------------------

#[test]
fn count_chars_image_returns_zero() {
    let block = Block::Image {
        src: "photo.png".into(),
        alt: "a photo".into(),
    };
    assert_eq!(count_chars(&block), 0);
}

#[test]
fn count_chars_horizontal_rule_returns_zero() {
    assert_eq!(count_chars(&Block::HorizontalRule), 0);
}

// -----------------------------------------------------------------------
// count_chars — nested structures
// -----------------------------------------------------------------------

#[test]
fn count_chars_table_inside_blockquote() {
    let cell_block = para("cell");
    let rows = vec![TableRow {
        cells: vec![TableCell {
            blocks: vec![cell_block],
            ..Default::default()
        }],
        is_header: false,
    }];
    let table = Block::Table { rows, col_count: 1 };
    let block = Block::BlockQuote {
        blocks: vec![table],
    };
    assert_eq!(count_chars(&block), 4); // "cell"
}

#[test]
fn count_chars_footnote_with_nested_list() {
    let list = Block::List {
        ordered: false,
        start: 1,
        items: vec![ListItem {
            blocks: vec![para("note item")],
            children: vec![],
            checked: None,
        }],
    };
    let block = Block::Footnote {
        id: "fn3".into(),
        content: vec![list],
    };
    assert_eq!(count_chars(&block), 9); // "note item"
}
