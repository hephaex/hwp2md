use crate::ir;

// ── Sub-struct definitions ──────────────────────────────────────────────

/// Character-level formatting state for the current XML run.
#[derive(Debug, Default)]
pub(crate) struct FormattingState {
    pub(crate) bold: bool,
    pub(crate) italic: bool,
    pub(crate) underline: bool,
    pub(crate) strike: bool,
    pub(crate) superscript: bool,
    pub(crate) subscript: bool,
    pub(crate) color: Option<String>,
    pub(crate) font_name: Option<String>,
}

impl FormattingState {
    pub(crate) fn reset(&mut self) {
        self.bold = false;
        self.italic = false;
        self.underline = false;
        self.strike = false;
        self.superscript = false;
        self.subscript = false;
        self.color = None;
        self.font_name = None;
    }
}

/// Table parsing accumulator.
#[derive(Debug)]
pub(crate) struct TableState {
    pub(crate) active: bool,
    pub(crate) in_cell: bool,
    pub(crate) rows: Vec<ir::TableRow>,
    pub(crate) current_row_cells: Vec<ir::TableCell>,
    pub(crate) cell_blocks: Vec<ir::Block>,
    pub(crate) cell_inlines: Vec<ir::Inline>,
    pub(crate) cell_text: String,
    pub(crate) col_count: usize,
    pub(crate) current_colspan: u32,
    pub(crate) current_rowspan: u32,
}

impl Default for TableState {
    fn default() -> Self {
        Self {
            active: false,
            in_cell: false,
            rows: Vec::new(),
            current_row_cells: Vec::new(),
            cell_blocks: Vec::new(),
            cell_inlines: Vec::new(),
            cell_text: String::new(),
            col_count: 0,
            current_colspan: 1,
            current_rowspan: 1,
        }
    }
}

/// List parsing accumulator.
#[derive(Debug, Default)]
pub(crate) struct ListState {
    pub(crate) ordered: bool,
    pub(crate) active: bool,
    pub(crate) items: Vec<ir::ListItem>,
    pub(crate) in_item: bool,
    pub(crate) item_blocks: Vec<ir::Block>,
    pub(crate) item_inlines: Vec<ir::Inline>,
    pub(crate) item_text: String,
}

/// Footnote / endnote parsing accumulator.
#[derive(Debug, Default)]
pub(crate) struct FootnoteState {
    pub(crate) active: bool,
    pub(crate) id: String,
    pub(crate) blocks: Vec<ir::Block>,
    pub(crate) inlines: Vec<ir::Inline>,
    pub(crate) text: String,
}

/// Page layout parsed from `<hp:secPr>` and its children.
#[derive(Debug, Default)]
pub(crate) struct PageLayoutState {
    pub(crate) landscape: bool,
    pub(crate) width: Option<u32>,
    pub(crate) height: Option<u32>,
    pub(crate) margin_left: Option<u32>,
    pub(crate) margin_right: Option<u32>,
    pub(crate) margin_top: Option<u32>,
    pub(crate) margin_bottom: Option<u32>,
    pub(crate) has_sec_pr: bool,
}

impl PageLayoutState {
    pub(crate) fn take(&self) -> Option<ir::PageLayout> {
        if !self.has_sec_pr {
            return None;
        }
        Some(ir::PageLayout {
            width: self.width,
            height: self.height,
            landscape: self.landscape,
            margin_left: self.margin_left,
            margin_right: self.margin_right,
            margin_top: self.margin_top,
            margin_bottom: self.margin_bottom,
        })
    }

    /// Parse `<hp:pageSize width="…" height="…"/>` attributes.
    pub(crate) fn parse_page_size(&mut self, e: &quick_xml::events::BytesStart) {
        for attr in e.attributes().flatten() {
            let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
            let val = attr.unescape_value().unwrap_or_default();
            match key {
                "width" | "hp:width" => {
                    if let Ok(n) = val.as_ref().parse::<u32>() {
                        self.width = Some(n);
                    }
                }
                "height" | "hp:height" => {
                    if let Ok(n) = val.as_ref().parse::<u32>() {
                        self.height = Some(n);
                    }
                }
                _ => {}
            }
        }
    }

    /// Parse `<hp:margin left="…" right="…" top="…" bottom="…"/>` attributes.
    pub(crate) fn parse_margin(&mut self, e: &quick_xml::events::BytesStart) {
        for attr in e.attributes().flatten() {
            let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
            let val = attr.unescape_value().unwrap_or_default();
            match key {
                "left" | "hp:left" => {
                    if let Ok(n) = val.as_ref().parse::<u32>() {
                        self.margin_left = Some(n);
                    }
                }
                "right" | "hp:right" => {
                    if let Ok(n) = val.as_ref().parse::<u32>() {
                        self.margin_right = Some(n);
                    }
                }
                "top" | "hp:top" => {
                    if let Ok(n) = val.as_ref().parse::<u32>() {
                        self.margin_top = Some(n);
                    }
                }
                "bottom" | "hp:bottom" => {
                    if let Ok(n) = val.as_ref().parse::<u32>() {
                        self.margin_bottom = Some(n);
                    }
                }
                _ => {}
            }
        }
    }

    /// Parse `<hp:pagePr landscape="…"/>` attributes.
    pub(crate) fn parse_page_pr(&mut self, e: &quick_xml::events::BytesStart) {
        for attr in e.attributes().flatten() {
            let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
            let val = attr.unescape_value().unwrap_or_default();
            if key == "landscape" || key == "hp:landscape" {
                self.landscape = val.as_ref() == "true" || val.as_ref() == "1";
            }
        }
    }
}

// ── Main ParseContext ───────────────────────────────────────────────────

/// Mutable parsing state threaded through the XML event handlers.
///
/// A single `ParseContext` is created per section and passed to each
/// `handle_*` function.  It accumulates runs, inlines, cell blocks, list
/// items, and footnote blocks until the matching closing tag flushes them
/// into the section.
///
/// Fields are grouped into sub-structs by concern:
/// [`FormattingState`], [`TableState`], [`ListState`], [`FootnoteState`],
/// [`PageLayoutState`].
pub(crate) struct ParseContext {
    // ── Paragraph / run state ──────────────────────────────────────
    pub(crate) in_paragraph: bool,
    pub(crate) in_run: bool,
    /// True when the parser is inside a `<hp:t>` element.
    pub(crate) in_text: bool,
    pub(crate) current_text: String,
    pub(crate) current_inlines: Vec<ir::Inline>,
    /// Ordered list of face names parsed from `<hh:fontface>` entries in
    /// header.xml.
    pub(crate) face_names: Vec<String>,
    pub(crate) heading_level: Option<u8>,

    // ── Character formatting ───────────────────────────────────────
    pub(crate) fmt: FormattingState,

    // ── Table ──────────────────────────────────────────────────────
    pub(crate) table: TableState,

    // ── List ───────────────────────────────────────────────────────
    pub(crate) list: ListState,

    // ── Equation ───────────────────────────────────────────────────
    pub(crate) equation_text: String,
    pub(crate) in_equation: bool,

    // ── Footnote / endnote ─────────────────────────────────────────
    pub(crate) footnote: FootnoteState,

    // ── Hyperlink ──────────────────────────────────────────────────
    pub(crate) in_hyperlink: bool,
    pub(crate) hyperlink_url: Option<String>,

    // ── Ruby annotation ────────────────────────────────────────────
    pub(crate) in_ruby: bool,
    pub(crate) ruby_base_text: String,
    pub(crate) ruby_annotation_text: String,
    pub(crate) ruby_current_part: RubyPart,

    // ── OWPML flat-paragraph list detection ─────────────────────────
    /// `paraPrIDRef` value read from the current `<hp:p>` open tag.
    pub(crate) current_para_pr_id: Option<String>,
    /// `numPrIDRef` value read from the current `<hp:p>` open tag.
    pub(crate) current_num_pr_id: Option<String>,
    /// When `Some`, the next paragraph flushed should be a `CodeBlock`.
    pub(crate) pending_code_lang: Option<Option<String>>,

    // ── Page layout ────────────────────────────────────────────────
    pub(crate) page_layout: PageLayoutState,
}

/// Which child element of a `<hp:ruby>` block is currently being parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum RubyPart {
    #[default]
    None,
    Base,
    Annotation,
}

impl Default for ParseContext {
    fn default() -> Self {
        Self {
            in_paragraph: false,
            in_run: false,
            in_text: false,
            current_text: String::new(),
            current_inlines: Vec::new(),
            face_names: Vec::new(),
            heading_level: None,
            fmt: FormattingState::default(),
            table: TableState::default(),
            list: ListState::default(),
            equation_text: String::new(),
            in_equation: false,
            footnote: FootnoteState::default(),
            in_hyperlink: false,
            hyperlink_url: None,
            in_ruby: false,
            ruby_base_text: String::new(),
            ruby_annotation_text: String::new(),
            ruby_current_part: RubyPart::None,
            current_para_pr_id: None,
            current_num_pr_id: None,
            pending_code_lang: None,
            page_layout: PageLayoutState::default(),
        }
    }
}

impl ParseContext {
    /// Extract the accumulated `PageLayout` from the context.
    pub(crate) fn take_page_layout(&self) -> Option<ir::PageLayout> {
        self.page_layout.take()
    }

    /// Returns a mutable reference to the active text buffer.
    ///
    /// Priority: footnote > list_item > cell > default paragraph buffer.
    pub(crate) fn active_text_buf(&mut self) -> &mut String {
        if self.footnote.active {
            &mut self.footnote.text
        } else if self.list.in_item {
            &mut self.list.item_text
        } else if self.table.in_cell {
            &mut self.table.cell_text
        } else {
            &mut self.current_text
        }
    }

    /// Push an inline to the active inline buffer.
    pub(crate) fn push_inline(&mut self, inline: ir::Inline) {
        if self.footnote.active {
            self.footnote.inlines.push(inline);
        } else if self.list.in_item {
            self.list.item_inlines.push(inline);
        } else if self.table.in_cell {
            self.table.cell_inlines.push(inline);
        } else {
            self.current_inlines.push(inline);
        }
    }

    /// Push a block to the active block buffer.
    ///
    /// Returns `Some(block)` when at top level (caller routes through staging).
    pub(crate) fn push_block_scoped(&mut self, block: ir::Block) -> Option<ir::Block> {
        if self.footnote.active {
            self.footnote.blocks.push(block);
            None
        } else if self.list.in_item {
            self.list.item_blocks.push(block);
            None
        } else if self.table.in_cell {
            self.table.cell_blocks.push(block);
            None
        } else {
            Some(block)
        }
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
            "bold" | "hp:bold" => ctx.fmt.bold = val.as_ref() == "true" || val.as_ref() == "1",
            "italic" | "hp:italic" => {
                ctx.fmt.italic = val.as_ref() == "true" || val.as_ref() == "1"
            }
            "underline" | "hp:underline" => {
                ctx.fmt.underline = !val.is_empty() && val.as_ref() != "none" && val.as_ref() != "0"
            }
            "strikeout" | "hp:strikeout" => {
                ctx.fmt.strike = !val.is_empty() && val.as_ref() != "none" && val.as_ref() != "0"
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
            ir::Inline::with_formatting(
                t,
                fmt.bold,
                fmt.italic,
                fmt.underline,
                fmt.strike,
                fmt.superscript,
                fmt.subscript,
                fmt.color.clone(),
            )
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
    ir::Inline::with_formatting(
        text,
        fmt.bold,
        fmt.italic,
        fmt.underline,
        fmt.strike,
        fmt.superscript,
        fmt.subscript,
        fmt.color.clone(),
    )
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

/// Flush whichever scope is currently active (footnote → list-item → cell →
/// top-level paragraph) so that any buffered inline run becomes a finished
/// block before the caller stages a sibling block.
///
/// Used immediately before pushing a block-level event (e.g. a page break
/// `<hp:ctrl id="newPage"/>`) that appears between text runs inside a single
/// `<hp:p>`.  Without this, the buffered text would be merged across the
/// block boundary and the new block would emit out of order.
///
/// At top level the accumulated `current_text`/`current_inlines` is wrapped
/// in a `Paragraph` and returned via `Option<StagedBlock>` for the caller to
/// append to its staging vector — mirroring the contract used by
/// [`flush_paragraph_staged`].  In every nested scope the flush stays
/// in-context and `None` is returned.
#[must_use = "top-level paragraph must be appended to the section staging vector"]
pub(crate) fn flush_active_paragraph_scope(ctx: &mut ParseContext) -> Option<StagedBlock> {
    if ctx.footnote.active {
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
