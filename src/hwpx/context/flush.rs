use crate::hwp::heading_style::detect_korean_regulation_heading;
use crate::ir::{self, Inline, InlineFormat};

use super::state::FormattingState;
use super::ParseContext;

/// Code-fence hint for the next paragraph flush.
///
/// Replaces `Option<Option<String>>` on [`ParseContext::pending_code_lang`].
/// `Default` is `Plain`.
#[derive(Debug, Default, PartialEq, Clone)]
pub(crate) enum CodeLangHint {
    /// Normal paragraph (no code-fence annotation).
    #[default]
    Plain,
    /// Code block, no language (`<!-- hwp2md:lang: -->`).
    CodeNoLang,
    /// Code block with language string (`<!-- hwp2md:lang:python -->`).
    Code(String),
}

// ── Tier-3 heading thresholds (HWPX charPr `height`, 1/100 pt units) ──────
//
// These constants mirror the HWP binary reader's tier-3 logic.  Tier-3 fires
// only when tier-1/2 (styleIDRef) has already failed to assign a heading level,
// so the thresholds are tuned to real-world Korean government document heading
// sizes without risk of false-positive promotion of body text.
const HWPX_H1_MIN_HEIGHT: u32 = 1600; // >= 16 pt
const HWPX_H2_MIN_HEIGHT: u32 = 1400; // >= 14 pt
const HWPX_H3_MIN_HEIGHT: u32 = 1200; // >= 12 pt

/// Parse a boolean XML attribute value, preserving the existing value for
/// unrecognised strings (i.e. neither "true"/"1" nor "false"/"0").
fn parse_bool_preserve(val: &str, current: bool) -> bool {
    match val {
        "true" | "1" => true,
        "false" | "0" => false,
        _ => current,
    }
}

/// Resolves the effective heading level for a paragraph's inline content.
///
/// Resolution priority (highest → lowest):
/// 1. **Tier-1/2** — `style_level` already set from `styleIDRef` (HWPX style name or
///    numeric ID).  Returned immediately when `Some`.
/// 2. **Tier-3** — font height + bold heuristic via `height_hint`.  Activates only
///    when tier-1/2 is absent; requires the run to be bold and the paragraph to
///    contain fewer than 100 characters (body-text guard).
/// 3. **Tier-4** — Korean regulation text-pattern detection via
///    `detect_korean_regulation_heading`.
///
/// Both `flush_paragraph` and `flush_paragraph_staged` call this helper so the
/// level-resolution rule stays in lockstep.  List-staging precedence
/// (`if is_heading { … }`) is a separate concern handled only by
/// `flush_paragraph_staged`.
///
/// Uses `or_else` chains (not `or`) so allocations are skipped when an earlier
/// tier already produced `Some`.
///
/// Called only by [`build_block`] (top-level paragraphs).
/// Nested-scope flush paths bypass heading detection by design.
fn effective_heading_level(
    style_level: Option<u8>,
    inlines: &[Inline],
    height_hint: Option<(u32, bool)>,
) -> Option<u8> {
    style_level
        .or_else(|| {
            // Tier-3: font height + bold (HWPX charPr-based).
            let (h, bold) = height_hint?;
            if !bold {
                return None;
            }
            // Guard: paragraphs with >= 100 characters are likely body text even
            // when they happen to use a large, bold font.
            let char_count: usize = inlines.iter().map(|i| i.text.chars().count()).sum();
            if char_count >= 100 {
                return None;
            }
            if h >= HWPX_H1_MIN_HEIGHT {
                Some(1)
            } else if h >= HWPX_H2_MIN_HEIGHT {
                Some(2)
            } else if h >= HWPX_H3_MIN_HEIGHT {
                Some(3)
            } else {
                None
            }
        })
        .or_else(|| {
            // Tier-4: Korean regulation text pattern.
            let combined: String = inlines.iter().map(|i| i.text.as_str()).collect();
            detect_korean_regulation_heading(&combined)
        })
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
            "height" | "hp:height" => {
                if let Ok(h) = val.as_ref().parse::<u32>() {
                    ctx.fmt.font_height = Some(h);
                }
            }
            _ => {}
        }
    }

    if let Some(idx) = face_id {
        ctx.fmt.font_name = ctx.face_names.get(idx).cloned();
    }

    // Update the per-paragraph max-height tracker used by tier-3 heading detection.
    if let Some(h) = ctx.fmt.font_height {
        if h > ctx.para_max_font_height {
            ctx.para_max_font_height = h;
            ctx.para_max_font_height_bold = ctx.fmt.bold;
        }
    }
}

/// Attempt to build a `CodeBlock` from `code_lang`.
///
/// Returns `Ok(block)` when the hint specifies a code block,
/// `Err(inlines)` when the hint is `Plain` (inlines returned to caller).
fn try_code_block(
    code_lang: CodeLangHint,
    inlines: Vec<ir::Inline>,
) -> Result<ir::Block, Vec<ir::Inline>> {
    match code_lang {
        CodeLangHint::CodeNoLang => Ok(ir::Block::CodeBlock {
            language: None,
            code: collect_inline_text(inlines),
        }),
        CodeLangHint::Code(lang) => Ok(ir::Block::CodeBlock {
            language: Some(lang),
            code: collect_inline_text(inlines),
        }),
        CodeLangHint::Plain => Err(inlines),
    }
}

/// Drain accumulated `text` + `inlines` into `blocks` as a `Paragraph` or `CodeBlock`.
///
/// When `code_lang` is `Plain`, always produces a `Paragraph` — heading detection
/// is deliberately absent here because this function is only called from nested
/// scopes (cell, list-item, footnote, header, footer).
///
/// This function is called from nested-scope flush paths (cell, list-item,
/// footnote, header, footer).  The `Plain` fallback deliberately emits
/// `Paragraph` **without** any heading detection (Tier-1 through Tier-4).
///
/// **Policy**: Nested-scope paragraphs do not become headings, even when
/// they match the Korean-regulation pattern (`제N조`, `제N장`).  Heading
/// promotion is reserved for top-level paragraphs (see `build_block`).
/// This invariant is enforced by the regression test
/// `nested_scope_korean_regulation_text_stays_paragraph_not_heading`.
fn flush_inlines_to_blocks(
    text: &mut String,
    inlines: &mut Vec<ir::Inline>,
    blocks: &mut Vec<ir::Block>,
    fmt: &FormattingState,
    code_lang: CodeLangHint,
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
        let block = try_code_block(code_lang, i)
            .unwrap_or_else(|i| ir::Block::Paragraph { inlines: i });
        blocks.push(block);
    }
}

/// Build an `ir::Inline` from the current formatting state.
fn make_inline(text: String, fmt: &FormattingState) -> ir::Inline {
    ir::Inline::with_formatting(text, &InlineFormat::from(fmt))
        .with_font_name(fmt.font_name.clone())
}

/// Consume a `Vec<ir::Inline>` and concatenate all inline text runs into one `String`.
fn collect_inline_text(inlines: Vec<ir::Inline>) -> String {
    inlines.into_iter().map(|i| i.text).collect()
}

/// Build a paragraph-level [`ir::Block`] from its components.
///
/// `code_lang` is a [`CodeLangHint`] that signals whether the paragraph should
/// become a [`ir::Block::CodeBlock`] and which language to annotate.  When
/// `CodeNoLang` or `Code(_)` the inlines are concatenated into a `CodeBlock`.
/// When `Plain`, [`effective_heading_level`] decides between
/// [`ir::Block::Heading`] and [`ir::Block::Paragraph`].
///
/// `height_hint` is `Some((max_height, was_bold))` carrying the per-paragraph
/// font-height tracker values for tier-3 heading detection.  Pass `None` when
/// the tracker is unavailable (e.g. non-top-level flush paths).
///
/// Both `flush_paragraph` and `flush_paragraph_staged` delegate here so CodeBlock / Heading /
/// Paragraph construction stays in one place.
///
/// Unlike [`flush_inlines_to_blocks`], this function applies the full
/// heading-detection pipeline (Tier-1 through Tier-4) for `Plain` hints.
fn build_block(
    inlines: Vec<ir::Inline>,
    code_lang: CodeLangHint,
    heading_level: Option<u8>,
    height_hint: Option<(u32, bool)>,
) -> ir::Block {
    try_code_block(code_lang, inlines).unwrap_or_else(|inlines| {
        let effective_level = effective_heading_level(heading_level, &inlines, height_hint);
        if let Some(level) = effective_level {
            ir::Block::Heading { level, inlines }
        } else {
            ir::Block::Paragraph { inlines }
        }
    })
}

/// Flush any pending paragraph inlines to `section.blocks` (test-only).
#[cfg(test)]
pub(crate) fn flush_paragraph(ctx: &mut ParseContext, section: &mut ir::Section) {
    if !ctx.current_text.is_empty() {
        let t = std::mem::take(&mut ctx.current_text);
        ctx.current_inlines.push(make_inline(t, &ctx.fmt));
    }

    let code_lang = std::mem::take(&mut ctx.pending_code_lang);

    if ctx.current_inlines.is_empty() {
        return;
    }

    let height_hint = Some((ctx.para_max_font_height, ctx.para_max_font_height_bold));
    let inlines = std::mem::take(&mut ctx.current_inlines);
    section.blocks.push(build_block(
        inlines,
        code_lang,
        ctx.heading_level,
        height_hint,
    ));
}

/// Variant of `flush_paragraph` used during OWPML flat-paragraph list
/// parsing.  Returns a [`StagedBlock`] for the staging vector.
pub(crate) fn flush_paragraph_staged(ctx: &mut ParseContext) -> Option<StagedBlock> {
    if !ctx.current_text.is_empty() {
        let t = std::mem::take(&mut ctx.current_text);
        ctx.current_inlines.push(make_inline(t, &ctx.fmt));
    }

    let para_pr_id = ctx.current_para_pr_id.take();
    let num_pr_id = ctx.current_num_pr_id.take();
    let code_lang = std::mem::take(&mut ctx.pending_code_lang);

    if ctx.current_inlines.is_empty() {
        return None;
    }

    let height_hint = Some((ctx.para_max_font_height, ctx.para_max_font_height_bold));
    let inlines = std::mem::take(&mut ctx.current_inlines);
    let block = build_block(inlines, code_lang, ctx.heading_level, height_hint);

    // Only Paragraph blocks are list-stageable: Heading and CodeBlock are always Plain.
    // A tier-4 regulation heading (e.g. "제1편 총칙") must not be re-staged as a ListPara
    // even if paraPrIDRef looks list-like; likewise a CodeBlock is never a list item.
    let list_depth: Option<u32> = if matches!(block, ir::Block::Paragraph { .. }) {
        match para_pr_id.as_deref() {
            Some("2") => Some(0),
            Some("3") => Some(1),
            Some(s) if s.parse::<u32>().ok().is_some_and(|n| n >= 4) => Some(1),
            _ => None,
        }
    } else {
        None
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

pub(crate) fn flush_cell_paragraph(ctx: &mut ParseContext, code_lang: CodeLangHint) {
    flush_inlines_to_blocks(
        &mut ctx.table.cell_text,
        &mut ctx.table.cell_inlines,
        &mut ctx.table.cell_blocks,
        &ctx.fmt,
        code_lang,
    );
}

pub(crate) fn flush_list_item_paragraph(ctx: &mut ParseContext, code_lang: CodeLangHint) {
    flush_inlines_to_blocks(
        &mut ctx.list.item_text,
        &mut ctx.list.item_inlines,
        &mut ctx.list.item_blocks,
        &ctx.fmt,
        code_lang,
    );
}

/// Flush pending footnote/endnote paragraph into `footnote.blocks`.
pub(crate) fn flush_footnote_paragraph(ctx: &mut ParseContext, code_lang: CodeLangHint) {
    flush_inlines_to_blocks(
        &mut ctx.footnote.text,
        &mut ctx.footnote.inlines,
        &mut ctx.footnote.blocks,
        &ctx.fmt,
        code_lang,
    );
}

/// Flush pending header paragraph into `header_footer.header_blocks`.
pub(crate) fn flush_header_paragraph(ctx: &mut ParseContext, code_lang: CodeLangHint) {
    flush_inlines_to_blocks(
        &mut ctx.header_footer.text,
        &mut ctx.header_footer.inlines,
        &mut ctx.header_footer.header_blocks,
        &ctx.fmt,
        code_lang,
    );
}

/// Flush pending footer paragraph into `header_footer.footer_blocks`.
pub(crate) fn flush_footer_paragraph(ctx: &mut ParseContext, code_lang: CodeLangHint) {
    flush_inlines_to_blocks(
        &mut ctx.header_footer.text,
        &mut ctx.header_footer.inlines,
        &mut ctx.header_footer.footer_blocks,
        &ctx.fmt,
        code_lang,
    );
}

/// Flush the active nested scope (header, footer, footnote, list-item, or cell) if one is open.
///
/// Returns `true` when a nested scope was flushed; `false` at top level.
/// Callers that return `false` are responsible for the top-level flush.
///
/// Branch order: header → footer → footnote → **cell → list** (cell-first).
///
/// `pending_code_lang` is consumed and forwarded to the nested-scope flush so
/// that code-fence annotations (`<!-- hwp2md:lang:LANG -->`) take effect inside
/// cells, list items, footnotes, headers, and footers.
///
/// The `active` guard on the header/footer branches matches the routing logic
/// in [`ParseContext::active_text_buf`] and [`ParseContext::push_inline`], which
/// both require `header_footer.active && (in_header || in_footer)` before
/// directing content into the header/footer buffers.  Without the `active`
/// check, a stale `in_header`/`in_footer` flag left over after the
/// `</hp:headerFooter>` closing tag could cause a spurious flush into the
/// wrong block list.
pub(crate) fn flush_nested_scope(ctx: &mut ParseContext) -> bool {
    if ctx.header_footer.in_header_active() {
        let code_lang = std::mem::take(&mut ctx.pending_code_lang);
        flush_header_paragraph(ctx, code_lang);
    } else if ctx.header_footer.in_footer_active() {
        let code_lang = std::mem::take(&mut ctx.pending_code_lang);
        flush_footer_paragraph(ctx, code_lang);
    } else if ctx.footnote.active {
        let code_lang = std::mem::take(&mut ctx.pending_code_lang);
        flush_footnote_paragraph(ctx, code_lang);
    } else if ctx.table.in_cell {
        let code_lang = std::mem::take(&mut ctx.pending_code_lang);
        flush_cell_paragraph(ctx, code_lang);
    } else if ctx.list.in_item {
        let code_lang = std::mem::take(&mut ctx.pending_code_lang);
        flush_list_item_paragraph(ctx, code_lang);
    } else {
        return false;
    }
    true
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
    if flush_nested_scope(ctx) {
        return None;
    }
    let code_lang = std::mem::take(&mut ctx.pending_code_lang);
    let mut top: Vec<ir::Block> = Vec::new();
    flush_inlines_to_blocks(
        &mut ctx.current_text,
        &mut ctx.current_inlines,
        &mut top,
        &ctx.fmt,
        code_lang,
    );
    top.pop().map(StagedBlock::Plain)
}
