use crate::ir;
use comrak::nodes::{AstNode, ListType, NodeValue};
use comrak::{parse_document, Arena, Options};

pub fn parse_markdown(input: &str) -> ir::Document {
    let arena = Arena::new();
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.footnotes = true;
    options.extension.math_dollars = true;
    options.extension.superscript = true;

    let root = parse_document(&arena, input, &options);

    let mut doc = ir::Document::new();
    doc.metadata = extract_frontmatter(input);

    let mut section = ir::Section { blocks: Vec::new() };

    for child in root.children() {
        if let Some(block) = node_to_block(child) {
            section.blocks.push(block);
        }
    }

    doc.sections.push(section);
    doc
}

fn extract_frontmatter(input: &str) -> ir::Metadata {
    let mut meta = ir::Metadata::default();

    let trimmed = input.trim_start();
    if !trimmed.starts_with("---") {
        return meta;
    }

    let rest = &trimmed[3..];
    if let Some(end) = rest.find("\n---") {
        let yaml_block = &rest[..end];
        for line in yaml_block.lines() {
            let line = line.trim();
            if let Some((key, val)) = line.split_once(':') {
                let key = key.trim();
                let val = val.trim().trim_matches('"').trim_matches('\'');
                match key {
                    "title" => meta.title = Some(val.to_string()),
                    "author" => meta.author = Some(val.to_string()),
                    "date" => meta.created = Some(val.to_string()),
                    "subject" => meta.subject = Some(val.to_string()),
                    "description" => meta.description = Some(val.to_string()),
                    "keywords" => {
                        // Accept both inline-array form `[a, b, c]` and
                        // plain comma-separated string `a, b, c`.
                        let raw = if val.starts_with('[') && val.ends_with(']') {
                            &val[1..val.len() - 1]
                        } else {
                            val
                        };
                        meta.keywords = raw
                            .split(',')
                            .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    _ => {}
                }
            }
        }
    }

    meta
}

fn node_to_block<'a>(node: &'a AstNode<'a>) -> Option<ir::Block> {
    let data = node.data.borrow();
    match &data.value {
        NodeValue::Heading(heading) => {
            let inlines = collect_inlines(node);
            Some(ir::Block::Heading {
                level: heading.level,
                inlines,
            })
        }
        NodeValue::Paragraph => {
            let inlines = collect_inlines(node);
            if inlines.is_empty() {
                return None;
            }
            Some(ir::Block::Paragraph { inlines })
        }
        NodeValue::CodeBlock(cb) => Some(ir::Block::CodeBlock {
            language: if cb.info.is_empty() {
                None
            } else {
                Some(cb.info.clone())
            },
            code: cb.literal.clone(),
        }),
        NodeValue::BlockQuote => {
            let blocks: Vec<_> = node.children().filter_map(|c| node_to_block(c)).collect();
            Some(ir::Block::BlockQuote { blocks })
        }
        NodeValue::List(list) => {
            let ordered = list.list_type == ListType::Ordered;
            let start = list.start as u32;
            let items: Vec<_> = node
                .children()
                .filter_map(|item| {
                    let item_data = item.data.borrow();
                    if matches!(item_data.value, NodeValue::Item(_)) {
                        let blocks: Vec<_> =
                            item.children().filter_map(|c| node_to_block(c)).collect();
                        Some(ir::ListItem {
                            blocks,
                            children: Vec::new(),
                        })
                    } else {
                        None
                    }
                })
                .collect();
            Some(ir::Block::List {
                ordered,
                start,
                items,
            })
        }
        NodeValue::Table(_) => {
            let mut rows = Vec::new();
            let mut col_count = 0;

            for (ri, row_node) in node.children().enumerate() {
                let row_data = row_node.data.borrow();
                if !matches!(row_data.value, NodeValue::TableRow(_)) {
                    continue;
                }

                let mut cells = Vec::new();
                for cell_node in row_node.children() {
                    let cell_data = cell_node.data.borrow();
                    if !matches!(cell_data.value, NodeValue::TableCell) {
                        continue;
                    }
                    let inlines = collect_inlines(cell_node);
                    cells.push(ir::TableCell {
                        blocks: vec![ir::Block::Paragraph { inlines }],
                        colspan: 1,
                        rowspan: 1,
                    });
                }
                col_count = col_count.max(cells.len());
                rows.push(ir::TableRow {
                    cells,
                    is_header: ri == 0,
                });
            }

            Some(ir::Block::Table { rows, col_count })
        }
        NodeValue::ThematicBreak => Some(ir::Block::HorizontalRule),
        NodeValue::Image(link) => {
            let alt = collect_alt_text(node);
            Some(ir::Block::Image {
                src: link.url.clone(),
                alt,
            })
        }
        NodeValue::FootnoteDefinition(def) => {
            let blocks: Vec<_> = node.children().filter_map(|c| node_to_block(c)).collect();
            Some(ir::Block::Footnote {
                id: def.name.clone(),
                content: blocks,
            })
        }
        NodeValue::Math(math) => Some(ir::Block::Math {
            display: math.display_math,
            tex: math.literal.clone(),
        }),
        _ => None,
    }
}

fn collect_alt_text<'a>(node: &'a AstNode<'a>) -> String {
    let mut text = String::new();
    for child in node.children() {
        let data = child.data.borrow();
        if let NodeValue::Text(t) = &data.value {
            text.push_str(t);
        }
    }
    text
}

fn collect_inlines<'a>(node: &'a AstNode<'a>) -> Vec<ir::Inline> {
    let mut inlines = Vec::new();
    collect_inlines_recursive(node, &mut inlines, InlineStyle::default());
    inlines
}

#[derive(Default, Clone)]
struct InlineStyle {
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
    code: bool,
    superscript: bool,
    subscript: bool,
    link: Option<String>,
}

fn collect_inlines_recursive<'a>(
    node: &'a AstNode<'a>,
    inlines: &mut Vec<ir::Inline>,
    style: InlineStyle,
) {
    let mut current_style = style.clone();
    for child in node.children() {
        let data = child.data.borrow();
        match &data.value {
            NodeValue::Text(text) => {
                inlines.push(ir::Inline {
                    text: text.clone(),
                    bold: current_style.bold,
                    italic: current_style.italic,
                    underline: current_style.underline,
                    strikethrough: current_style.strikethrough,
                    code: current_style.code,
                    superscript: current_style.superscript,
                    subscript: current_style.subscript,
                    link: current_style.link.clone(),
                    ..ir::Inline::default()
                });
            }
            NodeValue::SoftBreak | NodeValue::LineBreak => {
                inlines.push(ir::Inline::plain("\n"));
            }
            NodeValue::Code(code) => {
                inlines.push(ir::Inline {
                    text: code.literal.clone(),
                    code: true,
                    ..ir::Inline::default()
                });
            }
            NodeValue::HtmlInline(html) => {
                let tag = html.trim();
                if tag.eq_ignore_ascii_case("<u>") {
                    current_style.underline = true;
                    continue;
                } else if tag.eq_ignore_ascii_case("</u>") {
                    current_style.underline = style.underline;
                    continue;
                } else if tag.eq_ignore_ascii_case("<sub>") {
                    current_style.subscript = true;
                    continue;
                } else if tag.eq_ignore_ascii_case("</sub>") {
                    current_style.subscript = style.subscript;
                    continue;
                } else {
                    inlines.push(ir::Inline::plain(html.clone()));
                }
            }
            NodeValue::Strong => {
                let mut s = current_style.clone();
                s.bold = true;
                collect_inlines_recursive(child, inlines, s);
            }
            NodeValue::Emph => {
                let mut s = current_style.clone();
                s.italic = true;
                collect_inlines_recursive(child, inlines, s);
            }
            NodeValue::Strikethrough => {
                let mut s = current_style.clone();
                s.strikethrough = true;
                collect_inlines_recursive(child, inlines, s);
            }
            NodeValue::Superscript => {
                let mut s = current_style.clone();
                s.superscript = true;
                collect_inlines_recursive(child, inlines, s);
            }
            NodeValue::Link(link) => {
                let mut s = current_style.clone();
                s.link = Some(link.url.clone());
                collect_inlines_recursive(child, inlines, s);
            }
            NodeValue::Image(link) => {
                let alt = collect_alt_text(child);
                inlines.push(ir::Inline {
                    text: format!("![{}]({})", alt, link.url),
                    ..ir::Inline::default()
                });
            }
            NodeValue::FootnoteReference(fref) => {
                inlines.push(ir::Inline {
                    text: String::new(),
                    footnote_ref: Some(fref.name.clone()),
                    ..ir::Inline::default()
                });
            }
            NodeValue::Math(math) => {
                let delim = if math.display_math { "$$" } else { "$" };
                inlines.push(ir::Inline::plain(format!("{delim}{}{delim}", math.literal)));
            }
            _ => {
                collect_inlines_recursive(child, inlines, current_style.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
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
}
