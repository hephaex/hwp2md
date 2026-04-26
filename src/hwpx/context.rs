use crate::ir;

/// Mutable parsing state threaded through the XML event handlers.
///
/// A single `ParseContext` is created per section and passed to each
/// `handle_*` function.  It accumulates runs, inlines, cell blocks, list
/// items, and footnote blocks until the matching closing tag flushes them
/// into the section.
pub(crate) struct ParseContext {
    pub(crate) in_paragraph: bool,
    pub(crate) in_run: bool,
    /// True when the parser is inside a `<hp:t>` element.  Text is only
    /// accumulated into the active buffer when this flag is set, preventing
    /// XML formatting whitespace from bleeding into inline text content.
    pub(crate) in_text: bool,
    pub(crate) in_table: bool,
    pub(crate) in_cell: bool,
    pub(crate) current_text: String,
    pub(crate) current_inlines: Vec<ir::Inline>,
    pub(crate) current_bold: bool,
    pub(crate) current_italic: bool,
    pub(crate) current_underline: bool,
    pub(crate) current_strike: bool,
    pub(crate) current_superscript: bool,
    pub(crate) current_subscript: bool,
    /// CSS hex color string parsed from the `color` / `hp:color` attribute of
    /// `<charPr>`.  `None` means default text color (not rendered).
    pub(crate) current_color: Option<String>,
    /// Font name resolved from the DocInfo `<hh:fontface>` table via a
    /// `faceNameIDRef` index carried on a `<charPr>` element.  `None` means
    /// the run uses the document default font.
    pub(crate) current_font_name: Option<String>,
    /// Ordered list of face names parsed from `<hh:fontface>` entries in
    /// header.xml.  The position in the vec is the index used by
    /// `faceNameIDRef` in `<charPr>` elements inside section XML.
    pub(crate) face_names: Vec<String>,
    pub(crate) heading_level: Option<u8>,
    pub(crate) table_rows: Vec<ir::TableRow>,
    pub(crate) current_row_cells: Vec<ir::TableCell>,
    pub(crate) cell_blocks: Vec<ir::Block>,
    pub(crate) cell_inlines: Vec<ir::Inline>,
    pub(crate) cell_text: String,
    pub(crate) col_count: usize,
    // colspan/rowspan for the cell currently being parsed
    pub(crate) current_colspan: u32,
    pub(crate) current_rowspan: u32,
    pub(crate) list_ordered: bool,
    pub(crate) in_list: bool,
    pub(crate) list_items: Vec<ir::ListItem>,
    pub(crate) in_list_item: bool,
    pub(crate) list_item_blocks: Vec<ir::Block>,
    pub(crate) list_item_inlines: Vec<ir::Inline>,
    pub(crate) list_item_text: String,
    pub(crate) equation_text: String,
    pub(crate) in_equation: bool,
    // footnote / endnote accumulation
    pub(crate) in_footnote: bool,
    pub(crate) footnote_id: String,
    pub(crate) footnote_blocks: Vec<ir::Block>,
    pub(crate) footnote_inlines: Vec<ir::Inline>,
    pub(crate) footnote_text: String,
    // hyperlink (fieldBegin/fieldEnd) accumulation
    pub(crate) in_hyperlink: bool,
    pub(crate) hyperlink_url: Option<String>,
    // ruby annotation accumulation
    pub(crate) in_ruby: bool,
    pub(crate) ruby_base_text: String,
    pub(crate) ruby_annotation_text: String,
    /// Which sub-element of `<hp:ruby>` is currently active.
    pub(crate) ruby_current_part: RubyPart,
    // ── OWPML flat-paragraph list detection ─────────────────────────────
    /// `paraPrIDRef` value read from the current `<hp:p>` open tag.
    ///
    /// `None` means no `paraPrIDRef` attribute was present (normal paragraph).
    /// `Some(id)` is the raw string value — recognised values are `"2"` (list
    /// depth-0) and `"3"` (list depth-1+) as emitted by our own writer and as
    /// defined in header.xml's `<hh:paraProperties>` table.
    pub(crate) current_para_pr_id: Option<String>,
    /// `numPrIDRef` value read from the current `<hp:p>` open tag.
    ///
    /// `Some("1")` means the paragraph is an ordered list item (DIGIT
    /// numbering, the sole registered numbering definition).  `None` means it
    /// is either a non-list paragraph or an unordered list item (bullet lists
    /// omit `numPrIDRef` per the OWPML schema).
    pub(crate) current_num_pr_id: Option<String>,
    /// When `Some`, the next paragraph flushed by `flush_paragraph` should be
    /// emitted as a `CodeBlock` rather than a plain `Paragraph`.
    ///
    /// The inner `Option<String>` carries the language hint parsed from the
    /// `<!-- hwp2md:lang:LANG -->` comment preceding the paragraph:
    /// - `Some(Some("python"))` → code block with language `"python"`
    /// - `Some(None)`           → code block with no language hint
    ///
    /// Reset to `None` after each `flush_paragraph` call regardless of whether
    /// any inlines were present.
    pub(crate) pending_code_lang: Option<Option<String>>,
    // ── Page layout (parsed from <hp:secPr>) ────────────────────────────
    /// Accumulated page layout parsed from `<hp:secPr><hp:pagePr>` and its
    /// `<hp:pageSize>` / `<hp:margin>` children.  Populated during section
    /// XML parsing and transferred to `ir::Section::page_layout` after the
    /// event loop completes.
    pub(crate) page_layout_landscape: bool,
    pub(crate) page_layout_width: Option<u32>,
    pub(crate) page_layout_height: Option<u32>,
    pub(crate) page_layout_margin_left: Option<u32>,
    pub(crate) page_layout_margin_right: Option<u32>,
    pub(crate) page_layout_margin_top: Option<u32>,
    pub(crate) page_layout_margin_bottom: Option<u32>,
    /// True when at least one `<hp:secPr>` element was encountered.
    pub(crate) has_sec_pr: bool,
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
            in_table: false,
            in_cell: false,
            current_text: String::new(),
            current_inlines: Vec::new(),
            current_bold: false,
            current_italic: false,
            current_underline: false,
            current_strike: false,
            current_superscript: false,
            current_subscript: false,
            current_color: None,
            current_font_name: None,
            face_names: Vec::new(),
            heading_level: None,
            table_rows: Vec::new(),
            current_row_cells: Vec::new(),
            cell_blocks: Vec::new(),
            cell_inlines: Vec::new(),
            cell_text: String::new(),
            col_count: 0,
            // A cell with no span attributes spans exactly 1 column and 1 row.
            current_colspan: 1,
            current_rowspan: 1,
            list_ordered: false,
            in_list: false,
            list_items: Vec::new(),
            in_list_item: false,
            list_item_blocks: Vec::new(),
            list_item_inlines: Vec::new(),
            list_item_text: String::new(),
            equation_text: String::new(),
            in_equation: false,
            in_footnote: false,
            footnote_id: String::new(),
            footnote_blocks: Vec::new(),
            footnote_inlines: Vec::new(),
            footnote_text: String::new(),
            in_hyperlink: false,
            hyperlink_url: None,
            in_ruby: false,
            ruby_base_text: String::new(),
            ruby_annotation_text: String::new(),
            ruby_current_part: RubyPart::None,
            current_para_pr_id: None,
            current_num_pr_id: None,
            pending_code_lang: None,
            page_layout_landscape: false,
            page_layout_width: None,
            page_layout_height: None,
            page_layout_margin_left: None,
            page_layout_margin_right: None,
            page_layout_margin_top: None,
            page_layout_margin_bottom: None,
            has_sec_pr: false,
        }
    }
}

impl ParseContext {
    /// Extract the accumulated `PageLayout` from the context.
    ///
    /// Returns `Some(PageLayout)` when at least one `<hp:secPr>` element was
    /// encountered during parsing, `None` otherwise (e.g. very minimal HWPX
    /// files that omit section properties).
    pub(crate) fn take_page_layout(&self) -> Option<ir::PageLayout> {
        if !self.has_sec_pr {
            return None;
        }
        Some(ir::PageLayout {
            width: self.page_layout_width,
            height: self.page_layout_height,
            landscape: self.page_layout_landscape,
            margin_left: self.page_layout_margin_left,
            margin_right: self.page_layout_margin_right,
            margin_top: self.page_layout_margin_top,
            margin_bottom: self.page_layout_margin_bottom,
        })
    }

    /// Returns a mutable reference to the active text buffer.
    ///
    /// Priority: footnote > list_item > cell > default paragraph buffer.
    pub(crate) fn active_text_buf(&mut self) -> &mut String {
        if self.in_footnote {
            &mut self.footnote_text
        } else if self.in_list_item {
            &mut self.list_item_text
        } else if self.in_cell {
            &mut self.cell_text
        } else {
            &mut self.current_text
        }
    }

    /// Push an inline to the active inline buffer.
    pub(crate) fn push_inline(&mut self, inline: ir::Inline) {
        if self.in_footnote {
            self.footnote_inlines.push(inline);
        } else if self.in_list_item {
            self.list_item_inlines.push(inline);
        } else if self.in_cell {
            self.cell_inlines.push(inline);
        } else {
            self.current_inlines.push(inline);
        }
    }

    /// Push a block to the active block buffer.
    ///
    /// Note: the default (non-cell, non-list-item, non-footnote) target is the
    /// section's block list, which is passed separately to keep the borrow
    /// checker happy.  Call `section.blocks.push(block)` directly in that case.
    pub(crate) fn push_block_scoped(&mut self, block: ir::Block) -> Option<ir::Block> {
        if self.in_footnote {
            self.footnote_blocks.push(block);
            None
        } else if self.in_list_item {
            self.list_item_blocks.push(block);
            None
        } else if self.in_cell {
            self.cell_blocks.push(block);
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
///
/// Font resolution: HWPX section XML carries a `faceNameIDRef` (or
/// `hangulIDRef` in some encodings) attribute on `<charPr>` that is a
/// zero-based index into the `<hh:fontface>` list built from header.xml.
/// When `ctx.face_names` is populated, the index is resolved to a font name
/// and stored in `ctx.current_font_name`.  An out-of-range index is silently
/// ignored.
pub(crate) fn apply_charpr_attrs(e: &quick_xml::events::BytesStart, ctx: &mut ParseContext) {
    // Collect faceNameIDRef / hangulIDRef separately so we can resolve after
    // scanning all attributes (avoids a second pass).
    let mut face_id: Option<usize> = None;

    for attr in e.attributes().flatten() {
        let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
        let val = attr.unescape_value().unwrap_or_default();
        match key {
            "bold" | "hp:bold" => ctx.current_bold = val.as_ref() == "true" || val.as_ref() == "1",
            "italic" | "hp:italic" => {
                ctx.current_italic = val.as_ref() == "true" || val.as_ref() == "1"
            }
            "underline" | "hp:underline" => {
                ctx.current_underline =
                    !val.is_empty() && val.as_ref() != "none" && val.as_ref() != "0"
            }
            "strikeout" | "hp:strikeout" => {
                ctx.current_strike =
                    !val.is_empty() && val.as_ref() != "none" && val.as_ref() != "0"
            }
            "supscript" | "hp:supscript" => {
                ctx.current_superscript = val.as_ref() == "superscript";
                ctx.current_subscript = val.as_ref() == "subscript";
            }
            "color" | "hp:color" => {
                // HWPX color attribute is a hex string, potentially with a
                // leading "#" (e.g. "#FF0000") or without (e.g. "FF0000").
                // Black text (#000000 or "000000") is treated as default and
                // not propagated to avoid wrapping every run in a <span>.
                let raw = val.as_ref().trim_start_matches('#');
                if raw.is_empty() || raw.eq_ignore_ascii_case("000000") {
                    ctx.current_color = None;
                } else {
                    // Normalise to upper-case hex with a leading '#'.
                    ctx.current_color = Some(format!("#{}", raw.to_ascii_uppercase()));
                }
            }
            // HWPX section XML uses `faceNameIDRef` (HANGUL slot index into the
            // document-level fontface list) or the alias `hangulIDRef`.
            "faceNameIDRef" | "hp:faceNameIDRef" | "hangulIDRef" | "hp:hangulIDRef" => {
                if let Ok(idx) = val.as_ref().parse::<usize>() {
                    face_id = Some(idx);
                }
            }
            _ => {}
        }
    }

    // Resolve font index to a name using the pre-populated face_names table.
    if let Some(idx) = face_id {
        ctx.current_font_name = ctx.face_names.get(idx).cloned();
    }
}

/// Drain accumulated `text` + `inlines` into `blocks` as a `Paragraph`.
///
/// If `text` is non-empty an `ir::Inline` is built from `text` and the
/// supplied style fields and appended to `inlines` first.  If `inlines` is
/// still empty after that step nothing is pushed to `blocks`.
///
/// Style fields are passed individually (rather than `&ParseContext`) so that
/// callers can split the borrow between the mutable text/inline buffers and
/// the style fields without a borrow-checker conflict.
#[allow(clippy::too_many_arguments)]
fn flush_inlines_to_blocks(
    text: &mut String,
    inlines: &mut Vec<ir::Inline>,
    blocks: &mut Vec<ir::Block>,
    bold: bool,
    italic: bool,
    underline: bool,
    strike: bool,
    superscript: bool,
    subscript: bool,
    color: &Option<String>,
    font_name: &Option<String>,
) {
    if !text.is_empty() {
        let t = std::mem::take(text);
        inlines.push(
            ir::Inline::with_formatting(
                t,
                bold,
                italic,
                underline,
                strike,
                superscript,
                subscript,
                color.clone(),
            )
            .with_font_name(font_name.clone()),
        );
    }
    if !inlines.is_empty() {
        let i = std::mem::take(inlines);
        blocks.push(ir::Block::Paragraph { inlines: i });
    }
}

/// Flush any pending paragraph inlines to `section.blocks`.
///
/// When `ctx.pending_code_lang` is `Some`, the accumulated inline text is
/// treated as raw code source and emitted as a `Block::CodeBlock` with the
/// indicated language hint.  The flag is always reset to `None` after the
/// call regardless of whether any inlines were present.
///
/// Otherwise the inlines are emitted as a `Block::Heading` (when
/// `ctx.heading_level` is set) or a plain `Block::Paragraph`.
#[cfg(test)]
pub(crate) fn flush_paragraph(ctx: &mut ParseContext, section: &mut ir::Section) {
    // Flush any trailing text run into the inline buffer first.
    if !ctx.current_text.is_empty() {
        let t = std::mem::take(&mut ctx.current_text);
        ctx.current_inlines.push(
            ir::Inline::with_formatting(
                t,
                ctx.current_bold,
                ctx.current_italic,
                ctx.current_underline,
                ctx.current_strike,
                ctx.current_superscript,
                ctx.current_subscript,
                ctx.current_color.clone(),
            )
            .with_font_name(ctx.current_font_name.clone()),
        );
    }

    // Always consume the pending_code_lang hint so it does not bleed into the
    // next paragraph when the current one turns out to be empty.
    let code_lang = ctx.pending_code_lang.take();

    if ctx.current_inlines.is_empty() {
        return;
    }

    let inlines = std::mem::take(&mut ctx.current_inlines);

    // When a language-hint comment preceded this paragraph, reconstruct the
    // original CodeBlock.  The code text is recovered by concatenating all
    // inline texts (the writer emits code as a single <hp:t> run with no
    // formatting, so there will normally be exactly one inline).
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
/// parsing.  Instead of pushing directly to `section.blocks` it returns a
/// [`StagedBlock`] that the caller records in a staging vector.  The staging
/// vector is later collapsed into proper `Block::List` structures by
/// [`group_list_paragraphs`].
///
/// When the paragraph carries an OWPML `paraPrIDRef` value that identifies it
/// as a list item, the returned variant is `StagedBlock::ListPara`; otherwise
/// it is `StagedBlock::Plain`.
///
/// `paraPrIDRef` values recognised as list indicators (matching the constants
/// in `writer_header.rs`):
/// - `"2"` → depth 0  (`PARA_PR_LIST_D0`)
/// - `"3"` → depth 1+ (`PARA_PR_LIST_D1`)
///
/// `numPrIDRef` values recognised:
/// - `"1"` → ordered list (DIGIT numbering definition)
/// - absent / anything else → unordered (bullet) list
pub(crate) fn flush_paragraph_staged(ctx: &mut ParseContext) -> Option<StagedBlock> {
    // Flush any trailing text run into the inline buffer.
    if !ctx.current_text.is_empty() {
        let t = std::mem::take(&mut ctx.current_text);
        ctx.current_inlines.push(
            ir::Inline::with_formatting(
                t,
                ctx.current_bold,
                ctx.current_italic,
                ctx.current_underline,
                ctx.current_strike,
                ctx.current_superscript,
                ctx.current_subscript,
                ctx.current_color.clone(),
            )
            .with_font_name(ctx.current_font_name.clone()),
        );
    }

    // Always drain these context fields so they do not bleed into the next
    // paragraph when the current one turns out to be empty.
    let para_pr_id = ctx.current_para_pr_id.take();
    let num_pr_id = ctx.current_num_pr_id.take();
    let code_lang = ctx.pending_code_lang.take();

    if ctx.current_inlines.is_empty() {
        return None;
    }

    let inlines = std::mem::take(&mut ctx.current_inlines);

    // Code block paragraphs (preceded by a <!-- hwp2md:lang:LANG --> comment)
    // are never list items — emit them as plain staged blocks.
    if let Some(language) = code_lang {
        let code = inlines.into_iter().map(|i| i.text).collect::<String>();
        return Some(StagedBlock::Plain(ir::Block::CodeBlock { language, code }));
    }

    let block = if let Some(level) = ctx.heading_level {
        ir::Block::Heading { level, inlines }
    } else {
        ir::Block::Paragraph { inlines }
    };

    // Detect list-paragraph by paraPrIDRef.
    // "2" = PARA_PR_LIST_D0 (depth 0); "3" = PARA_PR_LIST_D1 (depth ≥ 1).
    // Headings are never list items even if they somehow carry paraPrIDRef.
    let is_heading = ctx.heading_level.is_some();
    let list_depth: Option<u32> = if is_heading {
        None
    } else {
        match para_pr_id.as_deref() {
            Some("2") => Some(0),
            Some("3") => Some(1),
            // Custom paraPr IDs >= 4: treat as depth 1 (deepest we track).
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

/// An intermediate block produced during OWPML section parsing before the
/// flat list-paragraph sequence is collapsed into `Block::List` structures.
///
/// `Plain` wraps any block that is not part of an OWPML list.  `ListPara`
/// wraps a paragraph that carries `paraPrIDRef` / `numPrIDRef` attributes
/// identifying it as a list item at a given depth.
#[derive(Debug)]
pub(crate) enum StagedBlock {
    Plain(ir::Block),
    ListPara {
        /// Nesting depth: 0 = top-level, 1 = first-level nested, …
        depth: u32,
        /// `true` when the paragraph carries `numPrIDRef="1"` (ordered list).
        ordered: bool,
        /// The paragraph block itself.
        block: ir::Block,
    },
}

/// Collapse a flat sequence of [`StagedBlock`]s into a proper `Vec<ir::Block>`
/// where consecutive `ListPara` entries at the same or increasing depth are
/// grouped into `Block::List` with `ListItem.children` for deeper levels.
///
/// # Algorithm
///
/// The algorithm walks the staged blocks left-to-right.  A run of consecutive
/// `ListPara` entries is collected into a pending list group.  When a `Plain`
/// block (or end of input) interrupts the run, the pending group is folded
/// into a `Block::List` and appended to the output.
///
/// Within a pending group, depth transitions are handled as follows:
/// - Same depth or decreasing depth: new top-level item in the same list.
/// - Increasing depth: nested children attached to the most recent item.
///
/// The ordered/unordered flag for the top-level list is taken from the **first**
/// item in the group.  If later items disagree (mixed ordered/unordered at the
/// same level) they are folded into the same list with the flag of the first
/// item — this matches OWPML reader behaviour where the list type is determined
/// by the first paragraph's `numPrIDRef`.
pub(crate) fn group_list_paragraphs(staged: Vec<StagedBlock>) -> Vec<ir::Block> {
    let mut out: Vec<ir::Block> = Vec::with_capacity(staged.len());

    // Pending run of consecutive ListPara entries.
    // Each entry is (depth, ordered, ir::Block).
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

/// Convert a flat list of `(depth, ordered, block)` tuples into a `Block::List`
/// with properly nested `ListItem` children.
///
/// The algorithm builds the top-level list using the `ordered` flag of the
/// first item.  Items at depth 0 become direct items; items at depth 1+
/// become children of the most recent depth-0 item.  Deeper nesting (depth 2+)
/// is flattened to depth-1 children because the OWPML schema only defines two
/// paraPr levels (`PARA_PR_LIST_D0` and `PARA_PR_LIST_D1`).
fn build_list(entries: Vec<(u32, bool, ir::Block)>) -> ir::Block {
    if entries.is_empty() {
        // Should not happen, but produce a safe empty list.
        return ir::Block::List {
            ordered: false,
            start: 1,
            items: vec![],
        };
    }

    let top_ordered = entries[0].1;

    // We build top-level items.  Each item may have children (depth >= 1).
    // Stack structure: we track items in progress.
    let mut items: Vec<ir::ListItem> = Vec::new();

    for (depth, _ordered, block) in entries {
        if depth == 0 {
            items.push(ir::ListItem {
                blocks: vec![block],
                children: vec![],
            });
        } else {
            // Attach as a child of the last top-level item.
            if items.is_empty() {
                // No parent yet — promote to top level (defensive).
                items.push(ir::ListItem {
                    blocks: vec![block],
                    children: vec![],
                });
            } else {
                let parent = items.last_mut().expect("items is non-empty");
                parent.children.push(ir::ListItem {
                    blocks: vec![block],
                    children: vec![],
                });
            }
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
        &mut ctx.cell_text,
        &mut ctx.cell_inlines,
        &mut ctx.cell_blocks,
        ctx.current_bold,
        ctx.current_italic,
        ctx.current_underline,
        ctx.current_strike,
        ctx.current_superscript,
        ctx.current_subscript,
        &ctx.current_color,
        &ctx.current_font_name,
    );
}

pub(crate) fn flush_list_item_paragraph(ctx: &mut ParseContext) {
    flush_inlines_to_blocks(
        &mut ctx.list_item_text,
        &mut ctx.list_item_inlines,
        &mut ctx.list_item_blocks,
        ctx.current_bold,
        ctx.current_italic,
        ctx.current_underline,
        ctx.current_strike,
        ctx.current_superscript,
        ctx.current_subscript,
        &ctx.current_color,
        &ctx.current_font_name,
    );
}

/// Flush any pending run text and inline list from within a footnote/endnote
/// paragraph into `footnote_blocks`.  Mirrors the logic of `flush_cell_paragraph`.
pub(crate) fn flush_footnote_paragraph(ctx: &mut ParseContext) {
    flush_inlines_to_blocks(
        &mut ctx.footnote_text,
        &mut ctx.footnote_inlines,
        &mut ctx.footnote_blocks,
        ctx.current_bold,
        ctx.current_italic,
        ctx.current_underline,
        ctx.current_strike,
        ctx.current_superscript,
        ctx.current_subscript,
        &ctx.current_color,
        &ctx.current_font_name,
    );
}
