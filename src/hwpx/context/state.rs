use crate::ir;

// ── Sub-struct definitions ──────────────────────────────────────────────

/// Character-level formatting state for the current XML run.
// Bool fields directly map to HWPX formatting state bits defined in the OWPML spec.
#[allow(clippy::struct_excessive_bools)]
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

impl From<&FormattingState> for crate::ir::InlineFormat {
    fn from(s: &FormattingState) -> Self {
        Self {
            bold: s.bold,
            italic: s.italic,
            underline: s.underline,
            strikethrough: s.strike,
            superscript: s.superscript,
            subscript: s.subscript,
            color: s.color.clone(),
        }
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
    pub(crate) inner_margin: Option<ir::TableInnerMargin>,
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
            inner_margin: None,
        }
    }
}

impl TableState {
    /// Parse `<hp:inMargin>` attributes into `self.inner_margin`.
    pub(crate) fn parse_in_margin(&mut self, e: &quick_xml::events::BytesStart) {
        let mut m = ir::TableInnerMargin { left: 0, right: 0, top: 0, bottom: 0 };
        for attr in e.attributes().flatten() {
            let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
            let val: u32 = attr.unescape_value().unwrap_or_default().parse().unwrap_or(0);
            match key {
                "left"   | "hp:left"   => m.left   = val,
                "right"  | "hp:right"  => m.right  = val,
                "top"    | "hp:top"    => m.top     = val,
                "bottom" | "hp:bottom" => m.bottom  = val,
                _ => {}
            }
        }
        self.inner_margin = Some(m);
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

/// Header/footer parsing accumulator.
///
/// OWPML documents may include a `<hp:headerFooter>` element as a sibling of
/// the section paragraphs.  It contains `<hp:header>` and `<hp:footer>` sub-
/// elements, each of which holds ordinary paragraph content.
#[derive(Debug, Default)]
pub(crate) struct HeaderFooterState {
    /// `true` while the parser is inside a `<hp:headerFooter>` element.
    pub(crate) active: bool,
    /// `true` while the parser is inside the `<hp:header>` child.
    pub(crate) in_header: bool,
    /// `true` while the parser is inside the `<hp:footer>` child.
    pub(crate) in_footer: bool,
    /// Accumulated blocks for the header region.
    pub(crate) header_blocks: Vec<ir::Block>,
    /// Accumulated blocks for the footer region.
    pub(crate) footer_blocks: Vec<ir::Block>,
    /// Temporary text buffer used while parsing header/footer paragraphs.
    pub(crate) text: String,
    /// Temporary inline buffer used while parsing header/footer paragraphs.
    pub(crate) inlines: Vec<ir::Inline>,
    /// The `type` attribute of the `<hp:headerFooter>` element (e.g. "both", "even", "odd").
    pub(crate) hf_type: Option<ir::HeaderFooterType>,
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
