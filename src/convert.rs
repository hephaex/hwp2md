use std::fs;
use std::path::Path;

use crate::error::Hwp2MdError;
use crate::hwp;
use crate::hwpx;
use crate::ir;
use crate::md;

pub fn to_markdown(
    input: &Path,
    output: Option<&Path>,
    assets_dir: Option<&Path>,
    frontmatter: bool,
) -> Result<(), Hwp2MdError> {
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let doc = match ext.as_str() {
        "hwp" => {
            tracing::info!("Parsing HWP 5.0: {:?}", input);
            hwp::read_hwp(input)?
        }
        "hwpx" => {
            tracing::info!("Parsing HWPX: {:?}", input);
            hwpx::read_hwpx(input)?
        }
        _ => {
            return Err(Hwp2MdError::UnsupportedFormat(format!(
                ".{ext}. Expected .hwp or .hwpx"
            )))
        }
    };

    if let Some(dir) = assets_dir {
        write_assets(&doc, dir)?;
    }

    let markdown = md::write_markdown(&doc, frontmatter);

    match output {
        Some(path) => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, &markdown)?;
            tracing::info!("Written to {:?}", path);
        }
        None => {
            print!("{markdown}");
        }
    }

    Ok(())
}

pub fn to_hwpx(
    input: &Path,
    output: Option<&Path>,
    style: Option<&Path>,
) -> Result<(), Hwp2MdError> {
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext != "md" && ext != "markdown" {
        return Err(Hwp2MdError::UnsupportedFormat(format!(
            "Expected .md or .markdown file, got .{ext}"
        )));
    }

    if style.is_some() {
        tracing::warn!("--style option is not yet implemented and will be ignored");
    }

    let content = fs::read_to_string(input)?;
    let doc = md::parse_markdown(&content);

    let out_path = output
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| input.with_extension("hwpx"));

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }

    hwpx::write_hwpx(&doc, &out_path, style)?;
    tracing::info!("Written to {:?}", out_path);

    Ok(())
}

pub fn show_info(input: &Path) -> Result<(), Hwp2MdError> {
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "hwp" => {
            let doc = hwp::read_hwp(input)?;
            print_info(&doc, input);
        }
        "hwpx" => {
            let doc = hwpx::read_hwpx(input)?;
            print_info(&doc, input);
        }
        _ => return Err(Hwp2MdError::UnsupportedFormat(format!(".{ext}"))),
    }

    Ok(())
}

fn print_info(doc: &ir::Document, path: &Path) {
    println!("File: {}", path.display());
    println!(
        "Format: {}",
        path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("unknown")
    );

    if let Some(ref title) = doc.metadata.title {
        println!("Title: {title}");
    }
    if let Some(ref author) = doc.metadata.author {
        println!("Author: {author}");
    }

    println!("Sections: {}", doc.sections.len());

    let block_count: usize = doc.sections.iter().map(|s| s.blocks.len()).sum();
    println!("Blocks: {block_count}");

    let char_count: usize = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .map(count_chars)
        .sum();
    println!("Characters: ~{char_count}");
    println!("Assets: {}", doc.assets.len());
}

fn count_chars(block: &ir::Block) -> usize {
    match block {
        ir::Block::Heading { inlines, .. } | ir::Block::Paragraph { inlines } => {
            inlines.iter().map(|i| i.text.chars().count()).sum()
        }
        ir::Block::CodeBlock { code, .. } => code.chars().count(),
        ir::Block::BlockQuote { blocks } => blocks.iter().map(count_chars).sum(),
        ir::Block::List { items, .. } => {
            items.iter().flat_map(|i| &i.blocks).map(count_chars).sum()
        }
        ir::Block::Table { rows, .. } => rows
            .iter()
            .flat_map(|r| &r.cells)
            .flat_map(|c| &c.blocks)
            .map(count_chars)
            .sum(),
        ir::Block::Math { tex, .. } => tex.chars().count(),
        ir::Block::Footnote { content, .. } => content.iter().map(count_chars).sum(),
        ir::Block::Image { .. } | ir::Block::HorizontalRule => 0,
    }
}

fn write_assets(doc: &ir::Document, dir: &Path) -> Result<(), Hwp2MdError> {
    if doc.assets.is_empty() {
        return Ok(());
    }

    fs::create_dir_all(dir)?;

    for asset in &doc.assets {
        let safe_name = std::path::Path::new(&asset.name)
            .file_name()
            .unwrap_or(std::ffi::OsStr::new("asset"));
        let path = dir.join(safe_name);
        fs::write(&path, &asset.data)?;
        tracing::info!("Extracted: {:?}", path);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Asset, Block, Document, Inline, ListItem, TableCell, TableRow};

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
                },
                ListItem {
                    blocks: vec![para("beta")],
                    children: vec![],
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
            }],
        };
        let block = Block::Footnote {
            id: "fn3".into(),
            content: vec![list],
        };
        assert_eq!(count_chars(&block), 9); // "note item"
    }

    // -----------------------------------------------------------------------
    // write_assets — no-op when doc has no assets
    // -----------------------------------------------------------------------

    #[test]
    fn write_assets_empty_doc_does_nothing() {
        let doc = Document::new();
        let dir = tempfile::tempdir().unwrap();
        // Must succeed and must NOT create any files.
        write_assets(&doc, dir.path()).unwrap();
        let entries: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
        assert!(
            entries.is_empty(),
            "Expected no files extracted for empty doc"
        );
    }

    #[test]
    fn write_assets_creates_dir_and_extracts_files() {
        let mut doc = Document::new();
        doc.assets.push(Asset {
            name: "image.png".into(),
            data: vec![0x89, 0x50, 0x4e, 0x47],
            mime_type: "image/png".into(),
        });
        doc.assets.push(Asset {
            name: "style.css".into(),
            data: b"body{}".to_vec(),
            mime_type: "text/css".into(),
        });

        let dir = tempfile::tempdir().unwrap();
        let assets_dir = dir.path().join("assets");
        // Directory does not exist yet — write_assets must create it.
        assert!(!assets_dir.exists());
        write_assets(&doc, &assets_dir).unwrap();

        assert!(assets_dir.join("image.png").exists());
        assert!(assets_dir.join("style.css").exists());
        assert_eq!(
            std::fs::read(assets_dir.join("style.css")).unwrap(),
            b"body{}"
        );
    }

    #[test]
    fn write_assets_path_traversal_name_is_sanitised() {
        // An asset with a path-traversal name should only use the file-name
        // component, not the directory path prefix.
        let mut doc = Document::new();
        doc.assets.push(Asset {
            name: "../../etc/evil.txt".into(),
            data: b"evil".to_vec(),
            mime_type: "text/plain".into(),
        });

        let dir = tempfile::tempdir().unwrap();
        write_assets(&doc, dir.path()).unwrap();

        // File must land inside the assets dir, not above it.
        assert!(dir.path().join("evil.txt").exists());
        // The parent directory must NOT have been escaped.
        assert!(!dir.path().join("../../etc/evil.txt").exists());
    }

    // -----------------------------------------------------------------------
    // to_hwpx — unsupported extension rejected
    // -----------------------------------------------------------------------

    #[test]
    fn to_hwpx_rejects_non_markdown_extension() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("document.txt");
        std::fs::write(&input, "# Hello\n").unwrap();
        let result = to_hwpx(&input, None, None);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("Expected .md or .markdown"),
            "unexpected error message: {msg}"
        );
    }

    // -----------------------------------------------------------------------
    // to_markdown — unsupported extension rejected
    // -----------------------------------------------------------------------

    #[test]
    fn to_markdown_rejects_unknown_extension() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("document.docx");
        std::fs::write(&input, b"placeholder").unwrap();
        let result = to_markdown(&input, None, None, false);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("Unsupported format"),
            "unexpected error message: {msg}"
        );
    }

    // -----------------------------------------------------------------------
    // to_markdown — markdown output to file
    // -----------------------------------------------------------------------

    #[test]
    fn to_markdown_md_input_to_output_file_via_hwpx_roundtrip() {
        // We cannot round-trip a full HWP binary here, but we can verify
        // that to_hwpx writes a file and to_markdown on a .hwpx file is
        // rejected for an unknown extension (coverage smoke test).
        let dir = tempfile::tempdir().unwrap();
        let md_in = dir.path().join("input.md");
        std::fs::write(&md_in, "# Title\n\nBody text.\n").unwrap();
        let hwpx_out = dir.path().join("output.hwpx");

        // to_hwpx must succeed for a valid .md file.
        to_hwpx(&md_in, Some(&hwpx_out), None).unwrap();
        assert!(hwpx_out.exists(), "hwpx output file not created");

        // to_markdown on the resulting hwpx must succeed.
        let md_out = dir.path().join("result.md");
        to_markdown(&hwpx_out, Some(&md_out), None, false).unwrap();
        assert!(md_out.exists(), "markdown output file not created");

        let content = std::fs::read_to_string(&md_out).unwrap();
        assert!(
            content.contains("Title"),
            "heading lost in hwpx roundtrip; got: {content:?}"
        );
    }

    // -----------------------------------------------------------------------
    // show_info — unsupported extension rejected
    // -----------------------------------------------------------------------

    #[test]
    fn show_info_rejects_unknown_extension() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("document.pdf");
        std::fs::write(&input, b"fake-pdf").unwrap();
        let result = show_info(&input);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("Unsupported format"),
            "unexpected error message: {msg}"
        );
    }
}
