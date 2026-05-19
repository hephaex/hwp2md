use crate::ir::{self, InlineFormat};
use comrak::nodes::{AstNode, ListType, NodeValue};
use comrak::{parse_document, Arena, Options};

#[must_use]
// Top-level parser walks all CommonMark node kinds; splitting would lose structural coherence.
#[allow(clippy::too_many_lines)]
pub fn parse_markdown(input: &str) -> ir::Document {
    // Walk the AST nodes and route them into body, header, or footer
    // depending on the surrounding `<!-- header -->` / `<!-- /header -->` and
    // `<!-- footer -->` / `<!-- /footer -->` marker pairs.  The markers are
    // detected at the comrak AST level (before `node_to_block` is called) so
    // that they never appear as IR blocks themselves.
    #[derive(PartialEq, Debug)]
    enum Region {
        Body,
        Header,
        Footer,
    }

    let arena = Arena::new();
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.footnotes = true;
    options.extension.math_dollars = true;
    options.extension.superscript = true;
    options.extension.tasklist = true;

    let root = parse_document(&arena, input, &options);

    let mut doc = ir::Document::new();
    doc.metadata = extract_frontmatter(input);

    let mut region = Region::Body;
    let mut header_blocks: Vec<ir::Block> = Vec::new();
    let mut footer_blocks: Vec<ir::Block> = Vec::new();
    let mut body_blocks: Vec<ir::Block> = Vec::new();

    for child in root.children() {
        // Inspect the node value to detect marker comments.  The borrow of
        // `child.data` must be dropped before calling `node_to_block`, which
        // borrows the same RefCell internally.
        let is_marker = {
            let data = child.data.borrow();
            if let NodeValue::HtmlBlock(html) = &data.value {
                if is_header_start_marker(&html.literal) {
                    region = Region::Header;
                    true
                } else if is_header_end_marker(&html.literal) {
                    region = Region::Body;
                    true
                } else if is_footer_start_marker(&html.literal) {
                    region = Region::Footer;
                    true
                } else if is_footer_end_marker(&html.literal) {
                    region = Region::Body;
                    true
                } else {
                    false
                }
            } else {
                false
            }
        };

        if is_marker {
            continue;
        }

        if let Some(block) = node_to_block(child) {
            match region {
                Region::Body => body_blocks.push(block),
                Region::Header => header_blocks.push(block),
                Region::Footer => footer_blocks.push(block),
            }
        }
    }

    // Unclosed marker fallback: if we reach end-of-input still inside a
    // Header or Footer region, the marker was never closed.  Move the
    // misrouted blocks into body_blocks so content is not silently lost,
    // and warn the caller.
    if region != Region::Body {
        tracing::warn!("unclosed {region:?} marker in markdown input, falling back to body");
        body_blocks.append(&mut header_blocks);
        body_blocks.append(&mut footer_blocks);
    }

    let section = ir::Section {
        blocks: body_blocks,
        page_layout: None,
        header: if header_blocks.is_empty() {
            None
        } else {
            Some(header_blocks)
        },
        footer: if footer_blocks.is_empty() {
            None
        } else {
            Some(footer_blocks)
        },
        header_footer_type: None,
    };

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

// Dispatches over all CommonMark node types; heading level is 1-6, always fits in u8.
#[allow(clippy::too_many_lines)]
#[allow(clippy::cast_possible_truncation)]
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
                    match &item_data.value {
                        NodeValue::Item(_) => {
                            let blocks: Vec<_> =
                                item.children().filter_map(|c| node_to_block(c)).collect();
                            Some(ir::ListItem {
                                blocks,
                                children: Vec::new(),
                                checked: None,
                            })
                        }
                        NodeValue::TaskItem(symbol) => {
                            let checked = symbol.is_some();
                            let blocks: Vec<_> =
                                item.children().filter_map(|c| node_to_block(c)).collect();
                            Some(ir::ListItem {
                                blocks,
                                children: Vec::new(),
                                checked: Some(checked),
                            })
                        }
                        _ => None,
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
        NodeValue::HtmlBlock(html) => {
            if is_pagebreak_marker(&html.literal) {
                Some(ir::Block::PageBreak)
            } else {
                None
            }
        }
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

/// Return `true` when an HTML block contains exactly a `<!-- pagebreak -->`
/// (or `<!--pagebreak-->`) marker, ignoring surrounding whitespace and
/// comparing the keyword case-insensitively.  This is the round-trip marker
/// emitted by [`crate::md::write_markdown`] for [`ir::Block::PageBreak`].
fn is_pagebreak_marker(html: &str) -> bool {
    html_comment_keyword(html).is_some_and(|k| k.eq_ignore_ascii_case("pagebreak"))
}

/// Return `true` when the HTML block is a `<!-- header -->` opening marker.
fn is_header_start_marker(html: &str) -> bool {
    html_comment_keyword(html).is_some_and(|k| k.eq_ignore_ascii_case("header"))
}

/// Return `true` when the HTML block is a `<!-- /header -->` closing marker.
fn is_header_end_marker(html: &str) -> bool {
    html_comment_keyword(html).is_some_and(|k| k.eq_ignore_ascii_case("/header"))
}

/// Return `true` when the HTML block is a `<!-- footer -->` opening marker.
fn is_footer_start_marker(html: &str) -> bool {
    html_comment_keyword(html).is_some_and(|k| k.eq_ignore_ascii_case("footer"))
}

/// Return `true` when the HTML block is a `<!-- /footer -->` closing marker.
fn is_footer_end_marker(html: &str) -> bool {
    html_comment_keyword(html).is_some_and(|k| k.eq_ignore_ascii_case("/footer"))
}

/// Extract the single keyword inside an HTML comment of the form
/// `<!-- keyword -->` (or `<!--keyword-->`).  Returns `None` when the string
/// is not a well-formed single-keyword comment.
///
/// The trimmed outer whitespace is stripped before checking the delimiters so
/// that comrak's trailing newline in `HtmlBlock.literal` does not break the
/// match.  The inner content is returned as-is (not lowercased) so callers can
/// apply their own case comparison.
fn html_comment_keyword(html: &str) -> Option<&str> {
    let trimmed = html.trim();
    let inner = trimmed
        .strip_prefix("<!--")
        .and_then(|s| s.strip_suffix("-->"))?;
    // The inner content must be a single token (no embedded whitespace that
    // would indicate extra text) — but leading/trailing space around the
    // keyword is allowed (e.g. `<!-- header -->`).
    let keyword = inner.trim();
    // Reject if the keyword itself contains whitespace (multi-word comment).
    if keyword.contains(char::is_whitespace) {
        return None;
    }
    Some(keyword)
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
    let default_style = InlineStyle::default();
    collect_inlines_recursive(node, &mut inlines, &default_style);
    inlines
}

// Bool fields track independent Markdown inline format states (bold, italic, code, strikethrough).
#[allow(clippy::struct_excessive_bools)]
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

// Handles all CommonMark node types in inline context; cannot be decomposed.
#[allow(clippy::too_many_lines)]
fn collect_inlines_recursive<'a>(
    node: &'a AstNode<'a>,
    inlines: &mut Vec<ir::Inline>,
    style: &InlineStyle,
) {
    let mut current_style = style.clone();

    // Ruby annotation state: tracks `<ruby>…<rt>annotation</rt></ruby>` sequences
    // in the inline HTML node stream emitted by comrak.
    let mut in_ruby = false;
    let mut in_rt = false;
    let mut ruby_annotation = String::new();
    // Index into `inlines` at which the current ruby base span started.
    let mut ruby_base_start: usize = 0;

    for child in node.children() {
        let data = child.data.borrow();
        match &data.value {
            NodeValue::Text(text) => {
                // When inside <rt>…</rt>, accumulate annotation text rather than
                // emitting a regular inline.
                if in_rt {
                    ruby_annotation.push_str(text);
                    continue;
                }
                let fmt = InlineFormat {
                    bold: current_style.bold,
                    italic: current_style.italic,
                    underline: current_style.underline,
                    strikethrough: current_style.strikethrough,
                    superscript: current_style.superscript,
                    subscript: current_style.subscript,
                    color: None,
                };
                inlines.push(
                    ir::Inline::with_formatting(text.clone(), &fmt)
                        .with_link(current_style.link.clone()),
                );
            }
            NodeValue::SoftBreak | NodeValue::LineBreak => {
                if !in_rt {
                    inlines.push(ir::Inline::plain("\n"));
                }
            }
            NodeValue::Code(code) => {
                if !in_rt {
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
            }
            NodeValue::HtmlInline(html) => {
                let tag = html.trim();
                if tag.eq_ignore_ascii_case("<ruby>") {
                    if in_ruby {
                        tracing::warn!("nested <ruby> not supported, ignoring inner");
                    } else {
                        in_ruby = true;
                        in_rt = false;
                        ruby_annotation.clear();
                        ruby_base_start = inlines.len();
                    }
                } else if tag.eq_ignore_ascii_case("<rt>") {
                    in_rt = true;
                } else if tag.eq_ignore_ascii_case("</rt>") {
                    in_rt = false;
                } else if tag.eq_ignore_ascii_case("</ruby>") {
                    // Attach the accumulated annotation to every inline that
                    // was emitted as part of the ruby base span.
                    if in_ruby && !ruby_annotation.is_empty() {
                        let annotation = ruby_annotation.clone();
                        for inline in &mut inlines[ruby_base_start..] {
                            inline.ruby = Some(annotation.clone());
                        }
                    }
                    in_ruby = false;
                    in_rt = false;
                    ruby_annotation.clear();
                } else if tag.eq_ignore_ascii_case("<u>") {
                    current_style.underline = true;
                } else if tag.eq_ignore_ascii_case("</u>") {
                    current_style.underline = style.underline;
                } else if tag.eq_ignore_ascii_case("<sub>") {
                    current_style.subscript = true;
                } else if tag.eq_ignore_ascii_case("</sub>") {
                    current_style.subscript = style.subscript;
                } else {
                    inlines.push(ir::Inline::plain(html.clone()));
                }
            }
            NodeValue::Strong => {
                let mut s = current_style.clone();
                s.bold = true;
                // For ruby base spans that contain rich content (e.g. bold),
                // recurse and then patch the newly-added inlines with the ruby
                // annotation when </ruby> is encountered later.
                collect_inlines_recursive(child, inlines, &s);
            }
            NodeValue::Emph => {
                let mut s = current_style.clone();
                s.italic = true;
                collect_inlines_recursive(child, inlines, &s);
            }
            NodeValue::Strikethrough => {
                let mut s = current_style.clone();
                s.strikethrough = true;
                collect_inlines_recursive(child, inlines, &s);
            }
            NodeValue::Superscript => {
                let mut s = current_style.clone();
                s.superscript = true;
                collect_inlines_recursive(child, inlines, &s);
            }
            NodeValue::Link(link) => {
                let mut s = current_style.clone();
                s.link = Some(link.url.clone());
                collect_inlines_recursive(child, inlines, &s);
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
                collect_inlines_recursive(child, inlines, &current_style);
            }
        }
    }
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "parser_tests_inline.rs"]
mod tests_inline;

#[cfg(test)]
#[path = "parser_tests_marker.rs"]
mod tests_marker;
