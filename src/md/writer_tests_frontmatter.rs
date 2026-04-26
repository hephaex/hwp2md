use super::*;
use crate::ir;

// -----------------------------------------------------------------------
// Shared helpers (re-declared here for standalone module use)
// -----------------------------------------------------------------------

fn plain(t: &str) -> ir::Inline {
    ir::Inline::plain(t)
}

// -----------------------------------------------------------------------
// write_markdown — frontmatter
// -----------------------------------------------------------------------

#[test]
fn write_markdown_frontmatter() {
    let mut doc = ir::Document::new();
    doc.metadata.title = Some("My Title".into());
    doc.metadata.author = Some("Author Name".into());
    doc.sections.push(ir::Section {
        blocks: Vec::new(),
        page_layout: None,
    });
    let md = write_markdown(&doc, true);
    assert!(md.starts_with("---\n"), "got: {md}");
    assert!(md.contains("title: \"My Title\""), "got: {md}");
    assert!(md.contains("author: \"Author Name\""), "got: {md}");
}

#[test]
fn write_markdown_multi_section_separator() {
    let mut doc = ir::Document::new();
    doc.sections.push(ir::Section {
        blocks: vec![ir::Block::Paragraph {
            inlines: vec![plain("Section 1")],
        }],

        page_layout: None,
    });
    doc.sections.push(ir::Section {
        blocks: vec![ir::Block::Paragraph {
            inlines: vec![plain("Section 2")],
        }],

        page_layout: None,
    });
    let md = write_markdown(&doc, false);
    assert!(md.contains("\n---\n"), "got: {md}");
    assert!(md.contains("Section 1"), "got: {md}");
    assert!(md.contains("Section 2"), "got: {md}");
}

#[test]
fn write_markdown_frontmatter_with_created_date() {
    let mut doc = ir::Document::new();
    doc.metadata.title = Some("Doc".into());
    doc.metadata.created = Some("2026-04-22".into());
    doc.sections.push(ir::Section {
        blocks: Vec::new(),
        page_layout: None,
    });
    let md = write_markdown(&doc, true);
    assert!(md.contains("date: \"2026-04-22\""), "got: {md}");
}

#[test]
fn write_markdown_frontmatter_with_subject() {
    let mut doc = ir::Document::new();
    doc.metadata.subject = Some("The subject".into());
    doc.sections.push(ir::Section {
        blocks: Vec::new(),
        page_layout: None,
    });
    let md = write_markdown(&doc, true);
    assert!(md.contains("subject: \"The subject\""), "got: {md}");
}

#[test]
fn write_markdown_frontmatter_with_description() {
    let mut doc = ir::Document::new();
    doc.metadata.description = Some("A description".into());
    doc.sections.push(ir::Section {
        blocks: Vec::new(),
        page_layout: None,
    });
    let md = write_markdown(&doc, true);
    assert!(md.contains("description: \"A description\""), "got: {md}");
}

#[test]
fn write_markdown_frontmatter_with_keywords() {
    let mut doc = ir::Document::new();
    doc.metadata.keywords = vec!["rust".into(), "hwp".into(), "converter".into()];
    doc.sections.push(ir::Section {
        blocks: Vec::new(),
        page_layout: None,
    });
    let md = write_markdown(&doc, true);
    assert!(md.contains("keywords:"), "got: {md}");
    assert!(md.contains("rust"), "got: {md}");
    assert!(md.contains("hwp"), "got: {md}");
    assert!(md.contains("converter"), "got: {md}");
}

#[test]
fn write_markdown_frontmatter_all_fields() {
    let mut doc = ir::Document::new();
    doc.metadata.title = Some("Full".into());
    doc.metadata.author = Some("Author".into());
    doc.metadata.created = Some("2026-01-01".into());
    doc.metadata.subject = Some("Subj".into());
    doc.metadata.description = Some("Desc".into());
    doc.metadata.keywords = vec!["a".into(), "b".into()];
    doc.sections.push(ir::Section {
        blocks: Vec::new(),
        page_layout: None,
    });
    let md = write_markdown(&doc, true);
    assert!(md.starts_with("---\n"), "got: {md}");
    assert!(md.contains("title:"), "got: {md}");
    assert!(md.contains("author:"), "got: {md}");
    assert!(md.contains("date:"), "got: {md}");
    assert!(md.contains("subject:"), "got: {md}");
    assert!(md.contains("description:"), "got: {md}");
    assert!(md.contains("keywords:"), "got: {md}");
}

#[test]
fn write_markdown_frontmatter_no_fields_emits_empty_block() {
    let mut doc = ir::Document::new();
    // No metadata fields set.
    doc.sections.push(ir::Section {
        blocks: Vec::new(),
        page_layout: None,
    });
    let md = write_markdown(&doc, true);
    // Should start with ---\n and end with ---\n\n even with no fields.
    assert!(md.starts_with("---\n---\n"), "got: {md}");
}

// -----------------------------------------------------------------------
// L2 fix: inline math block must emit a trailing blank line
// -----------------------------------------------------------------------

#[test]
fn write_markdown_inline_math_has_trailing_blank_line() {
    let doc = ir::Document {
        sections: vec![ir::Section {
            blocks: vec![ir::Block::Math {
                display: false,
                tex: "x+y".into(),
            }],

            page_layout: None,
        }],
        ..ir::Document::new()
    };
    let md = write_markdown(&doc, false);
    // Must end with two newlines so the next block is separated.
    assert!(
        md.contains("$x+y$\n\n"),
        "inline math must be followed by a blank line; got: {md:?}"
    );
}

#[test]
fn write_markdown_inline_math_followed_by_paragraph_has_blank_line() {
    let doc = ir::Document {
        sections: vec![ir::Section {
            blocks: vec![
                ir::Block::Math {
                    display: false,
                    tex: "a^2".into(),
                },
                ir::Block::Paragraph {
                    inlines: vec![plain("next paragraph")],
                },
            ],

            page_layout: None,
        }],
        ..ir::Document::new()
    };
    let md = write_markdown(&doc, false);
    // There must be a blank line between the math block and the paragraph.
    assert!(
        md.contains("$a^2$\n\nnext paragraph"),
        "inline math and following paragraph must be separated by a blank line; got: {md:?}"
    );
}
