use crate::error::Hwp2MdError;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::io::{Cursor, Write};

use super::header::{NUM_PR_DIGIT, PARA_PR_HEADING, PARA_PR_LIST_D0, PARA_PR_LIST_D1};
use super::{CharPrKey, ImageAssetMap, RefTables};
use crate::ir::{self, PageLayout};

/// Build a `PageLayout` from the style template or A4 portrait defaults.
fn style_page_layout(tables: &RefTables) -> PageLayout {
    let style = tables.style.as_ref();
    PageLayout {
        width: style.and_then(|s| s.page.width),
        height: style.and_then(|s| s.page.height),
        landscape: style.and_then(|s| s.page.landscape).unwrap_or(false),
        margin_left: style.and_then(|s| s.page.margin.left),
        margin_right: style.and_then(|s| s.page.margin.right),
        margin_top: style.and_then(|s| s.page.margin.top),
        margin_bottom: style.and_then(|s| s.page.margin.bottom),
    }
}

/// Maximum nesting depth for lists before further sub-levels are silently
/// dropped.  Prevents stack overflow on pathologically deep Markdown input.
const MAX_LIST_DEPTH: u32 = 10;

/// Maximum nesting depth for block quotes before further nesting is silently
/// dropped.  Mirrors the list depth guard to bound mutual recursion between
/// `write_block` and `write_list_items`.
const MAX_QUOTE_DEPTH: u32 = 10;

// ---------------------------------------------------------------------------
// section XML
// ---------------------------------------------------------------------------

/// Generate OWPML section XML for `section`.
///
/// `asset_map` maps each `Block::Image { src }` to the bare filename of the
/// corresponding `BinData/` entry in the HWPX ZIP.  When the src is present in
/// the map the entry name is used as `binaryItemIDRef`; otherwise the original
/// `src` string is emitted unchanged (preserving backward compatibility for
/// remote URLs and unresolved paths).
pub(super) fn generate_section_xml(
    section: &ir::Section,
    _index: usize,
    tables: &RefTables,
    asset_map: &ImageAssetMap,
) -> Result<String, Hwp2MdError> {
    let mut buf = Cursor::new(Vec::new());
    let mut writer = Writer::new_with_indent(&mut buf, b' ', 2);

    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

    let mut sec = BytesStart::new("hs:sec");
    sec.push_attribute(("xmlns:hs", "http://www.hancom.co.kr/hwpml/2011/section"));
    sec.push_attribute(("xmlns:hp", "http://www.hancom.co.kr/hwpml/2011/paragraph"));
    writer.write_event(Event::Start(sec))?;

    // Emit header/footer blocks before <hp:secPr> when present.
    let has_header = section.header.as_ref().is_some_and(|b| !b.is_empty());
    let has_footer = section.footer.as_ref().is_some_and(|b| !b.is_empty());
    if has_header || has_footer {
        let mut hf_elem = BytesStart::new("hp:headerFooter");
        if let Some(hf_type) = &section.header_footer_type {
            hf_elem.push_attribute(("type", hf_type.as_str()));
        }

        writer.write_event(Event::Start(hf_elem))?;
        let mut para_id_hf: u32 = 0;
        if has_header {
            writer.write_event(Event::Start(BytesStart::new("hp:header")))?;
            for block in section.header.as_deref().unwrap_or(&[]) {
                write_block(&mut writer, block, tables, &mut para_id_hf, 0, asset_map)?;
            }
            writer.write_event(Event::End(BytesEnd::new("hp:header")))?;
        }
        if has_footer {
            writer.write_event(Event::Start(BytesStart::new("hp:footer")))?;
            for block in section.footer.as_deref().unwrap_or(&[]) {
                write_block(&mut writer, block, tables, &mut para_id_hf, 0, asset_map)?;
            }
            writer.write_event(Event::End(BytesEnd::new("hp:footer")))?;
        }
        writer.write_event(Event::End(BytesEnd::new("hp:headerFooter")))?;
    }

    // Section-level page_layout takes precedence over the style template.
    let layout = section
        .page_layout
        .unwrap_or_else(|| style_page_layout(tables));
    write_sec_pr(&mut writer, &layout)?;

    let mut para_id: u32 = 0;
    for block in &section.blocks {
        write_block(&mut writer, block, tables, &mut para_id, 0, asset_map)?;
    }

    writer.write_event(Event::End(BytesEnd::new("hs:sec")))?;

    String::from_utf8(buf.into_inner())
        .map_err(|e| Hwp2MdError::HwpxWrite(format!("section XML is not valid UTF-8: {e}")))
}

/// Emit a single IR block as OWPML XML.
///
/// `para_id` is a section-scoped sequential counter.  Every `<hp:p>` element
/// that is directly emitted by this function (including those wrapping tables)
/// consumes one ID and increments the counter.  Paragraph IDs inside table
/// cells share the same counter so that all IDs remain globally unique within
/// the section.
///
/// `quote_depth` tracks how many nested `BlockQuote` wrappers surround the
/// current block.  When > 0 the emitted `<hp:p>` uses `paraPrIDRef="1"`
/// (the indented paragraph property) instead of `"0"`.
///
/// `asset_map` maps image `src` values to the resolved `BinData/` entry name
/// used as `binaryItemIDRef` in the emitted `<hp:img>` element.
// Handles all IR block variants; splitting would scatter related match arms.
#[allow(clippy::too_many_lines)]
fn write_block<W: Write>(
    writer: &mut Writer<W>,
    block: &ir::Block,
    tables: &RefTables,
    para_id: &mut u32,
    quote_depth: u32,
    asset_map: &ImageAssetMap,
) -> Result<(), quick_xml::Error> {
    // Select paraPrIDRef based on blockquote nesting depth.
    let para_pr_ref = if quote_depth > 0 { "1" } else { "0" };

    match block {
        ir::Block::Heading { level, inlines } => {
            // Style IDs match the hh:styles table: 1=Heading1 ... 6=Heading6.
            // Levels outside 1-6 are clamped to the nearest valid ID.
            let style_id = (*level).clamp(1, 6);
            let style_id_str = style_id.to_string();
            let id_str = para_id.to_string();
            *para_id += 1;
            // Headings use their own paraPr entry (id=4) for wider line spacing,
            // unless they are inside a block-quote (para_pr_ref="1" takes priority).
            let heading_para_pr = if quote_depth > 0 {
                para_pr_ref
            } else {
                PARA_PR_HEADING
            };
            let mut p = BytesStart::new("hp:p");
            p.push_attribute(("id", id_str.as_str()));
            p.push_attribute(("hp:styleIDRef", style_id_str.as_str()));
            p.push_attribute(("paraPrIDRef", heading_para_pr));
            writer.write_event(Event::Start(p))?;
            write_inlines(writer, inlines, tables)?;
            writer.write_event(Event::End(BytesEnd::new("hp:p")))?;
        }
        ir::Block::Paragraph { inlines } => {
            let id_str = para_id.to_string();
            *para_id += 1;
            let mut p = BytesStart::new("hp:p");
            p.push_attribute(("id", id_str.as_str()));
            p.push_attribute(("paraPrIDRef", para_pr_ref));
            writer.write_event(Event::Start(p))?;
            write_inlines(writer, inlines, tables)?;
            writer.write_event(Event::End(BytesEnd::new("hp:p")))?;
        }
        ir::Block::Table { rows, col_count } => {
            let id_str = para_id.to_string();
            *para_id += 1;
            let mut p = BytesStart::new("hp:p");
            p.push_attribute(("id", id_str.as_str()));
            p.push_attribute(("paraPrIDRef", para_pr_ref));
            writer.write_event(Event::Start(p))?;

            let mut run = BytesStart::new("hp:run");
            run.push_attribute(("charPrIDRef", "0"));
            writer.write_event(Event::Start(run))?;

            write_table(writer, rows, *col_count, tables, para_id, quote_depth, asset_map)?;

            writer.write_event(Event::End(BytesEnd::new("hp:run")))?;
            writer.write_event(Event::End(BytesEnd::new("hp:p")))?;
        }
        ir::Block::CodeBlock { code, language } => {
            // Emit a language-hint comment before the paragraph so the reader
            // can reconstruct `CodeBlock` with the correct `language` field on
            // roundtrip.  The convention is:
            //   <!-- hwp2md:lang:LANG -->   (e.g. <!-- hwp2md:lang:python -->)
            //   <!-- hwp2md:lang: -->       (no language hint)
            //
            // The comment is valid XML and invisible to OWPML validators.
            // Sanitize the language string so it cannot break the XML comment.
            // XML comments must not contain `--`; replace every occurrence of
            // `--` with a single `-` to keep the output well-formed.
            let raw = language.as_deref().unwrap_or("");
            let sanitized = raw.replace("--", "-");
            let lang_str = sanitized.as_str();
            let comment_text = format!(" hwp2md:lang:{lang_str} ");
            writer.write_event(Event::Comment(BytesText::new(&comment_text)))?;

            let code_id = tables.code_block_char_pr_id().to_string();
            let id_str = para_id.to_string();
            *para_id += 1;
            let mut p = BytesStart::new("hp:p");
            p.push_attribute(("id", id_str.as_str()));
            p.push_attribute(("paraPrIDRef", para_pr_ref));
            writer.write_event(Event::Start(p))?;
            let mut run = BytesStart::new("hp:run");
            run.push_attribute(("charPrIDRef", code_id.as_str()));
            writer.write_event(Event::Start(run))?;
            writer.write_event(Event::Start(BytesStart::new("hp:t")))?;
            writer.write_event(Event::Text(BytesText::new(code)))?;
            writer.write_event(Event::End(BytesEnd::new("hp:t")))?;
            writer.write_event(Event::End(BytesEnd::new("hp:run")))?;
            writer.write_event(Event::End(BytesEnd::new("hp:p")))?;
        }
        ir::Block::BlockQuote { blocks } => {
            // Depth guard: silently stop recursing beyond MAX_QUOTE_DEPTH to
            // prevent stack overflow on pathologically deeply nested quotes.
            if quote_depth >= MAX_QUOTE_DEPTH {
                return Ok(());
            }
            for b in blocks {
                write_block(writer, b, tables, para_id, quote_depth + 1, asset_map)?;
            }
        }
        ir::Block::List { items, ordered, .. } => {
            write_list_items(
                writer,
                items,
                *ordered,
                tables,
                para_id,
                quote_depth,
                0,
                asset_map,
            )?;
        }
        ir::Block::Image { src, alt } => {
            // Resolve the src to the embedded BinData entry name when available.
            // For remote URLs (not in the map) the original src is used as-is.
            let bin_ref: &str = asset_map.get(src.as_str()).map_or(src, String::as_str);

            let id_str = para_id.to_string();
            *para_id += 1;
            let mut p = BytesStart::new("hp:p");
            p.push_attribute(("id", id_str.as_str()));
            p.push_attribute(("paraPrIDRef", para_pr_ref));
            writer.write_event(Event::Start(p))?;
            let mut run = BytesStart::new("hp:run");
            run.push_attribute(("charPrIDRef", "0"));
            writer.write_event(Event::Start(run))?;
            writer.write_event(Event::Start(BytesStart::new("hp:pic")))?;
            let mut img = BytesStart::new("hp:img");
            img.push_attribute(("hp:binaryItemIDRef", bin_ref));
            img.push_attribute(("alt", alt.as_str()));
            writer.write_event(Event::Empty(img))?;
            writer.write_event(Event::End(BytesEnd::new("hp:pic")))?;
            writer.write_event(Event::End(BytesEnd::new("hp:run")))?;
            writer.write_event(Event::End(BytesEnd::new("hp:p")))?;
        }
        ir::Block::HorizontalRule => {
            let id_str = para_id.to_string();
            *para_id += 1;
            let mut p = BytesStart::new("hp:p");
            p.push_attribute(("id", id_str.as_str()));
            p.push_attribute(("paraPrIDRef", para_pr_ref));
            writer.write_event(Event::Start(p))?;
            let mut run = BytesStart::new("hp:run");
            run.push_attribute(("charPrIDRef", "0"));
            writer.write_event(Event::Start(run))?;
            writer.write_event(Event::Start(BytesStart::new("hp:t")))?;
            writer.write_event(Event::Text(BytesText::new(
                "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
            )))?;
            writer.write_event(Event::End(BytesEnd::new("hp:t")))?;
            writer.write_event(Event::End(BytesEnd::new("hp:run")))?;
            writer.write_event(Event::End(BytesEnd::new("hp:p")))?;
        }
        ir::Block::PageBreak => {
            // Emit `<hp:p>` containing an empty `<hp:ctrl id="newPage"/>`
            // inside an `<hp:run>`.  Hancom Office and the OWPML reference
            // recognise this as a forced page break.
            let id_str = para_id.to_string();
            *para_id += 1;
            let mut p = BytesStart::new("hp:p");
            p.push_attribute(("id", id_str.as_str()));
            p.push_attribute(("paraPrIDRef", para_pr_ref));
            writer.write_event(Event::Start(p))?;
            let mut run = BytesStart::new("hp:run");
            run.push_attribute(("charPrIDRef", "0"));
            writer.write_event(Event::Start(run))?;
            let mut ctrl = BytesStart::new("hp:ctrl");
            ctrl.push_attribute(("id", "newPage"));
            writer.write_event(Event::Empty(ctrl))?;
            writer.write_event(Event::End(BytesEnd::new("hp:run")))?;
            writer.write_event(Event::End(BytesEnd::new("hp:p")))?;
        }
        ir::Block::Math { tex, .. } => {
            let id_str = para_id.to_string();
            *para_id += 1;
            let mut p = BytesStart::new("hp:p");
            p.push_attribute(("id", id_str.as_str()));
            p.push_attribute(("paraPrIDRef", para_pr_ref));
            writer.write_event(Event::Start(p))?;
            let mut run = BytesStart::new("hp:run");
            run.push_attribute(("charPrIDRef", "0"));
            writer.write_event(Event::Start(run))?;
            writer.write_event(Event::Start(BytesStart::new("hp:equation")))?;
            writer.write_event(Event::Text(BytesText::new(tex)))?;
            writer.write_event(Event::End(BytesEnd::new("hp:equation")))?;
            writer.write_event(Event::End(BytesEnd::new("hp:run")))?;
            writer.write_event(Event::End(BytesEnd::new("hp:p")))?;
        }
        ir::Block::Footnote { id, content } => {
            let mut fn_el = BytesStart::new("hp:fn");
            fn_el.push_attribute(("noteId", id.as_str()));
            writer.write_event(Event::Start(fn_el))?;
            for b in content {
                write_block(writer, b, tables, para_id, quote_depth, asset_map)?;
            }
            writer.write_event(Event::End(BytesEnd::new("hp:fn")))?;
        }
    }
    Ok(())
}

/// Emit a complete OWPML `<hp:tbl>` element for the given rows.
///
/// Caller is responsible for the surrounding `<hp:p>` / `<hp:run>` wrappers.
/// The table element structure emitted follows the OWPML spec:
///
/// ```xml
/// <hp:tbl rowCnt="…" colCnt="…" borderFillIDRef="2" noAdjust="0">
///   <hp:tblPr>
///     <hp:inMargin left="141" right="141" top="141" bottom="141"/>
///   </hp:tblPr>
///   <hp:sz width="…" height="…"/>
///   <hp:pos treatAsChar="1"/>
///   <hp:tr>
///     <hp:trHeight value="1000"/>
///     <hp:tc>
///       <hp:cellAddr colAddr="…" rowAddr="…"/>
///       <hp:cellSpan colSpan="…" rowSpan="…"/>
///       <hp:cellSz width="8000" height="1000"/>
///       <hp:cellMargin left="510" right="510" top="141" bottom="141"/>
///       <hp:subList>…paragraphs…</hp:subList>
///     </hp:tc>
///   </hp:tr>
/// </hp:tbl>
/// ```
///
/// Cell widths use 8 000 HWP units per column (≈ 80 mm) and row height is
/// 1 000 HWP units (≈ 10 mm).  `borderFillIDRef="2"` references the
/// all-black solid border entry defined in `header.xml`.
/// Default cell width in HWP units (8 000 ≈ 80 mm) used for table sizing.
const TABLE_CELL_WIDTH: usize = 8_000;

/// Default row height in HWP units (1 000 ≈ 10 mm) used for table sizing.
const TABLE_CELL_HEIGHT: usize = 1_000;

/// Inner margin between table cells (left/right/top/bottom), ~1 mm in HWP units.
const TABLE_INNER_MARGIN: &str = "141";

/// Horizontal cell padding (left and right inner padding), ~3.6 mm in HWP units.
const TABLE_CELL_PAD_H: &str = "510";

/// Vertical cell padding (top and bottom inner padding), ~1 mm in HWP units.
const TABLE_CELL_PAD_V: &str = "141";

#[allow(clippy::too_many_arguments)]
fn write_table<W: Write>(
    writer: &mut Writer<W>,
    rows: &[ir::TableRow],
    col_count: usize,
    tables: &RefTables,
    para_id: &mut u32,
    quote_depth: u32,
    asset_map: &ImageAssetMap,
) -> Result<(), quick_xml::Error> {
    let row_cnt = rows.len();
    // Fall back to scanning the widest row when col_count is 0.
    let col_cnt = if col_count > 0 {
        col_count
    } else {
        rows.iter().map(|r| r.cells.len()).max().unwrap_or(0)
    };

    let tbl_width = (col_cnt * TABLE_CELL_WIDTH).to_string();
    let tbl_height = (row_cnt * TABLE_CELL_HEIGHT).to_string();
    let row_cnt_str = row_cnt.to_string();
    let col_cnt_str = col_cnt.to_string();
    let cell_height_str = TABLE_CELL_HEIGHT.to_string();

    let mut tbl = BytesStart::new("hp:tbl");
    tbl.push_attribute(("rowCnt", row_cnt_str.as_str()));
    tbl.push_attribute(("colCnt", col_cnt_str.as_str()));
    let table_fill_ref = tables.table_border_fill_id.to_string();
    tbl.push_attribute(("borderFillIDRef", table_fill_ref.as_str()));
    tbl.push_attribute(("noAdjust", "0"));
    writer.write_event(Event::Start(tbl))?;

    // <hp:tblPr> — table properties: inner cell gap margin
    writer.write_event(Event::Start(BytesStart::new("hp:tblPr")))?;
    let mut in_margin = BytesStart::new("hp:inMargin");
    in_margin.push_attribute(("left", TABLE_INNER_MARGIN));
    in_margin.push_attribute(("right", TABLE_INNER_MARGIN));
    in_margin.push_attribute(("top", TABLE_INNER_MARGIN));
    in_margin.push_attribute(("bottom", TABLE_INNER_MARGIN));
    writer.write_event(Event::Empty(in_margin))?;
    writer.write_event(Event::End(BytesEnd::new("hp:tblPr")))?;

    // <hp:sz> — overall table dimensions
    let mut sz = BytesStart::new("hp:sz");
    sz.push_attribute(("width", tbl_width.as_str()));
    sz.push_attribute(("height", tbl_height.as_str()));
    writer.write_event(Event::Empty(sz))?;

    // <hp:pos> — inline (treat-as-character) positioning
    let mut pos = BytesStart::new("hp:pos");
    pos.push_attribute(("treatAsChar", "1"));
    writer.write_event(Event::Empty(pos))?;

    for (row_idx, row) in rows.iter().enumerate() {
        let row_idx_str = row_idx.to_string();
        writer.write_event(Event::Start(BytesStart::new("hp:tr")))?;

        // <hp:trHeight> — default row height
        let mut tr_height = BytesStart::new("hp:trHeight");
        tr_height.push_attribute(("value", cell_height_str.as_str()));
        writer.write_event(Event::Empty(tr_height))?;

        for (col_idx, cell) in row.cells.iter().enumerate() {
            let col_idx_str = col_idx.to_string();
            let colspan_str = cell.colspan.to_string();
            let rowspan_str = cell.rowspan.to_string();

            writer.write_event(Event::Start(BytesStart::new("hp:tc")))?;

            // <hp:cellAddr> — cell position
            let mut cell_addr = BytesStart::new("hp:cellAddr");
            cell_addr.push_attribute(("colAddr", col_idx_str.as_str()));
            cell_addr.push_attribute(("rowAddr", row_idx_str.as_str()));
            writer.write_event(Event::Empty(cell_addr))?;

            // <hp:cellSpan> — span information (always emitted, even for 1×1)
            let mut cell_span = BytesStart::new("hp:cellSpan");
            cell_span.push_attribute(("colSpan", colspan_str.as_str()));
            cell_span.push_attribute(("rowSpan", rowspan_str.as_str()));
            writer.write_event(Event::Empty(cell_span))?;

            // <hp:cellSz> — cell size scales with colspan/rowspan; .max(1) guards malformed 0 values
            let cell_w = (cell.colspan.max(1) as usize * TABLE_CELL_WIDTH).to_string();
            let cell_h = (cell.rowspan.max(1) as usize * TABLE_CELL_HEIGHT).to_string();
            let mut cell_sz = BytesStart::new("hp:cellSz");
            cell_sz.push_attribute(("width", cell_w.as_str()));
            cell_sz.push_attribute(("height", cell_h.as_str()));
            writer.write_event(Event::Empty(cell_sz))?;

            // <hp:cellMargin> — inner padding (HWP units)
            let mut cell_margin = BytesStart::new("hp:cellMargin");
            cell_margin.push_attribute(("left", TABLE_CELL_PAD_H));
            cell_margin.push_attribute(("right", TABLE_CELL_PAD_H));
            cell_margin.push_attribute(("top", TABLE_CELL_PAD_V));
            cell_margin.push_attribute(("bottom", TABLE_CELL_PAD_V));
            writer.write_event(Event::Empty(cell_margin))?;

            // <hp:subList> — cell content (paragraphs)
            writer.write_event(Event::Start(BytesStart::new("hp:subList")))?;
            for b in &cell.blocks {
                write_block(writer, b, tables, para_id, quote_depth, asset_map)?;
            }
            writer.write_event(Event::End(BytesEnd::new("hp:subList")))?;

            writer.write_event(Event::End(BytesEnd::new("hp:tc")))?;
        }

        writer.write_event(Event::End(BytesEnd::new("hp:tr")))?;
    }

    writer.write_event(Event::End(BytesEnd::new("hp:tbl")))?;
    Ok(())
}

/// Emit a sequence of list items as OWPML paragraphs with numbering markup.
///
/// Each item in the list is written as a `<hp:p>` paragraph carrying two
/// numbering-related attributes:
///
/// - `numPrIDRef`: references the `<hh:numbering>` definition in the header
///   (`NUM_PR_BULLET` for unordered, `NUM_PR_DIGIT` for ordered).
/// - `paraPrIDRef`: references the indented paragraph property (`PARA_PR_LIST_D0`
///   for depth 0, `PARA_PR_LIST_D1` for depth ≥ 1).
///
/// After emitting the item's inline content, any nested child list is recurse
/// with `depth + 1`.  The `ordered` flag from the parent list is propagated
/// to child lists unless overridden by a nested `Block::List` inside an item.
///
/// `quote_depth` is passed through unchanged — list items can appear inside
/// block quotes and must still carry the correct `paraPrIDRef` for quoting.
/// However, the list's own indentation takes priority: when inside a quote,
/// the list paragraph still uses the list-specific `paraPrIDRef` rather than
/// the blockquote `paraPrIDRef`.  This matches Hancom's behaviour.
#[allow(clippy::too_many_arguments)]
fn write_list_items<W: Write>(
    writer: &mut Writer<W>,
    items: &[ir::ListItem],
    ordered: bool,
    tables: &RefTables,
    para_id: &mut u32,
    quote_depth: u32,
    list_depth: u32,
    asset_map: &ImageAssetMap,
) -> Result<(), quick_xml::Error> {
    // Depth guard: silently stop recursing beyond MAX_LIST_DEPTH to prevent
    // stack overflow when `write_block` (called for non-paragraph items)
    // and this function mutually recurse on deeply nested list input.
    if list_depth >= MAX_LIST_DEPTH {
        return Ok(());
    }

    // Select the paragraph property ID that controls left indentation.
    // OWPML only has two list paragraph styles: D0 (top level) and D1
    // (indented for depth ≥ 1).  Depths > 1 are intentionally mapped to D1
    // because no additional paraPr entries are defined for deeper nesting —
    // they appear visually identical to depth-1 items in Hancom Writer.
    let para_pr_ref = if list_depth == 0 {
        PARA_PR_LIST_D0
    } else {
        PARA_PR_LIST_D1
    };

    for item in items {
        // Emit each top-level block inside the item.  A well-formed Markdown
        // list item contains exactly one Paragraph, but we handle arbitrary
        // blocks for correctness.
        for block in &item.blocks {
            match block {
                // The common case: a single paragraph of inline text.
                // Emit it directly with list-specific attributes.
                ir::Block::Paragraph { inlines } => {
                    let id_str = para_id.to_string();
                    *para_id += 1;
                    let mut p = BytesStart::new("hp:p");
                    p.push_attribute(("id", id_str.as_str()));
                    p.push_attribute(("paraPrIDRef", para_pr_ref));
                    // Ordered lists reference the DIGIT numbering definition.
                    // Unordered (bullet) lists omit numPrIDRef — indentation
                    // is handled by the paragraph property alone.
                    if ordered {
                        p.push_attribute(("numPrIDRef", NUM_PR_DIGIT));
                    }
                    writer.write_event(Event::Start(p))?;
                    // For task list items, prepend a checkbox character run
                    // before the normal inline content.
                    if let Some(checked) = item.checked {
                        let checkbox = if checked { "☑ " } else { "☐ " };
                        let checkbox_inline = ir::Inline::plain(checkbox);
                        write_inline_run(writer, &checkbox_inline, tables)?;
                    }
                    write_inlines(writer, inlines, tables)?;
                    writer.write_event(Event::End(BytesEnd::new("hp:p")))?;
                }
                // Nested list inside an item: recurse with depth + 1,
                // preserving the parent ordered flag for the child unless
                // the child specifies its own ordered flag.
                ir::Block::List {
                    items: sub_items,
                    ordered: sub_ordered,
                    ..
                } => {
                    write_list_items(
                        writer,
                        sub_items,
                        *sub_ordered,
                        tables,
                        para_id,
                        quote_depth,
                        list_depth + 1,
                        asset_map,
                    )?;
                }
                // Any other block type (heading, table, code block, …) falls
                // back to the generic block writer without list attributes.
                other => {
                    write_block(writer, other, tables, para_id, quote_depth, asset_map)?;
                }
            }
        }

        // Recurse into direct child list items (sub-lists stored as children
        // of the ListItem rather than as a nested Block::List).
        if !item.children.is_empty() {
            write_list_items(
                writer,
                &item.children,
                ordered,
                tables,
                para_id,
                quote_depth,
                list_depth + 1,
                asset_map,
            )?;
        }
    }
    Ok(())
}

fn write_inlines<W: Write>(
    writer: &mut Writer<W>,
    inlines: &[ir::Inline],
    tables: &RefTables,
) -> Result<(), quick_xml::Error> {
    // Group consecutive inlines by link URL so that a single hyperlink span
    // wraps all inlines sharing the same link target.
    let mut i = 0;
    while i < inlines.len() {
        if let Some(ref url) = inlines[i].link {
            // Find the end of the consecutive group sharing the same URL.
            let url = url.clone();
            let group_start = i;
            while i < inlines.len() && inlines[i].link.as_deref() == Some(&url) {
                i += 1;
            }
            write_hyperlink_group(writer, &inlines[group_start..i], &url, tables)?;
        } else {
            write_inline_run(writer, &inlines[i], tables)?;
            i += 1;
        }
    }
    Ok(())
}

/// Emit the `<hp:secPr>` element with page layout metadata.
///
/// In OWPML, `<hp:secPr>` is a direct child of `<hs:sec>` and must appear
/// before any paragraph content.  The page dimensions and margins are expressed
/// in HWP units (1/7200 inch).
fn write_sec_pr<W: Write>(
    writer: &mut Writer<W>,
    layout: &PageLayout,
) -> Result<(), quick_xml::Error> {
    writer.write_event(Event::Start(BytesStart::new("hp:secPr")))?;

    // <hp:pagePr landscape="…">
    let landscape_str = if layout.landscape { "true" } else { "false" };
    let mut page_pr = BytesStart::new("hp:pagePr");
    page_pr.push_attribute(("landscape", landscape_str));
    writer.write_event(Event::Start(page_pr))?;

    // <hp:margin left="…" right="…" top="…" bottom="…" header="0" footer="0" gutter="0"/>
    let left = layout.margin_left.unwrap_or(5670).to_string();
    let right = layout.margin_right.unwrap_or(5670).to_string();
    let top = layout.margin_top.unwrap_or(4252).to_string();
    let bottom = layout.margin_bottom.unwrap_or(4252).to_string();
    let mut margin = BytesStart::new("hp:margin");
    margin.push_attribute(("left", left.as_str()));
    margin.push_attribute(("right", right.as_str()));
    margin.push_attribute(("top", top.as_str()));
    margin.push_attribute(("bottom", bottom.as_str()));
    margin.push_attribute(("header", "0"));
    margin.push_attribute(("footer", "0"));
    margin.push_attribute(("gutter", "0"));
    writer.write_event(Event::Empty(margin))?;

    // <hp:pageSize width="…" height="…"/>
    let width = layout.width.unwrap_or(59528).to_string();
    let height = layout.height.unwrap_or(84188).to_string();
    let mut page_size = BytesStart::new("hp:pageSize");
    page_size.push_attribute(("width", width.as_str()));
    page_size.push_attribute(("height", height.as_str()));
    writer.write_event(Event::Empty(page_size))?;

    writer.write_event(Event::End(BytesEnd::new("hp:pagePr")))?;
    writer.write_event(Event::End(BytesEnd::new("hp:secPr")))?;
    Ok(())
}

/// Emit a single non-link inline as an `<hp:run>` with text.
///
/// When the inline carries a ruby annotation, the run body uses the OWPML
/// `<hp:ruby>` structure instead of a plain `<hp:t>`.  When a `footnote_ref`
/// is set and the text is empty, a `<hp:noteRef>` element is emitted instead.
///
/// Returns immediately without emitting anything when the inline has no
/// meaningful content (empty text, no ruby, no `footnote_ref`).
fn write_inline_run<W: Write>(
    writer: &mut Writer<W>,
    inline: &ir::Inline,
    tables: &RefTables,
) -> Result<(), quick_xml::Error> {
    if inline.text.is_empty() && inline.ruby.is_none() && inline.footnote_ref.is_none() {
        return Ok(());
    }

    let key = CharPrKey::from_inline(inline, &tables.code_font);
    let char_pr_id = tables.char_pr_id(&key);

    let mut run = BytesStart::new("hp:run");
    run.push_attribute(("charPrIDRef", char_pr_id.to_string().as_str()));
    writer.write_event(Event::Start(run))?;

    // Emit section-level inline charPr when any formatting is set.
    // Header charPr only carries font/height/textColor/supscript; bold,
    // italic, underline, and strikeout live on the section-level charPr.
    write_inline_charpr(writer, inline, tables)?;

    if let Some(ref annotation) = inline.ruby {
        // Ruby annotation: wrap base text and annotation in <hp:ruby>.
        writer.write_event(Event::Start(BytesStart::new("hp:ruby")))?;

        writer.write_event(Event::Start(BytesStart::new("hp:baseText")))?;
        writer.write_event(Event::Start(BytesStart::new("hp:t")))?;
        writer.write_event(Event::Text(BytesText::new(&inline.text)))?;
        writer.write_event(Event::End(BytesEnd::new("hp:t")))?;
        writer.write_event(Event::End(BytesEnd::new("hp:baseText")))?;

        writer.write_event(Event::Start(BytesStart::new("hp:rubyText")))?;
        writer.write_event(Event::Start(BytesStart::new("hp:t")))?;
        writer.write_event(Event::Text(BytesText::new(annotation)))?;
        writer.write_event(Event::End(BytesEnd::new("hp:t")))?;
        writer.write_event(Event::End(BytesEnd::new("hp:rubyText")))?;

        writer.write_event(Event::End(BytesEnd::new("hp:ruby")))?;
    } else if inline.text.is_empty() {
        if let Some(ref note_id) = inline.footnote_ref {
            // Footnote reference marker (no visible text).
            let mut note_ref = BytesStart::new("hp:noteRef");
            note_ref.push_attribute(("noteId", note_id.as_str()));
            note_ref.push_attribute(("type", "FOOTNOTE"));
            writer.write_event(Event::Empty(note_ref))?;
        }
    } else {
        writer.write_event(Event::Start(BytesStart::new("hp:t")))?;
        writer.write_event(Event::Text(BytesText::new(&inline.text)))?;
        writer.write_event(Event::End(BytesEnd::new("hp:t")))?;
    }

    writer.write_event(Event::End(BytesEnd::new("hp:run")))?;
    Ok(())
}

/// Emit an inline `<hp:charPr>` element inside `<hp:run>` when the inline
/// carries any formatting flags (bold, italic, underline, strikethrough,
/// superscript, subscript, a non-default color, or a custom font name).
///
/// This is the *section-level* charPr (as opposed to the header-level charPr
/// which only stores font/height/textColor/supscript).  Bold and italic in
/// particular MUST be emitted here for OWPML conformance.
///
/// When `font_name` is set, a `faceNameIDRef` attribute is emitted pointing
/// to the index of that font in the header fontface table.  This allows the
/// reader to resolve the font name on roundtrip via `apply_charpr_attrs`.
fn write_inline_charpr<W: Write>(
    writer: &mut Writer<W>,
    inline: &ir::Inline,
    tables: &RefTables,
) -> Result<(), quick_xml::Error> {
    let has_formatting = inline.bold
        || inline.italic
        || inline.underline
        || inline.strikethrough
        || inline.superscript
        || inline.subscript
        || inline.color.is_some()
        || inline.font_name.is_some();

    if !has_formatting {
        return Ok(());
    }

    let mut charpr = BytesStart::new("hp:charPr");

    if inline.bold {
        charpr.push_attribute(("bold", "true"));
    }
    if inline.italic {
        charpr.push_attribute(("italic", "true"));
    }
    if inline.underline {
        charpr.push_attribute(("underline", "true"));
    }
    if inline.strikethrough {
        charpr.push_attribute(("strikeout", "true"));
    }
    if inline.superscript {
        charpr.push_attribute(("supscript", "superscript"));
    } else if inline.subscript {
        charpr.push_attribute(("supscript", "subscript"));
    }
    if let Some(ref color) = inline.color {
        // IR stores color as "#RRGGBB"; OWPML expects "RRGGBB" (no leading #).
        let raw = color.strip_prefix('#').unwrap_or(color);
        charpr.push_attribute(("color", raw));
    }
    if let Some(ref font) = inline.font_name {
        // Resolve font name to its index in the header fontface table.
        if let Some(idx) = tables.font_names.iter().position(|f| f == font) {
            charpr.push_attribute(("faceNameIDRef", idx.to_string().as_str()));
        }
    }

    writer.write_event(Event::Empty(charpr))?;
    Ok(())
}

/// Emit the OWPML fieldBegin/fieldEnd HYPERLINK pattern for a group of inlines
/// that all share the same link URL.
fn write_hyperlink_group<W: Write>(
    writer: &mut Writer<W>,
    inlines: &[ir::Inline],
    url: &str,
    tables: &RefTables,
) -> Result<(), quick_xml::Error> {
    // fieldBegin run (uses plain charPr id=0)
    let mut begin_run = BytesStart::new("hp:run");
    begin_run.push_attribute(("charPrIDRef", "0"));
    writer.write_event(Event::Start(begin_run))?;
    let mut field_begin = BytesStart::new("hp:fieldBegin");
    field_begin.push_attribute(("type", "HYPERLINK"));
    field_begin.push_attribute(("command", url));
    writer.write_event(Event::Empty(field_begin))?;
    writer.write_event(Event::End(BytesEnd::new("hp:run")))?;

    // Content runs
    for inline in inlines {
        write_inline_run(writer, inline, tables)?;
    }

    // fieldEnd run (uses plain charPr id=0)
    let mut end_run = BytesStart::new("hp:run");
    end_run.push_attribute(("charPrIDRef", "0"));
    writer.write_event(Event::Start(end_run))?;
    let mut field_end = BytesStart::new("hp:fieldEnd");
    field_end.push_attribute(("type", "HYPERLINK"));
    writer.write_event(Event::Empty(field_end))?;
    writer.write_event(Event::End(BytesEnd::new("hp:run")))?;

    Ok(())
}
