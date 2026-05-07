//! Example: using hwp2md as a library
//!
//! Demonstrates the [`ConvertOptions`] builder API and direct IR manipulation.
//! Run with: `cargo run --example convert`
//!
//! [`ConvertOptions`]: hwp2md::ConvertOptions

use hwp2md::ir::{Block, Document, Inline, Metadata, Section, TableCell, TableRow};
use hwp2md::md::write_markdown;

fn main() {
    // 1. ConvertOptions builder (requires real files at runtime):
    //
    // ```rust,no_run
    // use std::path::Path;
    // use hwp2md::ConvertOptions;
    //
    // // HWP/HWPX → Markdown (direction inferred from extensions)
    // ConvertOptions::new(Path::new("report.hwpx"), Path::new("report.md"))
    //     .frontmatter(true)
    //     .execute()
    //     .expect("conversion failed");
    //
    // // Markdown → HWPX, overwrite if output exists
    // ConvertOptions::new(Path::new("report.md"), Path::new("report.hwpx"))
    //     .force(true)
    //     .execute()
    //     .expect("conversion failed");
    // ```

    // 2. Build a small Document programmatically using the IR types.
    let mut doc = Document::new();
    doc.metadata = Metadata {
        title: Some("hwp2md demo".into()),
        author: Some("hwp2md".into()),
        ..Metadata::default()
    };

    let mut section = Section::default();

    // Heading
    section.blocks.push(Block::Heading {
        level: 1,
        inlines: vec![Inline::plain("hwp2md IR demo")],
    });

    // Paragraph with mixed inline formatting
    section.blocks.push(Block::Paragraph {
        inlines: vec![
            Inline::plain("Normal text, "),
            Inline::bold("bold text"),
            Inline::plain(", and plain again."),
        ],
    });

    // Table (2 columns: header row + one data row)
    section.blocks.push(Block::Table {
        col_count: 2,
        rows: vec![
            TableRow {
                is_header: true,
                cells: vec![
                    TableCell { blocks: vec![Block::Paragraph { inlines: vec![Inline::plain("Format")] }], ..TableCell::default() },
                    TableCell { blocks: vec![Block::Paragraph { inlines: vec![Inline::plain("Extension")] }], ..TableCell::default() },
                ],
            },
            TableRow {
                is_header: false,
                cells: vec![
                    TableCell { blocks: vec![Block::Paragraph { inlines: vec![Inline::plain("HWP 5.0")] }], ..TableCell::default() },
                    TableCell { blocks: vec![Block::Paragraph { inlines: vec![Inline::plain(".hwp")] }], ..TableCell::default() },
                ],
            },
        ],
    });

    doc.sections.push(section);

    // 3. Render to Markdown and print.
    let markdown = write_markdown(&doc, false);
    print!("{markdown}");
}
