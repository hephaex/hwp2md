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

    let mut section = ir::Section {
        blocks: Vec::new(),
        page_layout: None,
    };

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
                inlines.push(
                    ir::Inline::with_formatting(
                        text.clone(),
                        current_style.bold,
                        current_style.italic,
                        current_style.underline,
                        current_style.strikethrough,
                        current_style.superscript,
                        current_style.subscript,
                        None,
                    )
                    .with_link(current_style.link.clone()),
                );
            }
            NodeValue::SoftBreak | NodeValue::LineBreak => {
                inlines.push(ir::Inline::plain("\n"));
            }
            NodeValue::Code(code) => {
                inlines.push(ir::Inline {
                    text: code.literal.clone(),
                    bold: false,
                    italic: false,
                    underline: false,
                    strikethrough: false,
                    code: true,
                    superscript: false,
                    subscript: false,
                    link: None,
                    footnote_ref: None,
                    color: None,
                    font_name: None,
                    ruby: None,
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
                inlines.push(ir::Inline::plain(format!("![{}]({})", alt, link.url)));
            }
            NodeValue::FootnoteReference(fref) => {
                inlines.push(ir::Inline::footnote_ref(fref.name.clone()));
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
#[path = "parser_tests.rs"]
mod tests;
