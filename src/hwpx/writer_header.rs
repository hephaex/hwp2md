use crate::error::Hwp2MdError;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::io::{Cursor, Write};

use super::{CharPrKey, RefTables, LANG_SLOTS};
use crate::ir;

// ---------------------------------------------------------------------------
// header.xml -- OWPML reference tables
// ---------------------------------------------------------------------------

/// OWPML numbering definition ID for ordered (digit) lists.
///
/// Referenced by `numPrIDRef` on list-item paragraphs in section XML.
/// Unordered (bullet) list items do **not** use a `numPrIDRef` — their
/// visual bullet character and indentation are handled by the paragraph
/// property (`paraPrIDRef`) alone.
pub(super) const NUM_PR_DIGIT: &str = "1";

/// OWPML paragraph-property ID for top-level list items (depth = 0).
///
/// This entry carries the left-indent that visually separates list items
/// from the enclosing paragraph text.
pub(super) const PARA_PR_LIST_D0: &str = "2";

/// OWPML paragraph-property ID for nested list items (depth ≥ 1).
///
/// Each additional nesting level adds another step of left-indentation.
pub(super) const PARA_PR_LIST_D1: &str = "3";

/// Left margin (HWP units) applied to top-level list item paragraphs.
///
/// One HWP unit = 0.01 mm, so 400 ≈ 4 mm.
const LIST_D0_MARGIN_LEFT: &str = "400";

/// Left margin (HWP units) applied to nested list item paragraphs (depth ≥ 1).
const LIST_D1_MARGIN_LEFT: &str = "800";

pub(super) fn generate_header_xml(
    doc: &ir::Document,
    tables: &RefTables,
) -> Result<String, Hwp2MdError> {
    let mut buf = Cursor::new(Vec::new());
    let mut w = Writer::new_with_indent(&mut buf, b' ', 2);

    w.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

    let sec_cnt = doc.sections.len().max(1);

    // <hh:head version="1.1" secCnt="N">
    let mut head = BytesStart::new("hh:head");
    head.push_attribute(("xmlns:hh", "http://www.hancom.co.kr/hwpml/2011/head"));
    head.push_attribute(("version", "1.1"));
    head.push_attribute(("secCnt", sec_cnt.to_string().as_str()));
    w.write_event(Event::Start(head))?;

    // <hh:beginNum> -- required structural marker
    let mut begin_num = BytesStart::new("hh:beginNum");
    begin_num.push_attribute(("page", "1"));
    begin_num.push_attribute(("footnote", "1"));
    begin_num.push_attribute(("endnote", "1"));
    begin_num.push_attribute(("pic", "1"));
    begin_num.push_attribute(("tbl", "1"));
    begin_num.push_attribute(("equation", "1"));
    w.write_event(Event::Empty(begin_num))?;

    // <hh:refList> -- contains fontfaces, charProperties, paraProperties, styles,
    // and numbering definitions.
    w.write_event(Event::Start(BytesStart::new("hh:refList")))?;

    write_font_faces(&mut w, tables)?;
    write_border_fills(&mut w, tables)?;
    write_char_properties(&mut w, tables)?;
    write_para_properties(&mut w)?;
    write_styles(&mut w, tables)?;

    // <hh:numberingList> must be inside <hh:refList> per OWPML schema.
    // id=1 → unordered (bullet ●), id=2 → ordered (digit %d.)
    write_numbering_list(&mut w)?;

    w.write_event(Event::End(BytesEnd::new("hh:refList")))?;

    // <hh:trackchangeConfig> -- required by schema (minOccurs=1)
    w.write_event(Event::Empty(BytesStart::new("hh:trackchangeConfig")))?;

    w.write_event(Event::End(BytesEnd::new("hh:head")))?;

    String::from_utf8(buf.into_inner())
        .map_err(|e| Hwp2MdError::HwpxWrite(format!("header XML is not valid UTF-8: {e}")))
}

fn write_font_faces<W: Write>(
    w: &mut Writer<W>,
    tables: &RefTables,
) -> Result<(), quick_xml::Error> {
    let font_cnt = tables.font_names.len().to_string();

    let mut fontfaces = BytesStart::new("hh:fontfaces");
    fontfaces.push_attribute(("itemCnt", LANG_SLOTS.len().to_string().as_str()));
    w.write_event(Event::Start(fontfaces))?;

    for lang in LANG_SLOTS {
        let mut face = BytesStart::new("hh:fontface");
        face.push_attribute(("lang", lang));
        face.push_attribute(("fontCnt", font_cnt.as_str()));
        w.write_event(Event::Start(face))?;

        for (id, name) in tables.font_names.iter().enumerate() {
            let mut font = BytesStart::new("hh:font");
            font.push_attribute(("id", id.to_string().as_str()));
            font.push_attribute(("face", name.as_str()));
            font.push_attribute(("type", "REP"));
            w.write_event(Event::Empty(font))?;
        }

        w.write_event(Event::End(BytesEnd::new("hh:fontface")))?;
    }

    w.write_event(Event::End(BytesEnd::new("hh:fontfaces")))?;
    Ok(())
}

fn write_border_fills<W: Write>(
    w: &mut Writer<W>,
    tables: &RefTables,
) -> Result<(), quick_xml::Error> {
    let mut border_fills = BytesStart::new("hh:borderFills");
    border_fills.push_attribute(("itemCnt", "1"));
    w.write_event(Event::Start(border_fills))?;

    let id_str = tables.border_fill_id.to_string();
    let mut bf = BytesStart::new("hh:borderFill");
    bf.push_attribute(("id", id_str.as_str()));
    bf.push_attribute(("threeD", "false"));
    bf.push_attribute(("shadow", "false"));
    w.write_event(Event::Start(bf))?;

    let mut slash = BytesStart::new("hh:slash");
    slash.push_attribute(("type", "NONE"));
    slash.push_attribute(("Crooked", "false"));
    slash.push_attribute(("isCounter", "false"));
    w.write_event(Event::Empty(slash))?;

    let mut back_slash = BytesStart::new("hh:backSlash");
    back_slash.push_attribute(("type", "NONE"));
    back_slash.push_attribute(("Crooked", "false"));
    back_slash.push_attribute(("isCounter", "false"));
    w.write_event(Event::Empty(back_slash))?;

    for border_name in &[
        "hh:leftBorder",
        "hh:rightBorder",
        "hh:topBorder",
        "hh:bottomBorder",
        "hh:diagonal",
    ] {
        let mut border = BytesStart::new(*border_name);
        border.push_attribute(("type", "NONE"));
        border.push_attribute(("width", "0.12 mm"));
        border.push_attribute(("color", "000000"));
        w.write_event(Event::Empty(border))?;
    }

    w.write_event(Event::End(BytesEnd::new("hh:borderFill")))?;
    w.write_event(Event::End(BytesEnd::new("hh:borderFills")))?;
    Ok(())
}

fn write_char_properties<W: Write>(
    w: &mut Writer<W>,
    tables: &RefTables,
) -> Result<(), quick_xml::Error> {
    // Collect and sort by ID for deterministic output.
    let mut entries: Vec<(&CharPrKey, u32)> =
        tables.char_pr_ids.iter().map(|(k, &id)| (k, id)).collect();
    entries.sort_by_key(|(_, id)| *id);

    let mut char_properties = BytesStart::new("hh:charProperties");
    char_properties.push_attribute(("itemCnt", entries.len().to_string().as_str()));
    w.write_event(Event::Start(char_properties))?;

    for (key, id) in entries {
        let font_id: u32 = key
            .font_name
            .as_deref()
            .and_then(|name| {
                tables
                    .font_names
                    .iter()
                    .position(|f| f == name)
                    .map(|i| i as u32)
            })
            .unwrap_or(0);

        let color = key.color.as_deref().unwrap_or("#000000");

        let height_str = key.height.to_string();
        let border_fill_ref = tables.border_fill_id.to_string();
        // OWPML schema only allows: id, height, textColor, shadeColor,
        // useFontSpace, useKerning, symMark, borderFillIDRef.
        // bold/italic/underline/strikeout are NOT valid attributes here.
        let mut char_pr = BytesStart::new("hh:charPr");
        char_pr.push_attribute(("id", id.to_string().as_str()));
        char_pr.push_attribute(("height", height_str.as_str()));
        char_pr.push_attribute(("textColor", color));
        char_pr.push_attribute(("borderFillIDRef", border_fill_ref.as_str()));
        // OWPML uses a single `supscript` attribute for both superscript and
        // subscript.  The value is either "superscript" or "subscript".
        if key.superscript {
            char_pr.push_attribute(("supscript", "superscript"));
        } else if key.subscript {
            char_pr.push_attribute(("supscript", "subscript"));
        }
        w.write_event(Event::Start(char_pr))?;

        let font_id_str = font_id.to_string();
        let mut font_ref = BytesStart::new("hh:fontRef");
        font_ref.push_attribute(("hangul", font_id_str.as_str()));
        font_ref.push_attribute(("latin", font_id_str.as_str()));
        font_ref.push_attribute(("hanja", font_id_str.as_str()));
        font_ref.push_attribute(("japanese", font_id_str.as_str()));
        font_ref.push_attribute(("other", font_id_str.as_str()));
        font_ref.push_attribute(("symbol", font_id_str.as_str()));
        font_ref.push_attribute(("user", font_id_str.as_str()));
        w.write_event(Event::Empty(font_ref))?;

        // Required children (minOccurs=1) with default values.
        let mut ratio = BytesStart::new("hh:ratio");
        ratio.push_attribute(("hangul", "100"));
        ratio.push_attribute(("latin", "100"));
        ratio.push_attribute(("hanja", "100"));
        ratio.push_attribute(("japanese", "100"));
        ratio.push_attribute(("other", "100"));
        ratio.push_attribute(("symbol", "100"));
        ratio.push_attribute(("user", "100"));
        w.write_event(Event::Empty(ratio))?;

        let mut spacing = BytesStart::new("hh:spacing");
        spacing.push_attribute(("hangul", "0"));
        spacing.push_attribute(("latin", "0"));
        spacing.push_attribute(("hanja", "0"));
        spacing.push_attribute(("japanese", "0"));
        spacing.push_attribute(("other", "0"));
        spacing.push_attribute(("symbol", "0"));
        spacing.push_attribute(("user", "0"));
        w.write_event(Event::Empty(spacing))?;

        let mut rel_sz = BytesStart::new("hh:relSz");
        rel_sz.push_attribute(("hangul", "100"));
        rel_sz.push_attribute(("latin", "100"));
        rel_sz.push_attribute(("hanja", "100"));
        rel_sz.push_attribute(("japanese", "100"));
        rel_sz.push_attribute(("other", "100"));
        rel_sz.push_attribute(("symbol", "100"));
        rel_sz.push_attribute(("user", "100"));
        w.write_event(Event::Empty(rel_sz))?;

        let mut offset = BytesStart::new("hh:offset");
        offset.push_attribute(("hangul", "0"));
        offset.push_attribute(("latin", "0"));
        offset.push_attribute(("hanja", "0"));
        offset.push_attribute(("japanese", "0"));
        offset.push_attribute(("other", "0"));
        offset.push_attribute(("symbol", "0"));
        offset.push_attribute(("user", "0"));
        w.write_event(Event::Empty(offset))?;

        w.write_event(Event::End(BytesEnd::new("hh:charPr")))?;
    }

    w.write_event(Event::End(BytesEnd::new("hh:charProperties")))?;
    Ok(())
}

/// Left margin value for block-quote paragraphs (HWP units; 800 ≈ 20 mm).
pub(super) const QUOTE_MARGIN_LEFT: &str = "800";

fn write_para_properties<W: Write>(w: &mut Writer<W>) -> Result<(), quick_xml::Error> {
    // Four paraPr entries:
    //   id=0: default (no left indent)
    //   id=1: block-quote (left indent = 800)
    //   id=2: list item depth 0 (left indent = 400)
    //   id=3: list item depth 1+ (left indent = 800)
    let mut para_properties = BytesStart::new("hh:paraProperties");
    para_properties.push_attribute(("itemCnt", "4"));
    w.write_event(Event::Start(para_properties))?;

    // ── paraPr id=0: default (no left indent) ──
    write_single_para_pr(w, "0", "0")?;

    // ── paraPr id=1: block-quote (left indent) ──
    write_single_para_pr(w, "1", QUOTE_MARGIN_LEFT)?;

    // ── paraPr id=2: top-level list item ──
    write_single_para_pr(w, PARA_PR_LIST_D0, LIST_D0_MARGIN_LEFT)?;

    // ── paraPr id=3: nested list item (depth ≥ 1) ──
    write_single_para_pr(w, PARA_PR_LIST_D1, LIST_D1_MARGIN_LEFT)?;

    w.write_event(Event::End(BytesEnd::new("hh:paraProperties")))?;
    Ok(())
}

/// Emit a single `<hh:paraPr>` element with all required OWPML children.
///
/// `left_margin` is the HWP-unit value for the `<hh:left>` child inside
/// `<hh:margin>`.  All other margin children are emitted as `"0"`.
fn write_single_para_pr<W: Write>(
    w: &mut Writer<W>,
    id: &str,
    left_margin: &str,
) -> Result<(), quick_xml::Error> {
    let mut para_pr = BytesStart::new("hh:paraPr");
    para_pr.push_attribute(("id", id));
    w.write_event(Event::Start(para_pr))?;

    // <align> requires both horizontal and vertical attributes.
    let mut align = BytesStart::new("hh:align");
    align.push_attribute(("horizontal", "JUSTIFY"));
    align.push_attribute(("vertical", "BASELINE"));
    w.write_event(Event::Empty(align))?;

    // Required children (minOccurs=1) with default values.
    let mut heading = BytesStart::new("hh:heading");
    heading.push_attribute(("type", "NONE"));
    heading.push_attribute(("idRef", "0"));
    heading.push_attribute(("level", "0"));
    w.write_event(Event::Empty(heading))?;

    // breakSetting requires all seven attributes (schema: all required).
    let mut break_setting = BytesStart::new("hh:breakSetting");
    break_setting.push_attribute(("breakLatinWord", "KEEP_WORD"));
    break_setting.push_attribute(("breakNonLatinWord", "KEEP_WORD"));
    break_setting.push_attribute(("widowOrphan", "false"));
    break_setting.push_attribute(("keepWithNext", "false"));
    break_setting.push_attribute(("keepLines", "false"));
    break_setting.push_attribute(("pageBreakBefore", "false"));
    break_setting.push_attribute(("lineWrap", "BREAK"));
    w.write_event(Event::Empty(break_setting))?;

    let mut line_spacing = BytesStart::new("hh:lineSpacing");
    line_spacing.push_attribute(("type", "PERCENT"));
    line_spacing.push_attribute(("value", "160"));
    w.write_event(Event::Empty(line_spacing))?;

    // margin children are HWPValue elements (value attribute, no attrs on margin itself).
    w.write_event(Event::Start(BytesStart::new("hh:margin")))?;
    for child_name in &["hh:intent", "hh:left", "hh:right", "hh:prev", "hh:next"] {
        let mut child = BytesStart::new(*child_name);
        let value = if *child_name == "hh:left" {
            left_margin
        } else {
            "0"
        };
        child.push_attribute(("value", value));
        w.write_event(Event::Empty(child))?;
    }
    w.write_event(Event::End(BytesEnd::new("hh:margin")))?;

    let mut border = BytesStart::new("hh:border");
    border.push_attribute(("borderFillIDRef", "1"));
    w.write_event(Event::Empty(border))?;

    let mut auto_spacing = BytesStart::new("hh:autoSpacing");
    auto_spacing.push_attribute(("eAsianEng", "false"));
    auto_spacing.push_attribute(("eAsianNum", "false"));
    w.write_event(Event::Empty(auto_spacing))?;

    w.write_event(Event::End(BytesEnd::new("hh:paraPr")))?;
    Ok(())
}

fn write_styles<W: Write>(w: &mut Writer<W>, tables: &RefTables) -> Result<(), quick_xml::Error> {
    // 7 styles: Normal (id=0) + Heading1-6 (id=1..6).
    let mut styles = BytesStart::new("hh:styles");
    styles.push_attribute(("itemCnt", "7"));
    w.write_event(Event::Start(styles))?;

    // Normal style uses the default charPr (id=0).
    let mut normal = BytesStart::new("hh:style");
    normal.push_attribute(("id", "0"));
    normal.push_attribute(("type", "PARA"));
    normal.push_attribute(("name", "Normal"));
    normal.push_attribute(("paraPrIDRef", "0"));
    normal.push_attribute(("charPrIDRef", "0"));
    w.write_event(Event::Empty(normal))?;

    // Heading styles reference level-specific charPr entries.
    for level in 1..=6u8 {
        let char_pr_id = tables.heading_char_pr_id(level);
        let id_str = level.to_string();
        let char_pr_id_str = char_pr_id.to_string();
        let name = format!("Heading{level}");
        let mut style = BytesStart::new("hh:style");
        style.push_attribute(("id", id_str.as_str()));
        style.push_attribute(("type", "PARA"));
        style.push_attribute(("name", name.as_str()));
        style.push_attribute(("paraPrIDRef", "0"));
        style.push_attribute(("charPrIDRef", char_pr_id_str.as_str()));
        w.write_event(Event::Empty(style))?;
    }

    w.write_event(Event::End(BytesEnd::new("hh:styles")))?;
    Ok(())
}

/// Emit the OWPML `<hh:numberings>` element containing the ordered-list
/// numbering definition used by ordered list-item paragraphs in section XML.
///
/// Only one entry is emitted (id=1, `NUM_PR_DIGIT`) for ordered (decimal digit)
/// lists.  Unordered (bullet) lists do **not** reference a `<hh:numbering>`
/// entry — their indentation is carried entirely by the paragraph property.
///
/// The `<hh:paraHead>` child carries the required OWPML attributes and the
/// numbering format string (`%d.`).
fn write_numbering_list<W: Write>(w: &mut Writer<W>) -> Result<(), quick_xml::Error> {
    let mut num_list = BytesStart::new("hh:numberings");
    num_list.push_attribute(("itemCnt", "1"));
    w.write_event(Event::Start(num_list))?;

    // ── id=1: ordered (digit %d.) ────────────────────────────────────────
    let mut digit_num = BytesStart::new("hh:numbering");
    digit_num.push_attribute(("id", NUM_PR_DIGIT));
    digit_num.push_attribute(("start", "1"));
    w.write_event(Event::Start(digit_num))?;

    let mut digit_head = BytesStart::new("hh:paraHead");
    digit_head.push_attribute(("level", "1"));
    digit_head.push_attribute(("align", "LEFT"));
    digit_head.push_attribute(("useInstWidth", "false"));
    digit_head.push_attribute(("autoIndent", "true"));
    digit_head.push_attribute(("widthAdjust", "0"));
    digit_head.push_attribute(("textOffset", "400"));
    digit_head.push_attribute(("numFormat", "DIGIT"));
    digit_head.push_attribute(("charPrIDRef", "0"));
    digit_head.push_attribute(("checkable", "false"));
    digit_head.push_attribute(("start", "1"));
    w.write_event(Event::Start(digit_head))?;
    // Numbering format string: %d. (HWP uses %d as the counter placeholder)
    w.write_event(Event::Text(BytesText::new("%d.")))?;
    w.write_event(Event::End(BytesEnd::new("hh:paraHead")))?;

    w.write_event(Event::End(BytesEnd::new("hh:numbering")))?;

    w.write_event(Event::End(BytesEnd::new("hh:numberings")))?;
    Ok(())
}
