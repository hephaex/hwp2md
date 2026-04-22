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
}

impl Default for ParseContext {
    fn default() -> Self {
        Self {
            in_paragraph: false,
            in_run: false,
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
        }
    }
}

impl ParseContext {
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

/// Parse `bold`, `italic`, `underline`, and `strikeout` attributes from a
/// `<charPr>` or `<hp:charPr>` element and write them onto `ctx`.
///
/// Called from both `handle_start_element` (non-self-closing variant) and
/// `handle_empty_element` (self-closing variant) so the two paths are
/// guaranteed to behave identically.
pub(crate) fn apply_charpr_attrs(e: &quick_xml::events::BytesStart, ctx: &mut ParseContext) {
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
            _ => {}
        }
    }
}

pub(crate) fn flush_paragraph(ctx: &mut ParseContext, section: &mut ir::Section) {
    if !ctx.current_text.is_empty() {
        let text = std::mem::take(&mut ctx.current_text);
        ctx.current_inlines.push(ir::Inline {
            text,
            bold: ctx.current_bold,
            italic: ctx.current_italic,
            underline: ctx.current_underline,
            strikethrough: ctx.current_strike,
            superscript: ctx.current_superscript,
            subscript: ctx.current_subscript,
            ..ir::Inline::default()
        });
    }

    if ctx.current_inlines.is_empty() {
        return;
    }

    let inlines = std::mem::take(&mut ctx.current_inlines);
    let block = if let Some(level) = ctx.heading_level {
        ir::Block::Heading { level, inlines }
    } else {
        ir::Block::Paragraph { inlines }
    };
    section.blocks.push(block);
}

pub(crate) fn flush_cell_paragraph(ctx: &mut ParseContext) {
    if !ctx.cell_text.is_empty() {
        let text = std::mem::take(&mut ctx.cell_text);
        ctx.cell_inlines.push(ir::Inline {
            text,
            bold: ctx.current_bold,
            italic: ctx.current_italic,
            underline: ctx.current_underline,
            strikethrough: ctx.current_strike,
            superscript: ctx.current_superscript,
            subscript: ctx.current_subscript,
            ..ir::Inline::default()
        });
    }

    if !ctx.cell_inlines.is_empty() {
        let inlines = std::mem::take(&mut ctx.cell_inlines);
        ctx.cell_blocks.push(ir::Block::Paragraph { inlines });
    }
}

pub(crate) fn flush_list_item_paragraph(ctx: &mut ParseContext) {
    if !ctx.list_item_text.is_empty() {
        let text = std::mem::take(&mut ctx.list_item_text);
        ctx.list_item_inlines.push(ir::Inline {
            text,
            bold: ctx.current_bold,
            italic: ctx.current_italic,
            underline: ctx.current_underline,
            strikethrough: ctx.current_strike,
            superscript: ctx.current_superscript,
            subscript: ctx.current_subscript,
            ..ir::Inline::default()
        });
    }

    if !ctx.list_item_inlines.is_empty() {
        let inlines = std::mem::take(&mut ctx.list_item_inlines);
        ctx.list_item_blocks.push(ir::Block::Paragraph { inlines });
    }
}

/// Flush any pending run text and inline list from within a footnote/endnote
/// paragraph into `footnote_blocks`.  Mirrors the logic of `flush_cell_paragraph`.
pub(crate) fn flush_footnote_paragraph(ctx: &mut ParseContext) {
    if !ctx.footnote_text.is_empty() {
        let text = std::mem::take(&mut ctx.footnote_text);
        ctx.footnote_inlines.push(ir::Inline {
            text,
            bold: ctx.current_bold,
            italic: ctx.current_italic,
            underline: ctx.current_underline,
            strikethrough: ctx.current_strike,
            superscript: ctx.current_superscript,
            subscript: ctx.current_subscript,
            ..ir::Inline::default()
        });
    }

    if !ctx.footnote_inlines.is_empty() {
        let inlines = std::mem::take(&mut ctx.footnote_inlines);
        ctx.footnote_blocks.push(ir::Block::Paragraph { inlines });
    }
}
