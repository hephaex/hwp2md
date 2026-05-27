mod flush;
mod state;

#[cfg(test)]
pub(crate) use flush::flush_paragraph;
pub(crate) use flush::{
    apply_charpr_attrs, flush_active_paragraph_scope, flush_cell_paragraph,
    flush_footnote_paragraph, flush_list_item_paragraph, flush_nested_scope,
    flush_paragraph_staged, group_list_paragraphs, CodeLangHint, StagedBlock,
};
pub(crate) use state::{
    FootnoteState, FormattingState, HeaderFooterState, ListState, PageLayoutState, TableState,
};

use crate::ir;

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
// Bool fields track independent parse state flags (paragraph/run/text context, hyperlink, ruby, equation).
#[allow(clippy::struct_excessive_bools)]
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
    /// Hint for the next paragraph flush: whether it should become a `CodeBlock`
    /// and which language to annotate.  Defaults to `Plain` (normal paragraph).
    pub(crate) pending_code_lang: CodeLangHint,

    // ── Per-paragraph font-height tracker (tier-3 heading) ──────────
    /// Maximum `charPr height` value (1/100 pt) seen in the current paragraph.
    pub(crate) para_max_font_height: u32,
    /// Whether the run that produced `para_max_font_height` was also bold.
    pub(crate) para_max_font_height_bold: bool,

    // ── Page layout ────────────────────────────────────────────────
    pub(crate) page_layout: PageLayoutState,

    // ── Header / footer ────────────────────────────────────────────
    pub(crate) header_footer: HeaderFooterState,
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
            pending_code_lang: CodeLangHint::Plain,
            para_max_font_height: 0,
            para_max_font_height_bold: false,
            page_layout: PageLayoutState::default(),
            header_footer: HeaderFooterState::default(),
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
    /// Priority: header/footer > footnote > `list_item` > cell > default paragraph buffer.
    pub(crate) fn active_text_buf(&mut self) -> &mut String {
        if self.header_footer.active
            && (self.header_footer.in_header || self.header_footer.in_footer)
        {
            &mut self.header_footer.text
        } else if self.footnote.active {
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
        if self.header_footer.active
            && (self.header_footer.in_header || self.header_footer.in_footer)
        {
            self.header_footer.inlines.push(inline);
        } else if self.footnote.active {
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
        if self.header_footer.in_header {
            self.header_footer.header_blocks.push(block);
            None
        } else if self.header_footer.in_footer {
            self.header_footer.footer_blocks.push(block);
            None
        } else if self.footnote.active {
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
