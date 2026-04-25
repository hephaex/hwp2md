use crate::ir;

use super::context::{
    apply_charpr_attrs, flush_cell_paragraph, flush_footnote_paragraph, flush_list_item_paragraph,
    flush_paragraph, ParseContext, RubyPart,
};

pub(super) fn handle_start_element(
    local: &str,
    e: &quick_xml::events::BytesStart,
    ctx: &mut ParseContext,
) {
    match local {
        "p" | "hp:p" => {
            ctx.in_paragraph = true;
            ctx.current_text.clear();
            ctx.current_inlines.clear();
            ctx.heading_level = None;

            for attr in e.attributes().flatten() {
                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                if key == "styleIDRef" || key == "hp:styleIDRef" {
                    let val = attr.unescape_value().unwrap_or_default().to_string();
                    if let Some(level) = parse_heading_style(&val) {
                        ctx.heading_level = Some(level);
                    }
                }
            }
        }
        "run" | "hp:run" => {
            ctx.in_run = true;
            ctx.current_bold = false;
            ctx.current_italic = false;
            ctx.current_underline = false;
            ctx.current_strike = false;
            ctx.current_superscript = false;
            ctx.current_subscript = false;
            ctx.current_color = None;
        }
        "charPr" | "hp:charPr" => apply_charpr_attrs(e, ctx),
        "t" | "hp:t" => {
            ctx.in_text = true;
        }
        "tbl" | "hp:tbl" => {
            ctx.in_table = true;
            ctx.table_rows.clear();
            ctx.col_count = 0;
            for attr in e.attributes().flatten() {
                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                if key == "colCnt" || key == "hp:colCnt" {
                    if let Ok(n) = attr.unescape_value().unwrap_or_default().parse::<usize>() {
                        ctx.col_count = n;
                    }
                }
            }
        }
        "tr" | "hp:tr" => {
            ctx.current_row_cells.clear();
        }
        "tc" | "hp:tc" => {
            ctx.in_cell = true;
            ctx.cell_blocks.clear();
            ctx.cell_inlines.clear();
            ctx.cell_text.clear();
            ctx.current_colspan = 1;
            ctx.current_rowspan = 1;
        }
        "ol" => {
            ctx.in_list = true;
            ctx.list_ordered = true;
            ctx.list_items.clear();
        }
        "ul" => {
            ctx.in_list = true;
            ctx.list_ordered = false;
            ctx.list_items.clear();
        }
        "li" | "hp:li" => {
            ctx.in_list_item = true;
            ctx.list_item_blocks.clear();
            ctx.list_item_inlines.clear();
            ctx.list_item_text.clear();
        }
        "equation" | "hp:equation" | "eqEdit" | "hp:eqEdit" => {
            ctx.in_equation = true;
            ctx.equation_text.clear();
        }
        // <hp:fieldBegin type="HYPERLINK" command="https://..."> (non-self-closing)
        "fieldBegin" | "hp:fieldBegin" => {
            let mut field_type = String::new();
            let mut command = String::new();
            for attr in e.attributes().flatten() {
                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                let val = attr.unescape_value().unwrap_or_default().to_string();
                match key {
                    "type" | "hp:type" => field_type = val,
                    "command" | "hp:command" => command = val,
                    _ => {}
                }
            }
            if field_type == "HYPERLINK" && !command.is_empty() {
                ctx.in_hyperlink = true;
                ctx.hyperlink_url = Some(command);
            }
        }
        "ruby" | "hp:ruby" => {
            ctx.in_ruby = true;
            ctx.ruby_base_text.clear();
            ctx.ruby_annotation_text.clear();
            ctx.ruby_current_part = RubyPart::None;
        }
        "rubyText" | "hp:rubyText" => {
            ctx.ruby_current_part = RubyPart::Annotation;
        }
        "baseText" | "hp:baseText" => {
            ctx.ruby_current_part = RubyPart::Base;
        }
        "fn" | "hp:fn" | "footnote" | "hp:footnote" | "en" | "hp:en" | "endnote" | "hp:endnote" => {
            let mut id = String::new();
            for attr in e.attributes().flatten() {
                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                if key == "id" || key == "hp:id" || key == "noteId" || key == "hp:noteId" {
                    id = attr.unescape_value().unwrap_or_default().to_string();
                    break;
                }
            }
            ctx.in_footnote = true;
            ctx.footnote_id = id;
            ctx.footnote_blocks.clear();
            ctx.footnote_inlines.clear();
            ctx.footnote_text.clear();
        }
        _ => {}
    }
}

pub(super) fn handle_end_element(local: &str, ctx: &mut ParseContext, section: &mut ir::Section) {
    match local {
        "p" | "hp:p" => {
            if ctx.in_footnote {
                flush_footnote_paragraph(ctx);
            } else if ctx.in_cell {
                flush_cell_paragraph(ctx);
            } else if ctx.in_list_item {
                flush_list_item_paragraph(ctx);
            } else {
                flush_paragraph(ctx, section);
            }
            ctx.in_paragraph = false;
        }
        "run" | "hp:run" => {
            ctx.in_run = false;
        }
        "t" | "hp:t" => {
            ctx.in_text = false;
            if !ctx.current_text.is_empty() {
                let text = std::mem::take(&mut ctx.current_text);
                let inline = ir::Inline {
                    text,
                    bold: ctx.current_bold,
                    italic: ctx.current_italic,
                    underline: ctx.current_underline,
                    strikethrough: ctx.current_strike,
                    superscript: ctx.current_superscript,
                    subscript: ctx.current_subscript,
                    color: ctx.current_color.clone(),
                    link: if ctx.in_hyperlink {
                        ctx.hyperlink_url.clone()
                    } else {
                        None
                    },
                    ..ir::Inline::default()
                };
                ctx.push_inline(inline);
            }
        }
        "tbl" | "hp:tbl" => {
            let col_count = ctx.col_count.max(
                ctx.table_rows
                    .iter()
                    .map(|r| r.cells.len())
                    .max()
                    .unwrap_or(0),
            );
            if !ctx.table_rows.is_empty() {
                let rows = std::mem::take(&mut ctx.table_rows);
                section.blocks.push(ir::Block::Table { rows, col_count });
            }
            ctx.in_table = false;
        }
        "tr" | "hp:tr" => {
            let cells = std::mem::take(&mut ctx.current_row_cells);
            ctx.table_rows.push(ir::TableRow {
                cells,
                is_header: ctx.table_rows.is_empty(),
            });
        }
        "tc" | "hp:tc" => {
            flush_cell_paragraph(ctx);
            let blocks = std::mem::take(&mut ctx.cell_blocks);
            ctx.current_row_cells.push(ir::TableCell {
                blocks,
                colspan: ctx.current_colspan,
                rowspan: ctx.current_rowspan,
            });
            ctx.in_cell = false;
        }
        "li" | "hp:li" => {
            flush_list_item_paragraph(ctx);
            let blocks = std::mem::take(&mut ctx.list_item_blocks);
            ctx.list_items.push(ir::ListItem {
                blocks,
                children: Vec::new(),
            });
            ctx.in_list_item = false;
        }
        "ol" | "ul" => {
            if !ctx.list_items.is_empty() {
                let items = std::mem::take(&mut ctx.list_items);
                section.blocks.push(ir::Block::List {
                    ordered: ctx.list_ordered,
                    start: 1,
                    items,
                });
            }
            ctx.in_list = false;
        }
        "equation" | "hp:equation" | "eqEdit" | "hp:eqEdit" => {
            if !ctx.equation_text.is_empty() {
                let tex = std::mem::take(&mut ctx.equation_text);
                section.blocks.push(ir::Block::Math { display: true, tex });
            }
            ctx.in_equation = false;
        }
        // Non-self-closing fieldEnd (closing tag clears hyperlink state)
        "fieldEnd" | "hp:fieldEnd" => {
            ctx.in_hyperlink = false;
            ctx.hyperlink_url = None;
        }
        // Non-self-closing fieldBegin end tag (no-op; state set on open)
        "fieldBegin" | "hp:fieldBegin" => {}
        "rubyText" | "hp:rubyText" | "baseText" | "hp:baseText" => {
            ctx.ruby_current_part = RubyPart::None;
        }
        "ruby" | "hp:ruby" => {
            let base = std::mem::take(&mut ctx.ruby_base_text);
            let annotation = std::mem::take(&mut ctx.ruby_annotation_text);
            if !base.is_empty() || !annotation.is_empty() {
                let inline = ir::Inline {
                    text: base,
                    ruby: if annotation.is_empty() {
                        None
                    } else {
                        Some(annotation)
                    },
                    ..ir::Inline::default()
                };
                ctx.push_inline(inline);
            }
            ctx.in_ruby = false;
            ctx.ruby_current_part = RubyPart::None;
        }
        "fn" | "hp:fn" | "footnote" | "hp:footnote" | "en" | "hp:en" | "endnote" | "hp:endnote" => {
            flush_footnote_paragraph(ctx);
            if !ctx.footnote_blocks.is_empty() {
                let id = std::mem::take(&mut ctx.footnote_id);
                let content = std::mem::take(&mut ctx.footnote_blocks);
                section.blocks.push(ir::Block::Footnote { id, content });
            } else {
                ctx.footnote_id.clear();
            }
            ctx.in_footnote = false;
        }
        _ => {}
    }
}

pub(super) fn handle_text(text: &str, ctx: &mut ParseContext) {
    if ctx.in_equation {
        ctx.equation_text.push_str(text);
        return;
    }
    if ctx.in_ruby {
        match ctx.ruby_current_part {
            RubyPart::Base => ctx.ruby_base_text.push_str(text),
            RubyPart::Annotation => ctx.ruby_annotation_text.push_str(text),
            RubyPart::None => {}
        }
        return;
    }
    if ctx.in_run && ctx.in_text {
        ctx.active_text_buf().push_str(text);
    }
}

pub(super) fn handle_empty_element(
    local: &str,
    e: &quick_xml::events::BytesStart,
    ctx: &mut ParseContext,
    section: &mut ir::Section,
) {
    match local {
        "img" | "hp:img" | "picture" | "hp:picture" => {
            let mut src = String::new();
            let mut alt = String::new();
            for attr in e.attributes().flatten() {
                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                let val = attr.unescape_value().unwrap_or_default().to_string();
                match key {
                    "src" | "href" | "hp:href" | "binaryItemIDRef" | "hp:binaryItemIDRef" => {
                        src = val;
                    }
                    "alt" => alt = val,
                    _ => {}
                }
            }
            if !src.is_empty() {
                let img = ir::Block::Image { src, alt };
                if let Some(block) = ctx.push_block_scoped(img) {
                    section.blocks.push(block);
                }
            }
        }
        "lineBreak" | "hp:lineBreak" => {
            ctx.active_text_buf().push('\n');
        }
        // <hp:cellAddr colAddr="0" rowAddr="0" colSpan="2" rowSpan="1"/>
        // Appears as a self-closing child inside <hp:tc>.
        "cellAddr" | "hp:cellAddr" => {
            for attr in e.attributes().flatten() {
                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                let val = attr.unescape_value().unwrap_or_default();
                match key {
                    "colSpan" | "hp:colSpan" => {
                        if let Ok(n) = val.parse::<u32>() {
                            if n >= 1 {
                                ctx.current_colspan = n;
                            }
                        }
                    }
                    "rowSpan" | "hp:rowSpan" => {
                        if let Ok(n) = val.parse::<u32>() {
                            if n >= 1 {
                                ctx.current_rowspan = n;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        // <hp:charPr bold="true" italic="true" .../>  (self-closing variant)
        // Delegates to apply_charpr_attrs -- same logic as the Start element path.
        "charPr" | "hp:charPr" => apply_charpr_attrs(e, ctx),
        // <hp:fieldBegin type="HYPERLINK" command="https://..." />
        "fieldBegin" | "hp:fieldBegin" => {
            let mut field_type = String::new();
            let mut command = String::new();
            for attr in e.attributes().flatten() {
                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                let val = attr.unescape_value().unwrap_or_default().to_string();
                match key {
                    "type" | "hp:type" => field_type = val,
                    "command" | "hp:command" => command = val,
                    _ => {}
                }
            }
            if field_type == "HYPERLINK" && !command.is_empty() {
                ctx.in_hyperlink = true;
                ctx.hyperlink_url = Some(command);
            }
        }
        // <hp:fieldEnd type="HYPERLINK" />
        "fieldEnd" | "hp:fieldEnd" => {
            ctx.in_hyperlink = false;
            ctx.hyperlink_url = None;
        }
        // Footnote / endnote reference inline: a self-closing marker that records
        // which footnote the current text position cites.
        //
        // Accepted forms:
        //   <hp:noteRef noteId="1"/>
        //   <hp:ctrl id="fn" idRef="1"/>
        //   <hp:ctrl id="en" idRef="1"/>
        "noteRef" | "hp:noteRef" => {
            let mut note_id = String::new();
            for attr in e.attributes().flatten() {
                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                if key == "noteId" || key == "hp:noteId" || key == "id" || key == "hp:id" {
                    note_id = attr.unescape_value().unwrap_or_default().to_string();
                    break;
                }
            }
            if !note_id.is_empty() {
                let inline = ir::Inline {
                    footnote_ref: Some(note_id),
                    ..ir::Inline::default()
                };
                ctx.push_inline(inline);
            }
        }
        // <hp:ctrl id="fn" idRef="1"/> -- HWP-binary-style ctrl inline.
        "ctrl" | "hp:ctrl" => {
            let mut ctrl_kind = String::new();
            let mut id_ref = String::new();
            for attr in e.attributes().flatten() {
                let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                let val = attr.unescape_value().unwrap_or_default().to_string();
                match key {
                    "id" | "hp:id" => ctrl_kind = val,
                    "idRef" | "hp:idRef" => id_ref = val,
                    _ => {}
                }
            }
            if (ctrl_kind == "fn" || ctrl_kind == "en") && !id_ref.is_empty() {
                let inline = ir::Inline {
                    footnote_ref: Some(id_ref),
                    ..ir::Inline::default()
                };
                ctx.push_inline(inline);
            }
        }
        _ => {}
    }
}

/// Parse a HWP style ID reference and return the heading level (1–6) if it
/// identifies a heading style, or `None` otherwise.
pub(crate) fn parse_heading_style(style_ref: &str) -> Option<u8> {
    // Numeric style IDs: "1"–"6" map directly to heading levels.
    if let Ok(n) = style_ref.parse::<u8>() {
        if (1..=6).contains(&n) {
            return Some(n);
        }
    }

    let lower = style_ref.to_lowercase();
    if lower.contains("heading") || lower.contains("제목") || lower.contains("개요") {
        let num_str: String = style_ref
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        if let Ok(n) = num_str.parse::<u8>() {
            if (1..=6).contains(&n) {
                return Some(n);
            }
        }
        return Some(1);
    }
    None
}
