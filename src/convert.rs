use anyhow::{bail, Result};
use std::fs;
use std::path::Path;

use crate::hwp;
use crate::hwpx;
use crate::ir;
use crate::md;

pub fn to_markdown(
    input: &Path,
    output: Option<&Path>,
    assets_dir: Option<&Path>,
    frontmatter: bool,
) -> Result<()> {
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
        _ => bail!("Unsupported format: .{ext}. Expected .hwp or .hwpx"),
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

pub fn to_hwpx(input: &Path, output: Option<&Path>, style: Option<&Path>) -> Result<()> {
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext != "md" && ext != "markdown" {
        bail!("Expected .md or .markdown file, got .{ext}");
    }

    let content = fs::read_to_string(input)?;
    let doc = md::parse_markdown(&content);

    let out_path = output.map(|p| p.to_path_buf()).unwrap_or_else(|| {
        input.with_extension("hwpx")
    });

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }

    hwpx::write_hwpx(&doc, &out_path, style)?;
    tracing::info!("Written to {:?}", out_path);

    Ok(())
}

pub fn show_info(input: &Path) -> Result<()> {
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
        _ => bail!("Unsupported format: .{ext}"),
    }

    Ok(())
}

fn print_info(doc: &ir::Document, path: &Path) {
    println!("File: {}", path.display());
    println!("Format: {}", path.extension().and_then(|e| e.to_str()).unwrap_or("unknown"));

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
            inlines.iter().map(|i| i.text.len()).sum()
        }
        ir::Block::CodeBlock { code, .. } => code.len(),
        ir::Block::BlockQuote { blocks } => blocks.iter().map(count_chars).sum(),
        ir::Block::List { items, .. } => items
            .iter()
            .flat_map(|i| &i.blocks)
            .map(count_chars)
            .sum(),
        ir::Block::Table { rows, .. } => rows
            .iter()
            .flat_map(|r| &r.cells)
            .flat_map(|c| &c.blocks)
            .map(count_chars)
            .sum(),
        ir::Block::Math { tex, .. } => tex.len(),
        _ => 0,
    }
}

fn write_assets(doc: &ir::Document, dir: &Path) -> Result<()> {
    if doc.assets.is_empty() {
        return Ok(());
    }

    fs::create_dir_all(dir)?;

    for asset in &doc.assets {
        let path = dir.join(&asset.name);
        fs::write(&path, &asset.data)?;
        tracing::info!("Extracted: {:?}", path);
    }

    Ok(())
}
