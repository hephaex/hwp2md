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
    strikethrough: bool,
    code: bool,
    superscript: bool,
    link: Option<String>,
}

fn collect_inlines_recursive<'a>(
    node: &'a AstNode<'a>,
    inlines: &mut Vec<ir::Inline>,
    style: InlineStyle,
) {
    for child in node.children() {
        let data = child.data.borrow();
        match &data.value {
            NodeValue::Text(text) => {
                inlines.push(ir::Inline {
                    text: text.clone(),
                    bold: style.bold,
                    italic: style.italic,
                    strikethrough: style.strikethrough,
                    code: style.code,
                    superscript: style.superscript,
                    link: style.link.clone(),
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
            NodeValue::Strong => {
                let mut s = style.clone();
                s.bold = true;
                collect_inlines_recursive(child, inlines, s);
            }
            NodeValue::Emph => {
                let mut s = style.clone();
                s.italic = true;
                collect_inlines_recursive(child, inlines, s);
            }
            NodeValue::Strikethrough => {
                let mut s = style.clone();
                s.strikethrough = true;
                collect_inlines_recursive(child, inlines, s);
            }
            NodeValue::Superscript => {
                let mut s = style.clone();
                s.superscript = true;
                collect_inlines_recursive(child, inlines, s);
            }
            NodeValue::Link(link) => {
                let mut s = style.clone();
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
                collect_inlines_recursive(child, inlines, style.clone());
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
}
