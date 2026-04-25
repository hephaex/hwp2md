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
) {
    if !text.is_empty() {
        let t = std::mem::take(text);
        inlines.push(ir::Inline::with_formatting(
            t,
            bold,
            italic,
            underline,
            strike,
            superscript,
            subscript,
            color.clone(),
        ));
    }
    if !inlines.is_empty() {
        let i = std::mem::take(inlines);
        blocks.push(ir::Block::Paragraph { inlines: i });
    }
}

pub(crate) fn flush_paragraph(ctx: &mut ParseContext, section: &mut ir::Section) {
    // Flush any trailing text run into the inline buffer first.
    if !ctx.current_text.is_empty() {
        let t = std::mem::take(&mut ctx.current_text);
        ctx.current_inlines.push(ir::Inline::with_formatting(
            t,
            ctx.current_bold,
            ctx.current_italic,
            ctx.current_underline,
            ctx.current_strike,
            ctx.current_superscript,
            ctx.current_subscript,
            ctx.current_color.clone(),
        ));
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
    );
}
