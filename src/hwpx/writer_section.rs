use crate::error::Hwp2MdError;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::io::{Cursor, Write};

use super::{CharPrKey, RefTables};
use crate::ir;

// ---------------------------------------------------------------------------
// section XML
// ---------------------------------------------------------------------------

pub(super) fn generate_section_xml(
    section: &ir::Section,
    _index: usize,
    tables: &RefTables,
) -> Result<String, Hwp2MdError> {
    let mut buf = Cursor::new(Vec::new());
    let mut writer = Writer::new_with_indent(&mut buf, b' ', 2);

    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

    let mut sec = BytesStart::new("hs:sec");
    sec.push_attribute(("xmlns:hs", "http://www.hancom.co.kr/hwpml/2011/section"));
    sec.push_attribute(("xmlns:hp", "http://www.hancom.co.kr/hwpml/2011/paragraph"));
    writer.write_event(Event::Start(sec))?;

    let mut para_id: u32 = 0;
    for block in &section.blocks {
        write_block(&mut writer, block, tables, &mut para_id)?;
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
fn write_block<W: Write>(
    writer: &mut Writer<W>,
    block: &ir::Block,
    tables: &RefTables,
    para_id: &mut u32,
) -> Result<(), quick_xml::Error> {
    match block {
        ir::Block::Heading { level, inlines } => {
            // Style IDs match the hh:styles table: 1=Heading1 ... 6=Heading6.
            // Levels outside 1-6 are clamped to the nearest valid ID.
            let style_id = (*level).clamp(1, 6);
            let style_id_str = style_id.to_string();
            let id_str = para_id.to_string();
            *para_id += 1;
            let mut p = BytesStart::new("hp:p");
            p.push_attribute(("id", id_str.as_str()));
            p.push_attribute(("hp:styleIDRef", style_id_str.as_str()));
            p.push_attribute(("paraPrIDRef", "0"));
            writer.write_event(Event::Start(p))?;
            write_inlines(writer, inlines, tables)?;
            writer.write_event(Event::End(BytesEnd::new("hp:p")))?;
        }
        ir::Block::Paragraph { inlines } => {
            let id_str = para_id.to_string();
            *para_id += 1;
            let mut p = BytesStart::new("hp:p");
            p.push_attribute(("id", id_str.as_str()));
            p.push_attribute(("paraPrIDRef", "0"));
            writer.write_event(Event::Start(p))?;
            write_inlines(writer, inlines, tables)?;
            writer.write_event(Event::End(BytesEnd::new("hp:p")))?;
        }
        ir::Block::Table { rows, .. } => {
            // OWPML requires <hp:tbl> to be a child of <hp:run> inside <hp:p>.
            let row_cnt = rows.len();
            let col_cnt = rows.first().map_or(0, |r| r.cells.len());

            let id_str = para_id.to_string();
            *para_id += 1;
            let mut p = BytesStart::new("hp:p");
            p.push_attribute(("id", id_str.as_str()));
            p.push_attribute(("paraPrIDRef", "0"));
            writer.write_event(Event::Start(p))?;

            let mut run = BytesStart::new("hp:run");
            run.push_attribute(("charPrIDRef", "0"));
            writer.write_event(Event::Start(run))?;

            let mut tbl = BytesStart::new("hp:tbl");
            tbl.push_attribute(("rowCnt", row_cnt.to_string().as_str()));
            tbl.push_attribute(("colCnt", col_cnt.to_string().as_str()));
            writer.write_event(Event::Start(tbl))?;

            for row in rows {
                writer.write_event(Event::Start(BytesStart::new("hp:tr")))?;
                for cell in &row.cells {
                    writer.write_event(Event::Start(BytesStart::new("hp:tc")))?;
                    if cell.colspan > 1 || cell.rowspan > 1 {
                        let mut addr = BytesStart::new("hp:cellAddr");
                        addr.push_attribute(("colSpan", cell.colspan.to_string().as_str()));
                        addr.push_attribute(("rowSpan", cell.rowspan.to_string().as_str()));
                        writer.write_event(Event::Empty(addr))?;
                    }
                    for b in &cell.blocks {
                        write_block(writer, b, tables, para_id)?;
                    }
                    writer.write_event(Event::End(BytesEnd::new("hp:tc")))?;
                }
                writer.write_event(Event::End(BytesEnd::new("hp:tr")))?;
            }

            writer.write_event(Event::End(BytesEnd::new("hp:tbl")))?;
            writer.write_event(Event::End(BytesEnd::new("hp:run")))?;
            writer.write_event(Event::End(BytesEnd::new("hp:p")))?;
        }
        ir::Block::CodeBlock { code, .. } => {
            let code_id = tables.code_block_char_pr_id().to_string();
            let id_str = para_id.to_string();
            *para_id += 1;
            let mut p = BytesStart::new("hp:p");
            p.push_attribute(("id", id_str.as_str()));
            p.push_attribute(("paraPrIDRef", "0"));
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
            for b in blocks {
                write_block(writer, b, tables, para_id)?;
            }
        }
        ir::Block::List { items, .. } => {
            for item in items {
                for b in &item.blocks {
                    write_block(writer, b, tables, para_id)?;
                }
            }
        }
        ir::Block::Image { src, alt } => {
            let id_str = para_id.to_string();
            *para_id += 1;
            let mut p = BytesStart::new("hp:p");
            p.push_attribute(("id", id_str.as_str()));
            p.push_attribute(("paraPrIDRef", "0"));
            writer.write_event(Event::Start(p))?;
            let mut run = BytesStart::new("hp:run");
            run.push_attribute(("charPrIDRef", "0"));
            writer.write_event(Event::Start(run))?;
            writer.write_event(Event::Start(BytesStart::new("hp:pic")))?;
            let mut img = BytesStart::new("hp:img");
            img.push_attribute(("hp:binaryItemIDRef", src.as_str()));
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
            p.push_attribute(("paraPrIDRef", "0"));
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
        ir::Block::Math { tex, .. } => {
            let id_str = para_id.to_string();
            *para_id += 1;
            let mut p = BytesStart::new("hp:p");
            p.push_attribute(("id", id_str.as_str()));
            p.push_attribute(("paraPrIDRef", "0"));
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
        ir::Block::Footnote { content, .. } => {
            for b in content {
                write_block(writer, b, tables, para_id)?;
            }
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

/// Emit a single non-link inline as an `<hp:run>` with text.
///
/// When the inline carries a ruby annotation, the run body uses the OWPML
/// `<hp:ruby>` structure instead of a plain `<hp:t>`.  When a `footnote_ref`
/// is set and the text is empty, a `<hp:noteRef>` element is emitted instead.
///
/// Returns immediately without emitting anything when the inline has no
/// meaningful content (empty text, no ruby, no footnote_ref).
fn write_inline_run<W: Write>(
    writer: &mut Writer<W>,
    inline: &ir::Inline,
    tables: &RefTables,
) -> Result<(), quick_xml::Error> {
    if inline.text.is_empty() && inline.ruby.is_none() && inline.footnote_ref.is_none() {
        return Ok(());
    }

    let key = CharPrKey::from_inline(inline);
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
