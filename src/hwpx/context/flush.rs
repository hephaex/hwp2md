use crate::ir::{self, InlineFormat};

use super::state::FormattingState;
use super::ParseContext;

/// Parse a boolean XML attribute value, preserving the existing value for
/// unrecognised strings (i.e. neither "true"/"1" nor "false"/"0").
fn parse_bool_preserve(val: &str, current: bool) -> bool {
    match val {
        "true" | "1" => true,
        "false" | "0" => false,
        _ => current,
    }
}

/// Parse `bold`, `italic`, `underline`, `strikeout`, and font-face attributes
/// from a `<charPr>` or `<hp:charPr>` element and write them onto `ctx`.
///
/// Called from both `handle_start_element` (non-self-closing variant) and
/// `handle_empty_element` (self-closing variant) so the two paths are
/// guaranteed to behave identically.
pub(crate) fn apply_charpr_attrs(e: &quick_xml::events::BytesStart, ctx: &mut ParseContext) {
    let mut face_id: Option<usize> = None;

    for attr in e.attributes().flatten() {
        let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
        let val = attr.unescape_value().unwrap_or_default();
        match key {
            "bold" | "hp:bold" => {
                ctx.fmt.bold = parse_bool_preserve(val.as_ref(), ctx.fmt.bold);
            }
            "italic" | "hp:italic" => {
                ctx.fmt.italic = parse_bool_preserve(val.as_ref(), ctx.fmt.italic);
            }
            "underline" | "hp:underline" => {
                ctx.fmt.underline =
                    !val.is_empty() && val.as_ref() != "none" && val.as_ref() != "0";
            }
            "strikeout" | "hp:strikeout" => {
                ctx.fmt.strike = !val.is_empty() && val.as_ref() != "none" && val.as_ref() != "0";
            }
            "supscript" | "hp:supscript" => {
                ctx.fmt.superscript = val.as_ref() == "superscript";
                ctx.fmt.subscript = val.as_ref() == "subscript";
            }
            "color" | "hp:color" => {
                let raw = val.as_ref().trim_start_matches('#');
                if raw.is_empty() || raw.eq_ignore_ascii_case("000000") {
                    ctx.fmt.color = None;
                } else {
                    ctx.fmt.color = Some(format!("#{}", raw.to_ascii_uppercase()));
                }
            }
            "faceNameIDRef" | "hp:faceNameIDRef" | "hangulIDRef" | "hp:hangulIDRef" => {
                if let Ok(idx) = val.as_ref().parse::<usize>() {
                    face_id = Some(idx);
                }
            }
            _ => {}
        }
    }

    if let Some(idx) = face_id {
        ctx.fmt.font_name = ctx.face_names.get(idx).cloned();
    }
}

/// Drain accumulated `text` + `inlines` into `blocks` as a `Paragraph`.
fn flush_inlines_to_blocks(
    text: &mut String,
    inlines: &mut Vec<ir::Inline>,
    blocks: &mut Vec<ir::Block>,
    fmt: &FormattingState,
) {
    if !text.is_empty() {
        let t = std::mem::take(text);
        inlines.push(
            ir::Inline::with_formatting(t, &InlineFormat::from(fmt))
                .with_font_name(fmt.font_name.clone()),
        );
    }
    if !inlines.is_empty() {
        let i = std::mem::take(inlines);
        blocks.push(ir::Block::Paragraph { inlines: i });
    }
}

/// Build an `ir::Inline` from the current formatting state.
fn make_inline(text: String, fmt: &FormattingState) -> ir::Inline {
    ir::Inline::with_formatting(text, &InlineFormat::from(fmt))
        .with_font_name(fmt.font_name.clone())
}

/// Flush any pending paragraph inlines to `section.blocks` (test-only).
#[cfg(test)]
pub(crate) fn flush_paragraph(ctx: &mut ParseContext, section: &mut ir::Section) {
    if !ctx.current_text.is_empty() {
        let t = std::mem::take(&mut ctx.current_text);
        ctx.current_inlines.push(make_inline(t, &ctx.fmt));
    }

    let code_lang = ctx.pending_code_lang.take();

    if ctx.current_inlines.is_empty() {
        return;
    }

    let inlines = std::mem::take(&mut ctx.current_inlines);

    if let Some(language) = code_lang {
        let code = inlines.into_iter().map(|i| i.text).collect::<String>();
        section.blocks.push(ir::Block::CodeBlock { language, code });
        return;
    }

    let block = if let Some(level) = ctx.heading_level {
        ir::Block::Heading { level, inlines }
    } else {
        ir::Block::Paragraph { inlines }
    };
    section.blocks.push(block);
}

/// Variant of [`flush_paragraph`] used during OWPML flat-paragraph list
/// parsing.  Returns a [`StagedBlock`] for the staging vector.
pub(crate) fn flush_paragraph_staged(ctx: &mut ParseContext) -> Option<StagedBlock> {
    if !ctx.current_text.is_empty() {
        let t = std::mem::take(&mut ctx.current_text);
        ctx.current_inlines.push(make_inline(t, &ctx.fmt));
    }

    let para_pr_id = ctx.current_para_pr_id.take();
    let num_pr_id = ctx.current_num_pr_id.take();
    let code_lang = ctx.pending_code_lang.take();

    if ctx.current_inlines.is_empty() {
        return None;
    }

    let inlines = std::mem::take(&mut ctx.current_inlines);

    if let Some(language) = code_lang {
        let code = inlines.into_iter().map(|i| i.text).collect::<String>();
        return Some(StagedBlock::Plain(ir::Block::CodeBlock { language, code }));
    }

    let block = if let Some(level) = ctx.heading_level {
        ir::Block::Heading { level, inlines }
    } else {
        ir::Block::Paragraph { inlines }
    };

    let is_heading = ctx.heading_level.is_some();
    let list_depth: Option<u32> = if is_heading {
        None
    } else {
        match para_pr_id.as_deref() {
            Some("2") => Some(0),
            Some("3") => Some(1),
            Some(s) if s.parse::<u32>().ok().is_some_and(|n| n >= 4) => Some(1),
            _ => None,
        }
    };

    Some(if let Some(depth) = list_depth {
        let ordered = num_pr_id.as_deref() == Some("1");
        StagedBlock::ListPara {
            depth,
            ordered,
            block,
        }
    } else {
        StagedBlock::Plain(block)
    })
}

/// An intermediate block produced during OWPML section parsing.
#[derive(Debug)]
pub(crate) enum StagedBlock {
    Plain(ir::Block),
    ListPara {
        depth: u32,
        ordered: bool,
        block: ir::Block,
    },
}

/// Collapse a flat sequence of [`StagedBlock`]s into nested `Block::List`.
pub(crate) fn group_list_paragraphs(staged: Vec<StagedBlock>) -> Vec<ir::Block> {
    let mut out: Vec<ir::Block> = Vec::with_capacity(staged.len());
    let mut pending: Vec<(u32, bool, ir::Block)> = Vec::new();

    let flush_pending = |pending: &mut Vec<(u32, bool, ir::Block)>, out: &mut Vec<ir::Block>| {
        if pending.is_empty() {
            return;
        }
        let list = build_list(std::mem::take(pending));
        out.push(list);
    };

    for staged_block in staged {
        match staged_block {
            StagedBlock::Plain(block) => {
                flush_pending(&mut pending, &mut out);
                out.push(block);
            }
            StagedBlock::ListPara {
                depth,
                ordered,
                block,
            } => {
                pending.push((depth, ordered, block));
            }
        }
    }
    flush_pending(&mut pending, &mut out);

    out
}

fn build_list(entries: Vec<(u32, bool, ir::Block)>) -> ir::Block {
    if entries.is_empty() {
        return ir::Block::List {
            ordered: false,
            start: 1,
            items: vec![],
        };
    }

    let top_ordered = entries[0].1;
    let mut items: Vec<ir::ListItem> = Vec::new();

    for (depth, _ordered, block) in entries {
        if depth == 0 || items.is_empty() {
            items.push(ir::ListItem {
                blocks: vec![block],
                children: vec![],
                checked: None,
            });
        } else {
            let Some(parent) = items.last_mut() else {
                continue;
            };
            parent.children.push(ir::ListItem {
                blocks: vec![block],
                children: vec![],
                checked: None,
            });
        }
    }

    ir::Block::List {
        ordered: top_ordered,
        start: 1,
        items,
    }
}

pub(crate) fn flush_cell_paragraph(ctx: &mut ParseContext) {
    flush_inlines_to_blocks(
        &mut ctx.table.cell_text,
        &mut ctx.table.cell_inlines,
        &mut ctx.table.cell_blocks,
        &ctx.fmt,
    );
}

pub(crate) fn flush_list_item_paragraph(ctx: &mut ParseContext) {
    flush_inlines_to_blocks(
        &mut ctx.list.item_text,
        &mut ctx.list.item_inlines,
        &mut ctx.list.item_blocks,
        &ctx.fmt,
    );
}

/// Flush pending footnote/endnote paragraph into `footnote.blocks`.
pub(crate) fn flush_footnote_paragraph(ctx: &mut ParseContext) {
    flush_inlines_to_blocks(
        &mut ctx.footnote.text,
        &mut ctx.footnote.inlines,
        &mut ctx.footnote.blocks,
        &ctx.fmt,
    );
}

/// Flush pending header paragraph into `header_footer.header_blocks`.
pub(crate) fn flush_header_paragraph(ctx: &mut ParseContext) {
    flush_inlines_to_blocks(
        &mut ctx.header_footer.text,
        &mut ctx.header_footer.inlines,
        &mut ctx.header_footer.header_blocks,
        &ctx.fmt,
    );
}

/// Flush pending footer paragraph into `header_footer.footer_blocks`.
pub(crate) fn flush_footer_paragraph(ctx: &mut ParseContext) {
    flush_inlines_to_blocks(
        &mut ctx.header_footer.text,
        &mut ctx.header_footer.inlines,
        &mut ctx.header_footer.footer_blocks,
        &ctx.fmt,
    );
}

/// Flush whichever scope is currently active (footnote → list-item → cell →
/// top-level paragraph) so that any buffered inline run becomes a finished
/// block before the caller stages a sibling block.
///
/// At top level the accumulated `current_text`/`current_inlines` is wrapped
/// in a `Paragraph` and returned via `Option<StagedBlock>` for the caller to
/// append to its staging vector — mirroring the contract used by
/// [`flush_paragraph_staged`].  In every nested scope the flush stays
/// in-context and `None` is returned.
#[must_use = "top-level paragraph must be appended to the section staging vector"]
pub(crate) fn flush_active_paragraph_scope(ctx: &mut ParseContext) -> Option<StagedBlock> {
    if ctx.header_footer.in_header {
        flush_header_paragraph(ctx);
        None
    } else if ctx.header_footer.in_footer {
        flush_footer_paragraph(ctx);
        None
    } else if ctx.footnote.active {
        flush_footnote_paragraph(ctx);
        None
    } else if ctx.list.in_item {
        flush_list_item_paragraph(ctx);
        None
    } else if ctx.table.in_cell {
        flush_cell_paragraph(ctx);
        None
    } else {
        let mut top: Vec<ir::Block> = Vec::new();
        flush_inlines_to_blocks(
            &mut ctx.current_text,
            &mut ctx.current_inlines,
            &mut top,
            &ctx.fmt,
        );
        top.pop().map(StagedBlock::Plain)
    }
}
