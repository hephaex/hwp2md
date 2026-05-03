use super::*;
use crate::ir;

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

fn first_section_blocks(doc: &ir::Document) -> &[ir::Block] {
    doc.sections
        .first()
        .map(|s| s.blocks.as_slice())
        .unwrap_or(&[])
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
    if let ir::Block::Table { rows, col_count } = &blocks[0] {
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

#[test]
fn collect_inlines_underline_via_html_u_tag() {
    // HTML inline tags <u>…</u> are passed through as raw HTML by comrak when
    // unsafe HTML is NOT enabled (the default). In that case comrak emits the
    // raw tag string as HtmlInline nodes which our handler intercepts.
    let mut options = comrak::Options::default();
    options.render.unsafe_ = true; // allow raw HTML so comrak parses <u>
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, "Hello <u>world</u>!\n", &options);

    let para = root
        .children()
        .find(|c| matches!(c.data.borrow().value, NodeValue::Paragraph));
    let para = para.expect("paragraph node not found");

    let inlines = collect_inlines(para);
    let has_underline = inlines.iter().any(|i| i.underline && i.text == "world");
    assert!(
        has_underline,
        "underline inline not found; inlines: {inlines:?}"
    );
}

#[test]
fn collect_inlines_subscript_via_html_sub_tag() {
    let mut options = comrak::Options::default();
    options.render.unsafe_ = true;
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, "H<sub>2</sub>O\n", &options);

    let para = root
        .children()
        .find(|c| matches!(c.data.borrow().value, NodeValue::Paragraph));
    let para = para.expect("paragraph node not found");

    let inlines = collect_inlines(para);
    let has_subscript = inlines.iter().any(|i| i.subscript && i.text == "2");
    assert!(
        has_subscript,
        "subscript inline not found; inlines: {inlines:?}"
    );
}

// -----------------------------------------------------------------------
// collect_inlines — nested <u><sub> combinations
// -----------------------------------------------------------------------

fn parse_with_unsafe_html(input: &str) -> Vec<ir::Inline> {
    let mut options = comrak::Options::default();
    options.render.unsafe_ = true;
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, input, &options);
    let para = root
        .children()
        .find(|c| matches!(c.data.borrow().value, NodeValue::Paragraph))
        .expect("paragraph not found");
    collect_inlines(para)
}

#[test]
fn collect_inlines_u_wrapping_sub() {
    let inlines = parse_with_unsafe_html("<u><sub>text</sub></u>\n");
    let found = inlines
        .iter()
        .any(|i| i.underline && i.subscript && i.text == "text");
    assert!(
        found,
        "<u><sub>text</sub></u>: expected underline+subscript; got {inlines:?}"
    );
}

#[test]
fn collect_inlines_sub_wrapping_u() {
    let inlines = parse_with_unsafe_html("<sub><u>text</u></sub>\n");
    let found = inlines
        .iter()
        .any(|i| i.underline && i.subscript && i.text == "text");
    assert!(
        found,
        "<sub><u>text</u></sub>: expected underline+subscript; got {inlines:?}"
    );
}

#[test]
fn collect_inlines_unclosed_u_applies_underline_to_remaining() {
    let inlines = parse_with_unsafe_html("<u>text\n");
    let has_underline = inlines
        .iter()
        .any(|i| i.underline && i.text.contains("text"));
    assert!(
        has_underline,
        "unclosed <u>: underline should apply to remaining text; got {inlines:?}"
    );
}

#[test]
fn collect_inlines_u_then_sub_are_separate() {
    let inlines = parse_with_unsafe_html("<u>a</u><sub>b</sub>\n");
    let has_underline_a = inlines
        .iter()
        .any(|i| i.underline && !i.subscript && i.text == "a");
    let has_subscript_b = inlines
        .iter()
        .any(|i| i.subscript && !i.underline && i.text == "b");
    assert!(
        has_underline_a,
        "<u>a</u>: expected underline-only for 'a'; got {inlines:?}"
    );
    assert!(
        has_subscript_b,
        "<sub>b</sub>: expected subscript-only for 'b'; got {inlines:?}"
    );
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

// -----------------------------------------------------------------------
// SoftBreak / LineBreak → "\n" inline
// -----------------------------------------------------------------------

#[test]
fn collect_inlines_soft_break_emits_newline() {
    let doc = parse_markdown("first\nsecond\n");
    let blocks = first_section_blocks(&doc);
    assert_eq!(blocks.len(), 1, "expected exactly one paragraph block");
    match &blocks[0] {
        ir::Block::Paragraph { inlines } => {
            let combined: String = inlines.iter().map(|i| i.text.as_str()).collect();
            assert!(
                combined.contains("first"),
                "text 'first' missing; combined: {combined:?}"
            );
            assert!(
                combined.contains("second"),
                "text 'second' missing; combined: {combined:?}"
            );
            assert!(
                inlines.iter().any(|i| i.text == "\n"),
                "SoftBreak must produce a newline inline; inlines: {inlines:?}"
            );
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

#[test]
fn collect_inlines_hard_line_break_emits_newline() {
    // Two trailing spaces force a hard line break (LineBreak node in comrak).
    let doc = parse_markdown("line one  \nline two\n");
    let blocks = first_section_blocks(&doc);
    assert!(
        !blocks.is_empty(),
        "expected at least one block; got nothing"
    );
    if let ir::Block::Paragraph { inlines } = &blocks[0] {
        // Should contain text from both lines and possibly a newline inline.
        let combined: String = inlines.iter().map(|i| i.text.as_str()).collect();
        assert!(
            combined.contains("line one") || combined.contains("line two"),
            "line content missing; got: {combined:?}"
        );
    }
}

// -----------------------------------------------------------------------
// HtmlInline — unknown tags emitted as plain inline text
// -----------------------------------------------------------------------

#[test]
fn collect_inlines_unknown_html_inline_becomes_plain_text() {
    let mut options = comrak::Options::default();
    options.render.unsafe_ = true;
    let arena = comrak::Arena::new();
    // <span> is not handled — must be emitted verbatim as a plain inline.
    let root = comrak::parse_document(&arena, "Hello <span>world</span>!\n", &options);

    let para = root
        .children()
        .find(|c| matches!(c.data.borrow().value, NodeValue::Paragraph))
        .expect("paragraph not found");

    let inlines = collect_inlines(para);
    // The text "world" should appear either as a standalone plain inline or
    // inside the span tags. We just require no panic and some content.
    let combined: String = inlines.iter().map(|i| i.text.as_str()).collect();
    assert!(
        combined.contains("Hello"),
        "plain text lost; combined: {combined:?}"
    );
    // <span> and </span> should appear as plain text since they are unknown.
    assert!(
        combined.contains("span") || combined.contains("world"),
        "unknown html inline not rendered; combined: {combined:?}"
    );
}

// -----------------------------------------------------------------------
// collect_alt_text — text inside an image node
// -----------------------------------------------------------------------

#[test]
fn node_to_block_image_no_alt_returns_empty_alt() {
    // An image with no alt text should have an empty alt string.
    let doc = parse_markdown("![](photo.jpg)\n");
    let blocks = first_section_blocks(&doc);
    let found = blocks.iter().any(|b| match b {
        ir::Block::Image { alt, .. } => alt.is_empty(),
        ir::Block::Paragraph { inlines } => inlines.iter().any(|i| i.text.contains("photo.jpg")),
        _ => false,
    });
    assert!(found, "image not found; blocks: {blocks:?}");
}

// -----------------------------------------------------------------------
// </sub> close tag restores outer subscript state
// -----------------------------------------------------------------------

#[test]
fn collect_inlines_sub_close_tag_restores_state() {
    // After </sub>, subsequent text should NOT be subscript.
    let inlines = parse_with_unsafe_html("<sub>x</sub>y\n");
    let x_is_subscript = inlines.iter().any(|i| i.subscript && i.text == "x");
    let y_not_subscript = inlines.iter().any(|i| !i.subscript && i.text == "y");
    assert!(
        x_is_subscript,
        "x inside <sub> must be subscript; got {inlines:?}"
    );
    assert!(
        y_not_subscript,
        "y after </sub> must NOT be subscript; got {inlines:?}"
    );
}

// -----------------------------------------------------------------------
// Horizontal rule parsed correctly
// -----------------------------------------------------------------------

#[test]
fn parse_markdown_horizontal_rule() {
    let doc = parse_markdown("---\n\ntext\n");
    let blocks = first_section_blocks(&doc);
    assert!(
        blocks
            .iter()
            .any(|b| matches!(b, ir::Block::HorizontalRule)),
        "expected HorizontalRule block from '---'; blocks: {blocks:?}"
    );
    assert!(
        blocks
            .iter()
            .any(|b| matches!(b, ir::Block::Paragraph { .. })),
        "expected Paragraph block with 'text'; blocks: {blocks:?}"
    );
}

#[test]
fn parse_markdown_pagebreak_html_comment_yields_page_break_block() {
    let doc = parse_markdown("before\n\n<!-- pagebreak -->\n\nafter\n");
    let blocks = first_section_blocks(&doc);
    // Tighten: assert the exact block sequence, not just position bounds, so
    // that any silent re-ordering or extra blocks fail the test.
    let kinds: Vec<&'static str> = blocks
        .iter()
        .map(|b| match b {
            ir::Block::Paragraph { .. } => "para",
            ir::Block::PageBreak => "pb",
            _ => "other",
        })
        .collect();
    assert_eq!(
        kinds,
        vec!["para", "pb", "para"],
        "expected exact [para, pb, para] sequence: {blocks:?}"
    );
}

#[test]
fn parse_markdown_pagebreak_marker_is_case_insensitive() {
    let doc = parse_markdown("text\n\n<!-- PageBreak -->\n\nmore\n");
    let blocks = first_section_blocks(&doc);
    assert!(
        blocks.iter().any(|b| matches!(b, ir::Block::PageBreak)),
        "case-insensitive marker should yield PageBreak: {blocks:?}"
    );
}

#[test]
fn parse_markdown_unrelated_html_comment_is_not_pagebreak() {
    let doc = parse_markdown("text\n\n<!-- not a page break -->\n\nmore\n");
    let blocks = first_section_blocks(&doc);
    assert!(
        !blocks.iter().any(|b| matches!(b, ir::Block::PageBreak)),
        "non-pagebreak HTML comment must not yield PageBreak: {blocks:?}"
    );
}

#[test]
fn parse_markdown_pagebreak_lookalike_substring_is_rejected() {
    // The `pagebreakish` keyword contains `pagebreak` as a substring.  The
    // marker detector must require an EXACT match (after trimming), not a
    // substring search.
    let doc = parse_markdown("a\n\n<!-- pagebreakish -->\n\nb\n");
    let blocks = first_section_blocks(&doc);
    assert!(
        !blocks.iter().any(|b| matches!(b, ir::Block::PageBreak)),
        "lookalike `pagebreakish` must not match: {blocks:?}"
    );
}

#[test]
fn parse_markdown_pagebreak_marker_with_trailing_text_is_rejected() {
    // Comrak feeds the entire HTML block including any text after the
    // closing `-->`.  The detector must refuse to match in that case so
    // that the trailing text is preserved rather than swallowed.
    let doc = parse_markdown("a\n\n<!-- pagebreak --> trailing\n\nb\n");
    let blocks = first_section_blocks(&doc);
    assert!(
        !blocks.iter().any(|b| matches!(b, ir::Block::PageBreak)),
        "marker with trailing text must not match: {blocks:?}"
    );
}

// -----------------------------------------------------------------------
// parse_markdown — strikethrough via ~~…~~
// -----------------------------------------------------------------------

#[test]
fn parse_markdown_strikethrough_inline() {
    let doc = parse_markdown("~~struck~~\n");
    let blocks = first_section_blocks(&doc);
    if let ir::Block::Paragraph { inlines } = &blocks[0] {
        let has_strike = inlines.iter().any(|i| i.strikethrough);
        assert!(has_strike, "no strikethrough inline; inlines: {inlines:?}");
    } else {
        panic!("expected Paragraph, got {:?}", blocks[0]);
    }
}

// -----------------------------------------------------------------------
// parse_markdown — task list (GitHub-style checkboxes)
// -----------------------------------------------------------------------

#[test]
fn parse_markdown_task_list_unchecked_item() {
    let doc = parse_markdown("- [ ] unchecked\n");
    let blocks = first_section_blocks(&doc);
    if let ir::Block::List { items, ordered, .. } = &blocks[0] {
        assert!(!ordered, "task list must be unordered");
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].checked,
            Some(false),
            "unchecked item must have checked=Some(false); item: {:?}",
            items[0]
        );
    } else {
        panic!("expected List, got {:?}", blocks[0]);
    }
}

#[test]
fn parse_markdown_task_list_checked_item() {
    let doc = parse_markdown("- [x] checked\n");
    let blocks = first_section_blocks(&doc);
    if let ir::Block::List { items, .. } = &blocks[0] {
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].checked,
            Some(true),
            "checked item must have checked=Some(true); item: {:?}",
            items[0]
        );
    } else {
        panic!("expected List, got {:?}", blocks[0]);
    }
}

#[test]
fn parse_markdown_task_list_checked_capital_x() {
    let doc = parse_markdown("- [X] also checked\n");
    let blocks = first_section_blocks(&doc);
    if let ir::Block::List { items, .. } = &blocks[0] {
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].checked,
            Some(true),
            "capital-X checked item must have checked=Some(true); item: {:?}",
            items[0]
        );
    } else {
        panic!("expected List, got {:?}", blocks[0]);
    }
}

#[test]
fn parse_markdown_task_list_mixed_items() {
    let md = "- [x] done\n- [ ] todo\n- [ ] also todo\n";
    let doc = parse_markdown(md);
    let blocks = first_section_blocks(&doc);
    if let ir::Block::List { items, .. } = &blocks[0] {
        assert_eq!(items.len(), 3, "expected 3 items; got {}", items.len());
        assert_eq!(items[0].checked, Some(true), "item[0] must be checked");
        assert_eq!(items[1].checked, Some(false), "item[1] must be unchecked");
        assert_eq!(items[2].checked, Some(false), "item[2] must be unchecked");
    } else {
        panic!("expected List, got {:?}", blocks[0]);
    }
}

#[test]
fn parse_markdown_normal_list_item_has_checked_none() {
    let doc = parse_markdown("- plain item\n");
    let blocks = first_section_blocks(&doc);
    if let ir::Block::List { items, .. } = &blocks[0] {
        assert_eq!(
            items[0].checked, None,
            "normal list item must have checked=None; item: {:?}",
            items[0]
        );
    } else {
        panic!("expected List, got {:?}", blocks[0]);
    }
}

#[test]
fn parse_markdown_task_list_text_is_preserved() {
    let doc = parse_markdown("- [x] buy milk\n");
    let blocks = first_section_blocks(&doc);
    if let ir::Block::List { items, .. } = &blocks[0] {
        let text: String = items[0]
            .blocks
            .iter()
            .flat_map(|b| match b {
                ir::Block::Paragraph { inlines } => {
                    inlines.iter().map(|i| i.text.as_str()).collect::<Vec<_>>()
                }
                _ => vec![],
            })
            .collect();
        assert!(
            text.contains("buy milk"),
            "item text must be preserved; got: {text:?}"
        );
    } else {
        panic!("expected List, got {:?}", blocks[0]);
    }
}

// -----------------------------------------------------------------------
// parse_markdown — header/footer HTML comment markers
// -----------------------------------------------------------------------

#[test]
fn header_footer_markers_parsed_to_section() {
    let md = "\
<!-- header -->
Header line
<!-- /header -->

<!-- footer -->
Footer line
<!-- /footer -->

Body paragraph
";
    let doc = parse_markdown(md);
    let section = doc.sections.first().expect("section must exist");

    // Body must contain the paragraph but NOT the header/footer text.
    let body_text: String = section
        .blocks
        .iter()
        .filter_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
            } else {
                None
            }
        })
        .collect();
    assert!(
        body_text.contains("Body paragraph"),
        "body paragraph missing; got: {body_text:?}"
    );
    assert!(
        !body_text.contains("Header line"),
        "header text must not appear in body; got: {body_text:?}"
    );
    assert!(
        !body_text.contains("Footer line"),
        "footer text must not appear in body; got: {body_text:?}"
    );

    // Header blocks.
    let header_blocks = section
        .header
        .as_ref()
        .expect("section.header must be Some");
    let header_text: String = header_blocks
        .iter()
        .filter_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
            } else {
                None
            }
        })
        .collect();
    assert!(
        header_text.contains("Header line"),
        "header text not found; got: {header_text:?}"
    );

    // Footer blocks.
    let footer_blocks = section
        .footer
        .as_ref()
        .expect("section.footer must be Some");
    let footer_text: String = footer_blocks
        .iter()
        .filter_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
            } else {
                None
            }
        })
        .collect();
    assert!(
        footer_text.contains("Footer line"),
        "footer text not found; got: {footer_text:?}"
    );
}

#[test]
fn header_only_marker() {
    let md = "\
<!-- header -->
Just a header
<!-- /header -->

Body text
";
    let doc = parse_markdown(md);
    let section = doc.sections.first().expect("section must exist");

    let header = section
        .header
        .as_ref()
        .expect("section.header must be Some");
    let htext: String = header
        .iter()
        .filter_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
            } else {
                None
            }
        })
        .collect();
    assert!(
        htext.contains("Just a header"),
        "header text not found; got: {htext:?}"
    );

    assert!(
        section.footer.is_none(),
        "footer must be None when no footer marker present"
    );
}

#[test]
fn footer_only_marker() {
    let md = "\
<!-- footer -->
Just a footer
<!-- /footer -->

Body text
";
    let doc = parse_markdown(md);
    let section = doc.sections.first().expect("section must exist");

    assert!(
        section.header.is_none(),
        "header must be None when no header marker present"
    );

    let footer = section
        .footer
        .as_ref()
        .expect("section.footer must be Some");
    let ftext: String = footer
        .iter()
        .filter_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
            } else {
                None
            }
        })
        .collect();
    assert!(
        ftext.contains("Just a footer"),
        "footer text not found; got: {ftext:?}"
    );
}

#[test]
fn no_markers_leaves_header_footer_none() {
    let doc = parse_markdown("# Heading\n\nParagraph.\n");
    let section = doc.sections.first().expect("section must exist");
    assert!(
        section.header.is_none(),
        "header must be None when no markers are present"
    );
    assert!(
        section.footer.is_none(),
        "footer must be None when no markers are present"
    );
}

#[test]
fn markers_case_insensitive() {
    let md = "\
<!-- HEADER -->
Upper case header
<!-- /HEADER -->

<!-- FOOTER -->
Upper case footer
<!-- /FOOTER -->

Body
";
    let doc = parse_markdown(md);
    let section = doc.sections.first().expect("section must exist");

    let header = section
        .header
        .as_ref()
        .expect("header must be Some for HEADER marker");
    let htext: String = header
        .iter()
        .filter_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
            } else {
                None
            }
        })
        .collect();
    assert!(
        htext.contains("Upper case header"),
        "case-insensitive HEADER not matched; got: {htext:?}"
    );

    let footer = section
        .footer
        .as_ref()
        .expect("footer must be Some for FOOTER marker");
    let ftext: String = footer
        .iter()
        .filter_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
            } else {
                None
            }
        })
        .collect();
    assert!(
        ftext.contains("Upper case footer"),
        "case-insensitive FOOTER not matched; got: {ftext:?}"
    );
}

#[test]
fn header_footer_markers_roundtrip_via_write_then_parse() {
    use crate::md::write_markdown;

    let header_text = "Running header text";
    let footer_text = "Running footer text";
    let body_text = "Main body content";

    let mut doc = ir::Document::new();
    doc.sections.push(ir::Section {
        blocks: vec![ir::Block::Paragraph {
            inlines: vec![ir::Inline::plain(body_text)],
        }],
        page_layout: None,
        header: Some(vec![ir::Block::Paragraph {
            inlines: vec![ir::Inline::plain(header_text)],
        }]),
        footer: Some(vec![ir::Block::Paragraph {
            inlines: vec![ir::Inline::plain(footer_text)],
        }]),
        header_footer_type: None,
    });

    let md = write_markdown(&doc, false);
    assert!(
        md.contains("<!-- header -->"),
        "header open marker missing in MD output; md: {md:?}"
    );
    assert!(
        md.contains("<!-- /header -->"),
        "header close marker missing in MD output; md: {md:?}"
    );
    assert!(
        md.contains("<!-- footer -->"),
        "footer open marker missing in MD output; md: {md:?}"
    );
    assert!(
        md.contains("<!-- /footer -->"),
        "footer close marker missing in MD output; md: {md:?}"
    );

    let parsed = parse_markdown(&md);
    let section = parsed.sections.first().expect("section must exist");

    // Body preserved, header/footer NOT leaked into body.
    let body: String = section
        .blocks
        .iter()
        .filter_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
            } else {
                None
            }
        })
        .collect();
    assert!(
        body.contains(body_text),
        "body text lost after roundtrip; body: {body:?}"
    );
    assert!(
        !body.contains(header_text),
        "header text leaked into body; body: {body:?}"
    );
    assert!(
        !body.contains(footer_text),
        "footer text leaked into body; body: {body:?}"
    );

    // Header preserved.
    let header_blocks = section
        .header
        .as_ref()
        .expect("section.header must be Some");
    let htext: String = header_blocks
        .iter()
        .filter_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
            } else {
                None
            }
        })
        .collect();
    assert!(
        htext.contains(header_text),
        "header text lost after roundtrip; got: {htext:?}"
    );

    // Footer preserved.
    let footer_blocks = section
        .footer
        .as_ref()
        .expect("section.footer must be Some");
    let ftext: String = footer_blocks
        .iter()
        .filter_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
            } else {
                None
            }
        })
        .collect();
    assert!(
        ftext.contains(footer_text),
        "footer text lost after roundtrip; got: {ftext:?}"
    );
}
