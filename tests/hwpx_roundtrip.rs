/// HWPX structural roundtrip tests.
///
/// Each test follows the pipeline:
///   Markdown -> IR -> HWPX bytes -> read HWPX -> IR -> verify structure
///
/// These tests verify that the IR blocks and inlines survive a full
/// HWPX write-then-read cycle.
use hwp2md::ir;
use hwp2md::hwpx::{read_hwpx, write_hwpx};
use hwp2md::md::parse_markdown;

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

fn md_to_hwpx_to_ir(markdown: &str) -> ir::Document {
    let doc = parse_markdown(markdown);
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");
    read_hwpx(tmp.path()).expect("read_hwpx")
}

fn first_blocks(doc: &ir::Document) -> &[ir::Block] {
    doc.sections
        .first()
        .map(|s| s.blocks.as_slice())
        .unwrap_or(&[])
}

fn collect_all_text(blocks: &[ir::Block]) -> String {
    let mut out = String::new();
    for block in blocks {
        match block {
            ir::Block::Heading { inlines, .. } | ir::Block::Paragraph { inlines } => {
                for i in inlines {
                    out.push_str(&i.text);
                }
            }
            ir::Block::CodeBlock { code, .. } => {
                out.push_str(code);
            }
            ir::Block::Table { rows, .. } => {
                for row in rows {
                    for cell in &row.cells {
                        out.push_str(&collect_all_text(&cell.blocks));
                    }
                }
            }
            ir::Block::BlockQuote { blocks } | ir::Block::Footnote { content: blocks, .. } => {
                out.push_str(&collect_all_text(blocks));
            }
            ir::Block::List { items, .. } => {
                for item in items {
                    out.push_str(&collect_all_text(&item.blocks));
                }
            }
            _ => {}
        }
    }
    out
}

// -----------------------------------------------------------------------
// Test 1: Simple paragraph text roundtrip
// -----------------------------------------------------------------------

#[test]
fn hwpx_roundtrip_simple_paragraph() {
    let md = "Hello, this is a simple paragraph.\n";
    let doc = md_to_hwpx_to_ir(md);

    let blocks = first_blocks(&doc);
    assert!(
        !blocks.is_empty(),
        "at least one block expected after roundtrip"
    );

    let text = collect_all_text(blocks);
    assert!(
        text.contains("Hello, this is a simple paragraph"),
        "paragraph text must survive roundtrip; got: {text:?}"
    );
}

#[test]
fn hwpx_roundtrip_multiple_paragraphs() {
    let md = "First paragraph.\n\nSecond paragraph.\n";
    let doc = md_to_hwpx_to_ir(md);

    let text = collect_all_text(first_blocks(&doc));
    assert!(
        text.contains("First paragraph"),
        "first paragraph text: {text:?}"
    );
    assert!(
        text.contains("Second paragraph"),
        "second paragraph text: {text:?}"
    );
}

// -----------------------------------------------------------------------
// Test 2: Heading levels roundtrip
// -----------------------------------------------------------------------

#[test]
fn hwpx_roundtrip_heading_level_1() {
    let md = "# Main Title\n\nBody text.\n";
    let doc = md_to_hwpx_to_ir(md);
    let blocks = first_blocks(&doc);

    let text = collect_all_text(blocks);
    assert!(
        text.contains("Main Title"),
        "heading text must survive: {text:?}"
    );
    // The text must appear somewhere in the blocks; the reader may
    // reconstruct it as a Heading or a styled Paragraph.
    assert!(
        text.contains("Body text"),
        "body text must survive: {text:?}"
    );
}

#[test]
fn hwpx_roundtrip_heading_level_3() {
    let md = "### Subsection\n\nContent under subsection.\n";
    let doc = md_to_hwpx_to_ir(md);
    let text = collect_all_text(first_blocks(&doc));

    assert!(
        text.contains("Subsection"),
        "h3 text must survive: {text:?}"
    );
    assert!(
        text.contains("Content under subsection"),
        "body text: {text:?}"
    );
}

// -----------------------------------------------------------------------
// Test 3: Bold/italic formatting roundtrip
// -----------------------------------------------------------------------

#[test]
fn hwpx_roundtrip_bold_italic_formatting() {
    let md = "This has **bold** and *italic* words.\n";
    let doc = md_to_hwpx_to_ir(md);

    let text = collect_all_text(first_blocks(&doc));
    assert!(
        text.contains("bold"),
        "bold text must survive: {text:?}"
    );
    assert!(
        text.contains("italic"),
        "italic text must survive: {text:?}"
    );
    assert!(
        text.contains("This has"),
        "surrounding text must survive: {text:?}"
    );
}

// -----------------------------------------------------------------------
// Additional structural roundtrip tests
// -----------------------------------------------------------------------

#[test]
fn hwpx_roundtrip_code_block() {
    let md = "```rust\nfn main() {}\n```\n";
    let doc = md_to_hwpx_to_ir(md);

    let text = collect_all_text(first_blocks(&doc));
    assert!(
        text.contains("fn main()"),
        "code block text must survive: {text:?}"
    );
}

#[test]
fn hwpx_roundtrip_mixed_document() {
    let md = "\
# Document Title

A paragraph with **bold** text.

## Section Two

Another paragraph here.
";
    let doc = md_to_hwpx_to_ir(md);

    let text = collect_all_text(first_blocks(&doc));
    assert!(
        text.contains("Document Title"),
        "h1: {text:?}"
    );
    assert!(
        text.contains("bold"),
        "bold inline: {text:?}"
    );
    assert!(
        text.contains("Section Two"),
        "h2: {text:?}"
    );
    assert!(
        text.contains("Another paragraph"),
        "body: {text:?}"
    );
}

#[test]
fn hwpx_roundtrip_preserves_section_count() {
    // A single-section markdown document should produce exactly one section.
    let md = "Paragraph one.\n\nParagraph two.\n";
    let doc = md_to_hwpx_to_ir(md);

    assert_eq!(
        doc.sections.len(),
        1,
        "should have exactly one section after roundtrip"
    );
}

// -----------------------------------------------------------------------
// Test: Hyperlink roundtrip
// -----------------------------------------------------------------------

#[test]
fn hwpx_roundtrip_hyperlink_preserves_url_and_text() {
    // Build IR directly with a linked inline so we control the exact URL
    // and text without relying on the Markdown parser.
    let linked_inline = ir::Inline {
        text: "Click here".into(),
        link: Some("https://example.com".into()),
        ..ir::Inline::default()
    };
    let doc = ir::Document {
        metadata: ir::Metadata::default(),
        sections: vec![ir::Section {
            blocks: vec![ir::Block::Paragraph {
                inlines: vec![linked_inline],
            }],
        }],
        assets: Vec::new(),
    };

    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");
    let read_back = read_hwpx(tmp.path()).expect("read_hwpx");

    let blocks = read_back
        .sections
        .first()
        .map(|s| s.blocks.as_slice())
        .unwrap_or(&[]);

    let inlines = match blocks.first() {
        Some(ir::Block::Paragraph { inlines }) => inlines,
        other => panic!("expected Paragraph as first block, got: {other:?}"),
    };

    assert!(
        !inlines.is_empty(),
        "linked paragraph must contain at least one inline after roundtrip"
    );

    let linked = inlines
        .iter()
        .find(|i| i.link.is_some())
        .expect("at least one inline must carry a link after roundtrip");

    assert_eq!(
        linked.text, "Click here",
        "link text must survive HWPX roundtrip"
    );
    assert_eq!(
        linked.link.as_deref(),
        Some("https://example.com"),
        "link URL must survive HWPX roundtrip"
    );
}
