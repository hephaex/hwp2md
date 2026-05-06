// Shared test helpers for hwp2md integration tests.
// ---------------------------------------------------------------------------
// CLI helpers — used by cli.rs and cli_batch.rs
// ---------------------------------------------------------------------------

/// Returns a [`std::process::Command`] pointing at the compiled `hwp2md` binary.
#[allow(dead_code)]
pub fn cargo_bin() -> std::process::Command {
    std::process::Command::new(env!("CARGO_BIN_EXE_hwp2md"))
}

/// Produces a valid HWPX file at `path` by running `to-hwpx` on a minimal
/// Markdown source written to the same parent directory.
#[allow(dead_code)]
pub fn make_hwpx(path: &std::path::Path) {
    let dir = path.parent().expect("test path must have parent");
    let md_src = dir.join("_tmp_src.md");
    std::fs::write(&md_src, "# Batch Test\n\nContent.\n").expect("write md src");
    let result = cargo_bin()
        .args([
            "to-hwpx",
            md_src.to_str().expect("md source path must be valid UTF-8"),
            "--output",
            path.to_str().expect("output path must be valid UTF-8"),
        ])
        .output()
        .expect("run to-hwpx");
    assert!(
        result.status.success(),
        "make_hwpx failed; stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    std::fs::remove_file(&md_src).ok();
}

// ---------------------------------------------------------------------------
// IR helpers — used by roundtrip.rs and roundtrip_stability.rs
// ---------------------------------------------------------------------------

/// Creates a plain (unstyled) [`hwp2md::ir::Inline`] from a string slice.
#[allow(dead_code)]
pub fn plain(t: &str) -> hwp2md::ir::Inline {
    hwp2md::ir::Inline::plain(t)
}

/// Wraps `blocks` in a [`hwp2md::ir::Document`] containing a single
/// [`hwp2md::ir::Section`].
#[allow(dead_code)]
pub fn make_doc(blocks: Vec<hwp2md::ir::Block>) -> hwp2md::ir::Document {
    let mut doc = hwp2md::ir::Document::new();
    doc.sections.push(hwp2md::ir::Section {
        blocks,
        page_layout: None,
        ..Default::default()
    });
    doc
}

/// Returns the block slice from the first section of `doc`, or an empty
/// slice if the document has no sections.
#[allow(dead_code)]
pub fn first_blocks(doc: &hwp2md::ir::Document) -> &[hwp2md::ir::Block] {
    doc.sections
        .first()
        .map(|s| s.blocks.as_slice())
        .unwrap_or(&[])
}

// ---------------------------------------------------------------------------
// HWPX pipeline helpers — used by hwpx_roundtrip.rs, hwpx_roundtrip_full.rs,
// and hwpx_roundtrip_full2.rs
// ---------------------------------------------------------------------------

/// Collects all text content from a block slice, recursing into nested
/// structures (tables, lists, blockquotes, footnotes).
#[allow(dead_code)]
pub fn collect_all_text(blocks: &[hwp2md::ir::Block]) -> String {
    use hwp2md::ir::Block;
    let mut out = String::new();
    for block in blocks {
        match block {
            Block::Heading { inlines, .. } | Block::Paragraph { inlines } => {
                for i in inlines {
                    out.push_str(&i.text);
                }
            }
            Block::CodeBlock { code, .. } => {
                out.push_str(code);
            }
            Block::Table { rows, .. } => {
                for row in rows {
                    for cell in &row.cells {
                        out.push_str(&collect_all_text(&cell.blocks));
                    }
                }
            }
            Block::BlockQuote { blocks }
            | Block::Footnote {
                content: blocks, ..
            } => {
                out.push_str(&collect_all_text(blocks));
            }
            Block::List { items, .. } => {
                for item in items {
                    out.push_str(&collect_all_text(&item.blocks));
                }
            }
            _ => {}
        }
    }
    out
}

/// Pipeline (A): MD → HWPX → IR.
///
/// Parses `markdown`, writes it to a temporary HWPX file, then reads it
/// back and returns the resulting [`hwp2md::ir::Document`].
#[allow(dead_code)]
pub fn md_to_hwpx_to_ir(markdown: &str) -> hwp2md::ir::Document {
    use hwp2md::hwpx::{read_hwpx, write_hwpx};
    use hwp2md::md::parse_markdown;
    let doc = parse_markdown(markdown);
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");
    read_hwpx(tmp.path()).expect("read_hwpx")
}

/// Pipeline (B): MD → HWPX → IR → MD.
///
/// Returns the final Markdown string after the full roundtrip.
#[allow(dead_code)]
pub fn md_to_hwpx_to_md(markdown: &str) -> String {
    use hwp2md::md::write_markdown;
    let doc = md_to_hwpx_to_ir(markdown);
    write_markdown(&doc, false)
}

/// Pipeline (B) starting from a hand-built IR document: IR → HWPX → IR → MD.
#[allow(dead_code)]
pub fn ir_to_hwpx_to_md(doc: &hwp2md::ir::Document) -> String {
    use hwp2md::hwpx::{read_hwpx, write_hwpx};
    use hwp2md::md::write_markdown;
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    write_hwpx(doc, tmp.path(), None).expect("write_hwpx");
    let doc2 = read_hwpx(tmp.path()).expect("read_hwpx");
    write_markdown(&doc2, false)
}
