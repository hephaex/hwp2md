use super::*;
use crate::ir;

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

pub(super) fn first_section_blocks(doc: &ir::Document) -> &[ir::Block] {
    doc.sections.first().map_or(&[], |s| s.blocks.as_slice())
}

// -----------------------------------------------------------------------
// extract_frontmatter
// -----------------------------------------------------------------------

#[test]
fn extract_frontmatter_valid() {
    let md = "---\ntitle: \"Hello\"\nauthor: \"Alice\"\ndate: \"2026-01-01\"\n---\n\nBody";
    let meta = extract_frontmatter(md);
    assert_eq!(meta.title.as_deref(), Some("Hello"));
    assert_eq!(meta.author.as_deref(), Some("Alice"));
    assert_eq!(meta.created.as_deref(), Some("2026-01-01"));
}

#[test]
fn extract_frontmatter_no_frontmatter() {
    let meta = extract_frontmatter("# Just a heading\n\nSome text.");
    assert!(meta.title.is_none());
    assert!(meta.author.is_none());
}

#[test]
fn extract_frontmatter_incomplete_no_closing_delimiter() {
    // Opening --- exists but no closing --- → treated as no frontmatter.
    let meta = extract_frontmatter("---\ntitle: \"Oops\"\nno closing fence");
    assert!(meta.title.is_none());
}

// -----------------------------------------------------------------------
// parse_markdown — headings
// -----------------------------------------------------------------------

#[test]
fn parse_markdown_heading_level_1() {
    let doc = parse_markdown("# Hello World\n");
    let blocks = first_section_blocks(&doc);
    assert_eq!(blocks.len(), 1);
    if let ir::Block::Heading { level, inlines } = &blocks[0] {
        assert_eq!(*level, 1);
        assert!(!inlines.is_empty());
        assert_eq!(inlines[0].text, "Hello World");
    } else {
        panic!("expected Heading, got {:?}", blocks[0]);
    }
}

#[test]
fn parse_markdown_heading_level_3() {
    let doc = parse_markdown("### Deep\n");
    let blocks = first_section_blocks(&doc);
    if let ir::Block::Heading { level, .. } = &blocks[0] {
        assert_eq!(*level, 3);
    } else {
        panic!("expected Heading");
    }
}

// -----------------------------------------------------------------------
// parse_markdown — paragraph
// -----------------------------------------------------------------------

#[test]
fn parse_markdown_paragraph() {
    let doc = parse_markdown("Hello, parser!\n");
    let blocks = first_section_blocks(&doc);
    if let ir::Block::Paragraph { inlines } = &blocks[0] {
        let text: String = inlines.iter().map(|i| i.text.as_str()).collect();
        assert!(text.contains("Hello, parser!"));
    } else {
        panic!("expected Paragraph, got {:?}", blocks[0]);
    }
}

// -----------------------------------------------------------------------
// parse_markdown — inline styles
// -----------------------------------------------------------------------

#[test]
fn parse_markdown_bold() {
    let doc = parse_markdown("**bold text**\n");
    let blocks = first_section_blocks(&doc);
    if let ir::Block::Paragraph { inlines } = &blocks[0] {
        let bold_inline = inlines.iter().find(|i| i.bold);
        assert!(
            bold_inline.is_some(),
            "no bold inline found; inlines: {inlines:?}"
        );
    } else {
        panic!("expected Paragraph");
    }
}

#[test]
fn parse_markdown_italic() {
    let doc = parse_markdown("*italic text*\n");
    let blocks = first_section_blocks(&doc);
    if let ir::Block::Paragraph { inlines } = &blocks[0] {
        let italic = inlines.iter().find(|i| i.italic);
        assert!(
            italic.is_some(),
            "no italic inline found; inlines: {inlines:?}"
        );
    } else {
        panic!("expected Paragraph");
    }
}

// -----------------------------------------------------------------------
// parse_markdown — code block
// -----------------------------------------------------------------------

#[test]
fn parse_markdown_code_block() {
    let doc = parse_markdown("```rust\nfn main() {}\n```\n");
    let blocks = first_section_blocks(&doc);
    if let ir::Block::CodeBlock { language, code } = &blocks[0] {
        assert_eq!(language.as_deref(), Some("rust"));
        assert!(code.contains("fn main()"), "code: {code}");
    } else {
        panic!("expected CodeBlock, got {:?}", blocks[0]);
    }
}

// -----------------------------------------------------------------------
// parse_markdown — table
// -----------------------------------------------------------------------

#[test]
fn parse_markdown_table() {
    let md = "| A | B |\n| --- | --- |\n| 1 | 2 |\n";
    let doc = parse_markdown(md);
    let blocks = first_section_blocks(&doc);
    if let ir::Block::Table { rows, col_count, .. } = &blocks[0] {
        assert_eq!(*col_count, 2, "col_count: {col_count}");
        assert!(!rows.is_empty());
    } else {
        panic!("expected Table, got {:?}", blocks[0]);
    }
}

// -----------------------------------------------------------------------
// parse_markdown — list
// -----------------------------------------------------------------------

#[test]
fn parse_markdown_unordered_list() {
    let doc = parse_markdown("- item one\n- item two\n");
    let blocks = first_section_blocks(&doc);
    if let ir::Block::List { ordered, items, .. } = &blocks[0] {
        assert!(!ordered);
        assert_eq!(items.len(), 2);
    } else {
        panic!("expected List, got {:?}", blocks[0]);
    }
}

#[test]
fn parse_markdown_ordered_list() {
    let doc = parse_markdown("1. first\n2. second\n");
    let blocks = first_section_blocks(&doc);
    if let ir::Block::List {
        ordered,
        start,
        items,
        ..
    } = &blocks[0]
    {
        assert!(ordered);
        assert_eq!(*start, 1);
        assert_eq!(items.len(), 2);
    } else {
        panic!("expected ordered List, got {:?}", blocks[0]);
    }
}

// -----------------------------------------------------------------------
// parse_markdown — image
// -----------------------------------------------------------------------

#[test]
fn parse_markdown_image_with_alt_text() {
    let doc = parse_markdown("![a cat](cat.png)\n");
    let blocks = first_section_blocks(&doc);
    // comrak turns a standalone image in a paragraph into a Paragraph containing
    // an Image inline; the image block variant may also appear at top level.
    // We accept either representation.
    let found = blocks.iter().any(|b| match b {
        ir::Block::Image { src, alt } => src == "cat.png" && alt == "a cat",
        ir::Block::Paragraph { inlines } => inlines
            .iter()
            .any(|i| i.text.contains("cat.png") && i.text.contains("a cat")),
        _ => false,
    });
    assert!(found, "image not found in blocks: {blocks:?}");
}

// -----------------------------------------------------------------------
// parse_markdown — footnote
// -----------------------------------------------------------------------

#[test]
fn parse_markdown_footnote() {
    let md = "Text[^note].\n\n[^note]: The footnote.\n";
    let doc = parse_markdown(md);
    let blocks = first_section_blocks(&doc);
    let has_fn = blocks
        .iter()
        .any(|b| matches!(b, ir::Block::Footnote { id, .. } if id == "note"));
    assert!(has_fn, "footnote not found; blocks: {blocks:?}");
}

// -----------------------------------------------------------------------
// parse_markdown — math
// -----------------------------------------------------------------------

#[test]
fn parse_markdown_display_math() {
    // comrak math_dollars extension: $$formula$$ on a single line = display math.
    let md = "$$E=mc^2$$\n";
    let doc = parse_markdown(md);
    let blocks = first_section_blocks(&doc);
    // Display math may appear either as ir::Block::Math{display:true} or,
    // when comrak wraps it in a paragraph, as an inline whose text contains "$$".
    let has_display_math = blocks.iter().any(|b| match b {
        ir::Block::Math { display: true, .. } => true,
        ir::Block::Paragraph { inlines } => inlines.iter().any(|i| i.text.contains("$$")),
        _ => false,
    });
    assert!(
        has_display_math,
        "display math not found; blocks: {blocks:?}"
    );
}

// -----------------------------------------------------------------------
// parse_markdown — blockquote
// -----------------------------------------------------------------------

#[test]
fn parse_markdown_blockquote() {
    let doc = parse_markdown("> quoted text\n");
    let blocks = first_section_blocks(&doc);
    let has_bq = blocks
        .iter()
        .any(|b| matches!(b, ir::Block::BlockQuote { .. }));
    assert!(has_bq, "blockquote not found; blocks: {blocks:?}");
}

// -----------------------------------------------------------------------
// parse_markdown — frontmatter extraction via parse_markdown
// -----------------------------------------------------------------------

#[test]
fn parse_markdown_with_frontmatter_metadata() {
    let md = "---\ntitle: \"Parsed Title\"\nauthor: \"Bob\"\n---\n\n# Heading\n";
    let doc = parse_markdown(md);
    assert_eq!(doc.metadata.title.as_deref(), Some("Parsed Title"));
    assert_eq!(doc.metadata.author.as_deref(), Some("Bob"));
}

// -----------------------------------------------------------------------
// extract_frontmatter — additional cases
// -----------------------------------------------------------------------

#[test]
fn extract_frontmatter_keywords_array_format() {
    let md = "---\nkeywords: [rust, wasm, cli]\n---\n";
    let meta = extract_frontmatter(md);
    assert_eq!(meta.keywords, vec!["rust", "wasm", "cli"]);
}

#[test]
fn extract_frontmatter_keywords_comma_format() {
    let md = "---\nkeywords: alpha, beta, gamma\n---\n";
    let meta = extract_frontmatter(md);
    assert_eq!(meta.keywords, vec!["alpha", "beta", "gamma"]);
}

#[test]
fn extract_frontmatter_empty_input() {
    let meta = extract_frontmatter("");
    assert!(meta.title.is_none());
    assert!(meta.keywords.is_empty());
}

#[test]
fn extract_frontmatter_subject_and_description() {
    let md = "---\nsubject: My Subject\ndescription: A longer desc\n---\n";
    let meta = extract_frontmatter(md);
    assert_eq!(meta.subject.as_deref(), Some("My Subject"));
    assert_eq!(meta.description.as_deref(), Some("A longer desc"));
}

// -----------------------------------------------------------------------
// node_to_block — additional cases
// -----------------------------------------------------------------------

#[test]
fn node_to_block_footnote_definition() {
    let md = "[^fn]: Footnote body.\n\nText[^fn].\n";
    let doc = parse_markdown(md);
    let blocks = first_section_blocks(&doc);
    let found = blocks
        .iter()
        .any(|b| matches!(b, ir::Block::Footnote { id, .. } if id == "fn"));
    assert!(
        found,
        "FootnoteDefinition block not found; blocks: {blocks:?}"
    );
}

#[test]
fn node_to_block_display_math() {
    let md = "$$x^2 + y^2 = z^2$$\n";
    let doc = parse_markdown(md);
    let blocks = first_section_blocks(&doc);
    let found = blocks.iter().any(|b| match b {
        ir::Block::Math { display: true, tex } => tex.contains("x^2"),
        ir::Block::Paragraph { inlines } => inlines.iter().any(|i| i.text.contains("$$")),
        _ => false,
    });
    assert!(found, "display Math block not found; blocks: {blocks:?}");
}

#[test]
fn node_to_block_inline_math_in_paragraph() {
    let md = "Area is $\\pi r^2$ units.\n";
    let doc = parse_markdown(md);
    let blocks = first_section_blocks(&doc);
    if let ir::Block::Paragraph { inlines } = &blocks[0] {
        let has_math = inlines.iter().any(|i| i.text.contains('$'));
        assert!(has_math, "inline math not found in inlines: {inlines:?}");
    } else {
        panic!("expected Paragraph, got {:?}", blocks[0]);
    }
}

#[test]
fn node_to_block_image_alt_preserved() {
    let md = "![description here](img/photo.jpg)\n";
    let doc = parse_markdown(md);
    let blocks = first_section_blocks(&doc);
    let found = blocks.iter().any(|b| match b {
        ir::Block::Image { alt, src } => alt == "description here" && src == "img/photo.jpg",
        ir::Block::Paragraph { inlines } => inlines
            .iter()
            .any(|i| i.text.contains("description here") && i.text.contains("img/photo.jpg")),
        _ => false,
    });
    assert!(found, "image with alt text not found; blocks: {blocks:?}");
}

// -----------------------------------------------------------------------
// collect_inlines — additional cases
// -----------------------------------------------------------------------

#[test]
fn collect_inlines_bold_and_italic_combined() {
    let doc = parse_markdown("***both***\n");
    let blocks = first_section_blocks(&doc);
    if let ir::Block::Paragraph { inlines } = &blocks[0] {
        let found = inlines.iter().any(|i| i.bold && i.italic);
        assert!(found, "no bold+italic inline; inlines: {inlines:?}");
    } else {
        panic!("expected Paragraph");
    }
}

#[test]
fn collect_inlines_link_with_bold_text() {
    let doc = parse_markdown("[**click**](https://example.com)\n");
    let blocks = first_section_blocks(&doc);
    if let ir::Block::Paragraph { inlines } = &blocks[0] {
        let found = inlines
            .iter()
            .any(|i| i.bold && i.link.as_deref() == Some("https://example.com"));
        assert!(found, "bold+link inline not found; inlines: {inlines:?}");
    } else {
        panic!("expected Paragraph");
    }
}

#[test]
fn collect_inlines_footnote_reference() {
    let md = "Text[^ref].\n\n[^ref]: Note.\n";
    let doc = parse_markdown(md);
    let blocks = first_section_blocks(&doc);
    if let ir::Block::Paragraph { inlines } = &blocks[0] {
        let has_ref = inlines
            .iter()
            .any(|i| i.footnote_ref.as_deref() == Some("ref"));
        assert!(
            has_ref,
            "footnote reference not found; inlines: {inlines:?}"
        );
    } else {
        panic!("expected Paragraph, got {:?}", blocks[0]);
    }
}

#[test]
fn collect_inlines_inline_image() {
    let doc = parse_markdown("See ![icon](icons/star.svg) here.\n");
    let blocks = first_section_blocks(&doc);
    if let ir::Block::Paragraph { inlines } = &blocks[0] {
        let has_img = inlines.iter().any(|i| i.text.contains("icons/star.svg"));
        assert!(has_img, "inline image not found; inlines: {inlines:?}");
    } else {
        panic!("expected Paragraph");
    }
}

#[test]
fn collect_inlines_superscript() {
    // comrak superscript extension: ^text^
    let doc = parse_markdown("E=mc^2^\n");
    let blocks = first_section_blocks(&doc);
    if let ir::Block::Paragraph { inlines } = &blocks[0] {
        let has_sup = inlines.iter().any(|i| i.superscript);
        assert!(
            has_sup,
            "superscript inline not found; inlines: {inlines:?}"
        );
    } else {
        panic!("expected Paragraph");
    }
}

// -----------------------------------------------------------------------
// parse_markdown — complex document
// -----------------------------------------------------------------------

#[test]
fn parse_markdown_complex_document() {
    let md = r#"---
title: "Complex Doc"
author: "Tester"
keywords: [a, b, c]
---

# Heading One

A paragraph with **bold**, *italic*, and `code`.

> A blockquote with text.

| Col1 | Col2 |
| ---- | ---- |
| A    | B    |

1. First
2. Second

$$math$$

[^note]: Footnote text.

See[^note].
"#;
    let doc = parse_markdown(md);
    assert_eq!(doc.metadata.title.as_deref(), Some("Complex Doc"));
    assert_eq!(doc.metadata.author.as_deref(), Some("Tester"));
    assert_eq!(doc.metadata.keywords, vec!["a", "b", "c"]);

    let blocks = first_section_blocks(&doc);
    assert!(
        blocks
            .iter()
            .any(|b| matches!(b, ir::Block::Heading { level: 1, .. })),
        "h1 missing"
    );
    assert!(
        blocks
            .iter()
            .any(|b| matches!(b, ir::Block::Paragraph { .. })),
        "paragraph missing"
    );
    assert!(
        blocks
            .iter()
            .any(|b| matches!(b, ir::Block::BlockQuote { .. })),
        "blockquote missing"
    );
    assert!(
        blocks.iter().any(|b| matches!(b, ir::Block::Table { .. })),
        "table missing"
    );
    assert!(
        blocks
            .iter()
            .any(|b| matches!(b, ir::Block::List { ordered: true, .. })),
        "ordered list missing"
    );
    assert!(
        blocks.iter().any(|b| match b {
            ir::Block::Math { .. } => true,
            ir::Block::Paragraph { inlines } => inlines.iter().any(|i| i.text.contains("$$")),
            _ => false,
        }),
        "math missing"
    );
    assert!(
        blocks
            .iter()
            .any(|b| matches!(b, ir::Block::Footnote { id, .. } if id == "note")),
        "footnote definition missing"
    );
}
