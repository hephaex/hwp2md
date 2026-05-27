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
    /// Font height in 1/100 pt units, parsed from `<hp:charPr height="…"/>`.
    pub(crate) font_height: Option<u32>,
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
        self.font_height = None;
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
    ///
    /// Axes absent from the element default to [`ir::DEFAULT_TABLE_INNER_MARGIN`].
    /// Axes with non-numeric values are silently skipped (default preserved).
    pub(crate) fn parse_in_margin(&mut self, e: &quick_xml::events::BytesStart) {
        let d = ir::DEFAULT_TABLE_INNER_MARGIN;
        let mut m = ir::TableInnerMargin { left: d, right: d, top: d, bottom: d };
        for attr in e.attributes().flatten() {
            let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
            let Ok(val) = attr.unescape_value().unwrap_or_default().parse::<u32>() else {
                continue;
            };
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
                // Strict boolean: only "true"/"1" or "false"/"0" mutate state.
                // Unknown values preserve prior state, consistent with
                // parse_page_size / parse_margin skip-on-parse-failure semantics.
                match val.as_ref() {
                    "true" | "1"  => self.landscape = true,
                    "false" | "0" => self.landscape = false,
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Build a BytesStart for an empty XML element without leaking memory.
    // `into_owned()` copies the byte data into a new allocation so the
    // local `xml` String can drop at end of function — no Box::leak needed.
    fn make_empty(tag: &str, attrs: &str) -> quick_xml::events::BytesStart<'static> {
        let xml = if attrs.is_empty() {
            format!("<{tag}/>")
        } else {
            format!("<{tag} {attrs}/>")
        };
        let mut reader = quick_xml::Reader::from_str(&xml);
        match reader.read_event().unwrap() {
            quick_xml::events::Event::Empty(e) => e.into_owned(),
            ev => panic!("expected Event::Empty, got {ev:?}"),
        }
    }

    fn make_in_margin(attrs: &str) -> quick_xml::events::BytesStart<'static> {
        make_empty("hp:inMargin", attrs)
    }

    #[test]
    fn parse_in_margin_all_axes_explicit() {
        let mut state = TableState::default();
        state.parse_in_margin(&make_in_margin(r#"left="11" right="22" top="33" bottom="44""#));
        let m = state.inner_margin.unwrap();
        assert_eq!(m.left, 11);
        assert_eq!(m.right, 22);
        assert_eq!(m.top, 33);
        assert_eq!(m.bottom, 44);
    }

    #[test]
    fn parse_in_margin_partial_left_only_defaults_to_141() {
        let mut state = TableState::default();
        state.parse_in_margin(&make_in_margin(r#"left="200""#));
        let m = state.inner_margin.unwrap();
        assert_eq!(m.left, 200, "left must be overridden");
        assert_eq!(m.right, 141, "right must default to 141");
        assert_eq!(m.top, 141, "top must default to 141");
        assert_eq!(m.bottom, 141, "bottom must default to 141");
    }

    #[test]
    fn parse_in_margin_partial_top_bottom_defaults_to_141() {
        let mut state = TableState::default();
        state.parse_in_margin(&make_in_margin(r#"top="50" bottom="50""#));
        let m = state.inner_margin.unwrap();
        assert_eq!(m.left, 141, "left must default to 141");
        assert_eq!(m.right, 141, "right must default to 141");
        assert_eq!(m.top, 50, "top must be overridden");
        assert_eq!(m.bottom, 50, "bottom must be overridden");
    }

    #[test]
    fn parse_in_margin_no_attrs_all_default_to_141() {
        let mut state = TableState::default();
        state.parse_in_margin(&make_in_margin(""));
        let m = state.inner_margin.unwrap();
        assert_eq!(m.left, 141);
        assert_eq!(m.right, 141);
        assert_eq!(m.top, 141);
        assert_eq!(m.bottom, 141);
    }

    #[test]
    fn parse_in_margin_hp_prefixed_attrs() {
        let mut state = TableState::default();
        state.parse_in_margin(&make_in_margin(r#"hp:left="77" hp:bottom="88""#));
        let m = state.inner_margin.unwrap();
        assert_eq!(m.left, 77, "hp:left must be recognised");
        assert_eq!(m.right, 141, "right must default to 141");
        assert_eq!(m.top, 141, "top must default to 141");
        assert_eq!(m.bottom, 88, "hp:bottom must be recognised");
    }

    #[test]
    fn parse_in_margin_invalid_value_keeps_default() {
        let mut state = TableState::default();
        state.parse_in_margin(&make_in_margin(r#"left="oops" right="50""#));
        let m = state.inner_margin.unwrap();
        assert_eq!(m.left, 141, "invalid value must preserve 141 default");
        assert_eq!(m.right, 50);
        assert_eq!(m.top, 141);
        assert_eq!(m.bottom, 141);
    }

    // ── PageLayoutState parsers ───────────────────────────────────────────

    // parse_page_size ──────────────────────────────────────────────────────

    #[test]
    fn parse_page_size_both_dims() {
        let mut s = PageLayoutState::default();
        s.parse_page_size(&make_empty("hp:pageSize", r#"width="59528" height="84188""#));
        assert_eq!(s.width, Some(59528));
        assert_eq!(s.height, Some(84188));
    }

    #[test]
    fn parse_page_size_hp_prefixed_attrs() {
        let mut s = PageLayoutState::default();
        s.parse_page_size(&make_empty("hp:pageSize", r#"hp:width="42000" hp:height="59528""#));
        assert_eq!(s.width, Some(42000));
        assert_eq!(s.height, Some(59528));
    }

    #[test]
    fn parse_page_size_width_only_leaves_height_none() {
        let mut s = PageLayoutState::default();
        s.parse_page_size(&make_empty("hp:pageSize", r#"width="42000""#));
        assert_eq!(s.width, Some(42000));
        assert_eq!(s.height, None);
    }

    #[test]
    fn parse_page_size_invalid_value_keeps_none() {
        let mut s = PageLayoutState::default();
        s.parse_page_size(&make_empty("hp:pageSize", r#"width="auto" height="84188""#));
        assert_eq!(s.width, None, "invalid width must stay None");
        assert_eq!(s.height, Some(84188));
    }

    // parse_margin ─────────────────────────────────────────────────────────

    #[test]
    fn parse_margin_all_axes() {
        let mut s = PageLayoutState::default();
        s.parse_margin(&make_empty("hp:margin", r#"left="1701" right="1701" top="2000" bottom="1500""#));
        assert_eq!(s.margin_left,   Some(1701));
        assert_eq!(s.margin_right,  Some(1701));
        assert_eq!(s.margin_top,    Some(2000));
        assert_eq!(s.margin_bottom, Some(1500));
    }

    #[test]
    fn parse_margin_hp_prefixed_attrs() {
        let mut s = PageLayoutState::default();
        s.parse_margin(&make_empty("hp:margin", r#"hp:left="800" hp:right="900""#));
        assert_eq!(s.margin_left,  Some(800));
        assert_eq!(s.margin_right, Some(900));
        assert_eq!(s.margin_top,   None);
        assert_eq!(s.margin_bottom, None);
    }

    #[test]
    fn parse_margin_partial_leaves_rest_none() {
        let mut s = PageLayoutState::default();
        s.parse_margin(&make_empty("hp:margin", r#"top="2000""#));
        assert_eq!(s.margin_left,   None);
        assert_eq!(s.margin_right,  None);
        assert_eq!(s.margin_top,    Some(2000));
        assert_eq!(s.margin_bottom, None);
    }

    #[test]
    fn parse_margin_invalid_value_keeps_none() {
        let mut s = PageLayoutState::default();
        s.parse_margin(&make_empty("hp:margin", r#"left="inherit" right="1000""#));
        assert_eq!(s.margin_left,  None, "invalid value must stay None");
        assert_eq!(s.margin_right, Some(1000));
    }

    // parse_page_pr ────────────────────────────────────────────────────────

    #[test]
    fn parse_page_pr_landscape_true() {
        let mut s = PageLayoutState::default();
        s.parse_page_pr(&make_empty("hp:pagePr", r#"landscape="true""#));
        assert!(s.landscape);
    }

    #[test]
    fn parse_page_pr_landscape_one() {
        let mut s = PageLayoutState::default();
        s.parse_page_pr(&make_empty("hp:pagePr", r#"landscape="1""#));
        assert!(s.landscape);
    }

    #[test]
    fn parse_page_pr_landscape_false() {
        let mut s = PageLayoutState { landscape: true, ..PageLayoutState::default() };
        s.parse_page_pr(&make_empty("hp:pagePr", r#"landscape="false""#));
        assert!(!s.landscape);
    }

    #[test]
    fn parse_page_pr_landscape_zero_resets() {
        let mut s = PageLayoutState { landscape: true, ..PageLayoutState::default() };
        s.parse_page_pr(&make_empty("hp:pagePr", r#"landscape="0""#));
        assert!(!s.landscape, "landscape=\"0\" must reset to false");
    }

    #[test]
    fn parse_page_pr_hp_prefixed_landscape() {
        let mut s = PageLayoutState::default();
        s.parse_page_pr(&make_empty("hp:pagePr", r#"hp:landscape="true""#));
        assert!(s.landscape);
    }

    #[test]
    fn parse_page_pr_no_attrs_preserves_existing_landscape() {
        let mut s = PageLayoutState { landscape: true, ..PageLayoutState::default() };
        s.parse_page_pr(&make_empty("hp:pagePr", ""));
        assert!(s.landscape, "absent attr must not touch existing landscape");
    }

    // ── Group A: parse_page_size edge cases ──────────────────────────────

    #[test]
    fn parse_page_size_invalid_value_preserves_existing_some() {
        let mut s = PageLayoutState { width: Some(59528), height: Some(84188), ..PageLayoutState::default() };
        s.parse_page_size(&make_empty("hp:pageSize", r#"width="bogus""#));
        assert_eq!(s.width, Some(59528), "invalid parse must not reset existing Some");
        assert_eq!(s.height, Some(84188), "unmentioned field unchanged");
    }

    #[test]
    fn parse_page_size_negative_value_preserves_existing_some() {
        let mut s = PageLayoutState { width: Some(42000), ..PageLayoutState::default() };
        s.parse_page_size(&make_empty("hp:pageSize", r#"width="-1""#));
        assert_eq!(s.width, Some(42000), "negative fails u32 parse; existing Some preserved");
    }

    #[test]
    fn parse_page_size_overflow_value_preserves_existing_some() {
        let mut s = PageLayoutState { width: Some(42000), ..PageLayoutState::default() };
        s.parse_page_size(&make_empty("hp:pageSize", r#"width="4294967296""#));
        assert_eq!(s.width, Some(42000), "u32::MAX+1 overflow; existing Some preserved");
    }

    // ── Group B: parse_margin edge cases ─────────────────────────────────

    #[test]
    fn parse_margin_invalid_value_preserves_existing_some() {
        let mut s = PageLayoutState { margin_left: Some(1701), margin_right: Some(1701), ..PageLayoutState::default() };
        s.parse_margin(&make_empty("hp:margin", r#"left="oops" right="2000""#));
        assert_eq!(s.margin_left, Some(1701), "invalid left: existing Some preserved");
        assert_eq!(s.margin_right, Some(2000), "valid right: updated");
    }

    #[test]
    fn parse_margin_negative_value_preserves_existing_some() {
        let mut s = PageLayoutState { margin_top: Some(2000), ..PageLayoutState::default() };
        s.parse_margin(&make_empty("hp:margin", r#"top="-50""#));
        assert_eq!(s.margin_top, Some(2000), "negative top fails u32 parse; existing Some preserved");
    }

    #[test]
    fn parse_margin_overflow_value_preserves_existing_some() {
        let mut s = PageLayoutState { margin_bottom: Some(1500), ..PageLayoutState::default() };
        s.parse_margin(&make_empty("hp:margin", r#"bottom="4294967296""#));
        assert_eq!(s.margin_bottom, Some(1500), "overflow bottom; existing Some preserved");
    }

    // ── Group C: parse_page_pr edge cases ──────────────────────────────────

    #[test]
    fn parse_page_pr_unknown_value_preserves_existing() {
        let mut s = PageLayoutState { landscape: true, ..PageLayoutState::default() };
        s.parse_page_pr(&make_empty("hp:pagePr", r#"landscape="yes""#));
        assert!(s.landscape, "unrecognised value preserves prior true state");
    }

    #[test]
    fn parse_page_pr_empty_value_preserves_existing() {
        let mut s = PageLayoutState { landscape: true, ..PageLayoutState::default() };
        s.parse_page_pr(&make_empty("hp:pagePr", r#"landscape="""#));
        assert!(s.landscape, "empty string value preserves prior true state");
    }

    #[test]
    fn parse_page_pr_uppercase_true_preserves_false() {
        let mut s = PageLayoutState::default();
        s.parse_page_pr(&make_empty("hp:pagePr", r#"landscape="TRUE""#));
        assert!(!s.landscape, "case-sensitive: TRUE preserves prior false state");
    }

    #[test]
    fn parse_page_pr_unknown_value_preserves_false() {
        let mut s = PageLayoutState::default(); // landscape: false
        s.parse_page_pr(&make_empty("hp:pagePr", r#"landscape="yes""#));
        assert!(!s.landscape, "unrecognised value preserves prior false state");
    }

    #[test]
    fn parse_page_pr_unknown_attribute_ignored_preserves_landscape() {
        let mut s = PageLayoutState { landscape: true, ..PageLayoutState::default() };
        s.parse_page_pr(&make_empty("hp:pagePr", r#"numbering="continuous""#));
        assert!(s.landscape, "unknown attribute is silently ignored; existing state preserved");
    }

    #[test]
    fn parse_page_pr_mixed_known_and_unknown_attrs() {
        let mut s = PageLayoutState::default();
        s.parse_page_pr(&make_empty(
            "hp:pagePr",
            r#"numbering="continuous" landscape="true" footnote="endOfPage""#,
        ));
        assert!(s.landscape, "landscape parsed correctly among unknown attrs");
    }
}
