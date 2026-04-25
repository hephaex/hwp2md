use crate::error::Hwp2MdError;
use crate::ir;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::collections::HashMap;
use std::io::{Cursor, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

// ---------------------------------------------------------------------------
// Font / character-property tables
// ---------------------------------------------------------------------------

const DEFAULT_FONT: &str = "바탕";
const LANG_SLOTS: [&str; 7] = [
    "HANGUL", "LATIN", "HANJA", "JAPANESE", "OTHER", "SYMBOL", "USER",
];

/// A unique character-formatting combination extracted from the document IR.
///
/// `id=0` is always the plain/default entry (all fields false, no color, no
/// custom font).  Additional entries are assigned IDs starting at 1.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CharPrKey {
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
    color: Option<String>,
    font_name: Option<String>,
}

impl CharPrKey {
    fn plain() -> Self {
        Self {
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            color: None,
            font_name: None,
        }
    }

    fn from_inline(inline: &ir::Inline) -> Self {
        Self {
            bold: inline.bold,
            italic: inline.italic,
            underline: inline.underline,
            strikethrough: inline.strikethrough,
            color: inline.color.clone(),
            font_name: inline.font_name.clone(),
        }
    }

    fn is_plain(&self) -> bool {
        !self.bold
            && !self.italic
            && !self.underline
            && !self.strikethrough
            && self.color.is_none()
            && self.font_name.is_none()
    }
}

/// Collected reference tables built from a complete document scan.
struct RefTables {
    /// Maps `CharPrKey` → sequential ID (0 = default/plain).
    char_pr_ids: HashMap<CharPrKey, u32>,
    /// Unique font names in document order (first seen wins for ordering).
    font_names: Vec<String>,
}

impl RefTables {
    fn build(doc: &ir::Document) -> Self {
        let mut char_pr_ids: HashMap<CharPrKey, u32> = HashMap::new();
        let mut font_names: Vec<String> = Vec::new();
        let mut font_set: std::collections::HashSet<String> = std::collections::HashSet::new();

        // id=0 is always the plain entry.
        char_pr_ids.insert(CharPrKey::plain(), 0);

        // Collect default font as id=0 font.
        font_set.insert(DEFAULT_FONT.to_owned());
        font_names.push(DEFAULT_FONT.to_owned());

        let mut next_id: u32 = 1;

        for section in &doc.sections {
            collect_from_blocks(
                &section.blocks,
                &mut char_pr_ids,
                &mut next_id,
                &mut font_names,
                &mut font_set,
            );
        }

        Self {
            char_pr_ids,
            font_names,
        }
    }

    fn char_pr_id(&self, key: &CharPrKey) -> u32 {
        *self.char_pr_ids.get(key).unwrap_or(&0)
    }
}

fn collect_from_blocks(
    blocks: &[ir::Block],
    char_pr_ids: &mut HashMap<CharPrKey, u32>,
    next_id: &mut u32,
    font_names: &mut Vec<String>,
    font_set: &mut std::collections::HashSet<String>,
) {
    for block in blocks {
        match block {
            ir::Block::Heading { inlines, .. } | ir::Block::Paragraph { inlines } => {
                collect_from_inlines(inlines, char_pr_ids, next_id, font_names, font_set);
            }
            ir::Block::Table { rows, .. } => {
                for row in rows {
                    for cell in &row.cells {
                        collect_from_blocks(
                            &cell.blocks,
                            char_pr_ids,
                            next_id,
                            font_names,
                            font_set,
                        );
                    }
                }
            }
            ir::Block::BlockQuote { blocks }
            | ir::Block::Footnote {
                content: blocks, ..
            } => {
                collect_from_blocks(blocks, char_pr_ids, next_id, font_names, font_set);
            }
            ir::Block::List { items, .. } => {
                for item in items {
                    collect_from_blocks(&item.blocks, char_pr_ids, next_id, font_names, font_set);
                }
            }
            ir::Block::CodeBlock { .. }
            | ir::Block::Image { .. }
            | ir::Block::HorizontalRule
            | ir::Block::Math { .. } => {}
        }
    }
}

fn collect_from_inlines(
    inlines: &[ir::Inline],
    char_pr_ids: &mut HashMap<CharPrKey, u32>,
    next_id: &mut u32,
    font_names: &mut Vec<String>,
    font_set: &mut std::collections::HashSet<String>,
) {
    for inline in inlines {
        let key = CharPrKey::from_inline(inline);

        if let Some(font) = &inline.font_name {
            if font_set.insert(font.clone()) {
                font_names.push(font.clone());
            }
        }

        if let std::collections::hash_map::Entry::Vacant(e) = char_pr_ids.entry(key) {
            e.insert(*next_id);
            *next_id += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn write_hwpx(
    doc: &ir::Document,
    output: &Path,
    _style: Option<&Path>,
) -> Result<(), Hwp2MdError> {
    let tables = RefTables::build(doc);

    let file = std::fs::File::create(output)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip.start_file(
        "mimetype",
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
    )?;
    zip.write_all(b"application/hwp+zip")?;

    zip.start_file("META-INF/container.xml", options)?;
    zip.write_all(generate_container_xml().as_bytes())?;

    zip.start_file("version.xml", options)?;
    zip.write_all(generate_version_xml().as_bytes())?;

    zip.start_file("Contents/header.xml", options)?;
    zip.write_all(generate_header_xml(doc, &tables)?.as_bytes())?;

    zip.start_file("Contents/content.hpf", options)?;
    zip.write_all(generate_content_hpf(doc).as_bytes())?;

    for (i, section) in doc.sections.iter().enumerate() {
        let path = format!("Contents/section{i}.xml");
        zip.start_file(&path, options)?;
        zip.write_all(generate_section_xml(section, i, &tables)?.as_bytes())?;
    }

    if doc.sections.is_empty() {
        zip.start_file("Contents/section0.xml", options)?;
        let empty_section = ir::Section { blocks: Vec::new() };
        zip.write_all(generate_section_xml(&empty_section, 0, &tables)?.as_bytes())?;
    }

    for (i, asset) in doc.assets.iter().enumerate() {
        let safe_name = std::path::Path::new(&asset.name)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("asset_{i}"));
        let path = format!("BinData/{safe_name}");
        zip.start_file(&path, options)?;
        zip.write_all(&asset.data)?;
    }

    zip.finish()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Static file generators
// ---------------------------------------------------------------------------

fn generate_container_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0">
  <rootfiles>
    <rootfile full-path="Contents/content.hpf" media-type="application/hwp+xml"/>
  </rootfiles>
</container>"#
        .to_string()
}

fn generate_version_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<hh:HWPCompatibleDocument xmlns:hh="http://www.hancom.co.kr/hwpml/2011/head" version="1.1">
  <hh:DocInfo>
    <hh:HWPVersion Major="5" Minor="1" Micro="0" Build="0"/>
  </hh:DocInfo>
</hh:HWPCompatibleDocument>"#
        .to_string()
}

// ---------------------------------------------------------------------------
// header.xml — OWPML reference tables
// ---------------------------------------------------------------------------

fn generate_header_xml(doc: &ir::Document, tables: &RefTables) -> Result<String, Hwp2MdError> {
    let title = doc.metadata.title.as_deref().unwrap_or("");
    let author = doc.metadata.author.as_deref().unwrap_or("");

    let mut buf = Cursor::new(Vec::new());
    let mut w = Writer::new_with_indent(&mut buf, b' ', 2);

    w.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

    // <hh:head>
    let mut head = BytesStart::new("hh:head");
    head.push_attribute(("xmlns:hh", "http://www.hancom.co.kr/hwpml/2011/head"));
    w.write_event(Event::Start(head))?;

    // <hh:docInfo>
    w.write_event(Event::Start(BytesStart::new("hh:docInfo")))?;

    w.write_event(Event::Start(BytesStart::new("hh:title")))?;
    w.write_event(Event::Text(BytesText::new(title)))?;
    w.write_event(Event::End(BytesEnd::new("hh:title")))?;

    w.write_event(Event::Start(BytesStart::new("hh:creator")))?;
    w.write_event(Event::Text(BytesText::new(author)))?;
    w.write_event(Event::End(BytesEnd::new("hh:creator")))?;

    w.write_event(Event::End(BytesEnd::new("hh:docInfo")))?;

    // <hh:beginNum> — required structural marker
    let mut begin_num = BytesStart::new("hh:beginNum");
    begin_num.push_attribute(("page", "1"));
    begin_num.push_attribute(("footnote", "1"));
    begin_num.push_attribute(("endnote", "1"));
    begin_num.push_attribute(("pic", "1"));
    begin_num.push_attribute(("tbl", "1"));
    begin_num.push_attribute(("equation", "1"));
    w.write_event(Event::Empty(begin_num))?;

    // <hh:refList>
    w.write_event(Event::Start(BytesStart::new("hh:refList")))?;

    write_font_faces(&mut w, tables)?;
    write_char_properties(&mut w, tables)?;
    write_para_properties(&mut w)?;

    w.write_event(Event::End(BytesEnd::new("hh:refList")))?;

    w.write_event(Event::End(BytesEnd::new("hh:head")))?;

    Ok(String::from_utf8(buf.into_inner()).unwrap_or_default())
}

fn write_font_faces<W: Write>(
    w: &mut Writer<W>,
    tables: &RefTables,
) -> Result<(), quick_xml::Error> {
    w.write_event(Event::Start(BytesStart::new("hh:fontfaces")))?;

    for lang in LANG_SLOTS {
        let mut face = BytesStart::new("hh:fontface");
        face.push_attribute(("lang", lang));
        w.write_event(Event::Start(face))?;

        for (id, name) in tables.font_names.iter().enumerate() {
            let mut font = BytesStart::new("hh:font");
            font.push_attribute(("id", id.to_string().as_str()));
            font.push_attribute(("face", name.as_str()));
            w.write_event(Event::Empty(font))?;
        }

        w.write_event(Event::End(BytesEnd::new("hh:fontface")))?;
    }

    w.write_event(Event::End(BytesEnd::new("hh:fontfaces")))?;
    Ok(())
}

fn write_char_properties<W: Write>(
    w: &mut Writer<W>,
    tables: &RefTables,
) -> Result<(), quick_xml::Error> {
    w.write_event(Event::Start(BytesStart::new("hh:charProperties")))?;

    // Collect and sort by ID for deterministic output.
    let mut entries: Vec<(&CharPrKey, u32)> =
        tables.char_pr_ids.iter().map(|(k, &id)| (k, id)).collect();
    entries.sort_by_key(|(_, id)| *id);

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

        let mut char_pr = BytesStart::new("hh:charPr");
        char_pr.push_attribute(("id", id.to_string().as_str()));
        char_pr.push_attribute(("height", "1000"));
        char_pr.push_attribute(("textColor", color));
        if key.bold {
            char_pr.push_attribute(("bold", "true"));
        }
        if key.italic {
            char_pr.push_attribute(("italic", "true"));
        }
        if key.underline {
            char_pr.push_attribute(("underline", "bottom"));
        }
        if key.strikethrough {
            char_pr.push_attribute(("strikeout", "line"));
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

        w.write_event(Event::End(BytesEnd::new("hh:charPr")))?;
    }

    w.write_event(Event::End(BytesEnd::new("hh:charProperties")))?;
    Ok(())
}

fn write_para_properties<W: Write>(w: &mut Writer<W>) -> Result<(), quick_xml::Error> {
    w.write_event(Event::Start(BytesStart::new("hh:paraProperties")))?;

    let mut para_pr = BytesStart::new("hh:paraPr");
    para_pr.push_attribute(("id", "0"));
    w.write_event(Event::Start(para_pr))?;

    let mut align = BytesStart::new("hh:align");
    align.push_attribute(("horizontal", "JUSTIFY"));
    w.write_event(Event::Empty(align))?;

    let mut line_spacing = BytesStart::new("hh:lineSpacing");
    line_spacing.push_attribute(("type", "PERCENT"));
    line_spacing.push_attribute(("value", "160"));
    w.write_event(Event::Empty(line_spacing))?;

    w.write_event(Event::End(BytesEnd::new("hh:paraPr")))?;

    w.write_event(Event::End(BytesEnd::new("hh:paraProperties")))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// content.hpf
// ---------------------------------------------------------------------------

fn generate_content_hpf(doc: &ir::Document) -> String {
    let section_count = doc.sections.len().max(1);
    let mut items = String::new();
    for i in 0..section_count {
        items.push_str(&format!(
            "    <hp:item href=\"section{i}.xml\" type=\"Section\"/>\n"
        ));
    }

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<hp:HWPMLPackage xmlns:hp="http://www.hancom.co.kr/hwpml/2011/packageInfo">
  <hp:compatibledocument version="1.1"/>
  <hp:contents>
{items}  </hp:contents>
</hp:HWPMLPackage>"#
    )
}

// ---------------------------------------------------------------------------
// section XML
// ---------------------------------------------------------------------------

fn generate_section_xml(
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

    for block in &section.blocks {
        write_block(&mut writer, block, tables)?;
    }

    writer.write_event(Event::End(BytesEnd::new("hs:sec")))?;

    Ok(String::from_utf8(buf.into_inner()).unwrap_or_default())
}

fn write_block<W: Write>(
    writer: &mut Writer<W>,
    block: &ir::Block,
    tables: &RefTables,
) -> Result<(), quick_xml::Error> {
    match block {
        ir::Block::Heading { level, inlines } => {
            let mut p = BytesStart::new("hp:p");
            p.push_attribute(("hp:styleIDRef", format!("Heading{level}").as_str()));
            p.push_attribute(("paraPrIDRef", "0"));
            writer.write_event(Event::Start(p))?;
            write_inlines(writer, inlines, tables)?;
            writer.write_event(Event::End(BytesEnd::new("hp:p")))?;
        }
        ir::Block::Paragraph { inlines } => {
            let mut p = BytesStart::new("hp:p");
            p.push_attribute(("paraPrIDRef", "0"));
            writer.write_event(Event::Start(p))?;
            write_inlines(writer, inlines, tables)?;
            writer.write_event(Event::End(BytesEnd::new("hp:p")))?;
        }
        ir::Block::Table { rows, .. } => {
            writer.write_event(Event::Start(BytesStart::new("hp:tbl")))?;
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
                        write_block(writer, b, tables)?;
                    }
                    writer.write_event(Event::End(BytesEnd::new("hp:tc")))?;
                }
                writer.write_event(Event::End(BytesEnd::new("hp:tr")))?;
            }
            writer.write_event(Event::End(BytesEnd::new("hp:tbl")))?;
        }
        ir::Block::CodeBlock { code, .. } => {
            let mut p = BytesStart::new("hp:p");
            p.push_attribute(("paraPrIDRef", "0"));
            writer.write_event(Event::Start(p))?;
            let mut run = BytesStart::new("hp:run");
            run.push_attribute(("hp:charPrIDRef", "code"));
            writer.write_event(Event::Start(run))?;
            writer.write_event(Event::Start(BytesStart::new("hp:t")))?;
            writer.write_event(Event::Text(BytesText::new(code)))?;
            writer.write_event(Event::End(BytesEnd::new("hp:t")))?;
            writer.write_event(Event::End(BytesEnd::new("hp:run")))?;
            writer.write_event(Event::End(BytesEnd::new("hp:p")))?;
        }
        ir::Block::BlockQuote { blocks } => {
            for b in blocks {
                write_block(writer, b, tables)?;
            }
        }
        ir::Block::List { items, .. } => {
            for item in items {
                for b in &item.blocks {
                    write_block(writer, b, tables)?;
                }
            }
        }
        ir::Block::Image { src, alt } => {
            let mut p = BytesStart::new("hp:p");
            p.push_attribute(("paraPrIDRef", "0"));
            writer.write_event(Event::Start(p))?;
            let mut img = BytesStart::new("hp:img");
            img.push_attribute(("hp:binaryItemIDRef", src.as_str()));
            img.push_attribute(("alt", alt.as_str()));
            writer.write_event(Event::Empty(img))?;
            writer.write_event(Event::End(BytesEnd::new("hp:p")))?;
        }
        ir::Block::HorizontalRule => {
            let mut p = BytesStart::new("hp:p");
            p.push_attribute(("paraPrIDRef", "0"));
            writer.write_event(Event::Start(p))?;
            writer.write_event(Event::Text(BytesText::new("───────────────────")))?;
            writer.write_event(Event::End(BytesEnd::new("hp:p")))?;
        }
        ir::Block::Math { tex, .. } => {
            let mut p = BytesStart::new("hp:p");
            p.push_attribute(("paraPrIDRef", "0"));
            writer.write_event(Event::Start(p))?;
            writer.write_event(Event::Start(BytesStart::new("hp:equation")))?;
            writer.write_event(Event::Text(BytesText::new(tex)))?;
            writer.write_event(Event::End(BytesEnd::new("hp:equation")))?;
            writer.write_event(Event::End(BytesEnd::new("hp:p")))?;
        }
        ir::Block::Footnote { content, .. } => {
            for b in content {
                write_block(writer, b, tables)?;
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
    for inline in inlines {
        let key = CharPrKey::from_inline(inline);
        let char_pr_id = tables.char_pr_id(&key);

        let mut run = BytesStart::new("hp:run");
        run.push_attribute(("charPrIDRef", char_pr_id.to_string().as_str()));
        writer.write_event(Event::Start(run))?;

        // Emit inline hp:charPr only for non-plain runs so that readers
        // that do not consult the header table still get the formatting hint.
        if !key.is_plain() {
            let mut char_pr = BytesStart::new("hp:charPr");
            if key.bold {
                char_pr.push_attribute(("bold", "true"));
            }
            if key.italic {
                char_pr.push_attribute(("italic", "true"));
            }
            if key.underline {
                char_pr.push_attribute(("underline", "bottom"));
            }
            if key.strikethrough {
                char_pr.push_attribute(("strikeout", "line"));
            }
            writer.write_event(Event::Empty(char_pr))?;
        }

        writer.write_event(Event::Start(BytesStart::new("hp:t")))?;
        writer.write_event(Event::Text(BytesText::new(&inline.text)))?;
        writer.write_event(Event::End(BytesEnd::new("hp:t")))?;

        writer.write_event(Event::End(BytesEnd::new("hp:run")))?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "writer_tests.rs"]
mod tests;
