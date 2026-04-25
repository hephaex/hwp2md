use crate::error::Hwp2MdError;
use crate::ir;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::io::{Cursor, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

pub fn write_hwpx(
    doc: &ir::Document,
    output: &Path,
    _style: Option<&Path>,
) -> Result<(), Hwp2MdError> {
    let file = std::fs::File::create(output)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip.start_file(
        "mimetype",
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
    )?;
    zip.write_all(b"application/hwpx+zip")?;

    zip.start_file("META-INF/container.xml", options)?;
    zip.write_all(generate_container_xml().as_bytes())?;

    zip.start_file("version.xml", options)?;
    zip.write_all(generate_version_xml().as_bytes())?;

    zip.start_file("Contents/header.xml", options)?;
    zip.write_all(generate_header_xml(doc)?.as_bytes())?;

    zip.start_file("Contents/content.hpf", options)?;
    zip.write_all(generate_content_hpf(doc).as_bytes())?;

    for (i, section) in doc.sections.iter().enumerate() {
        let path = format!("Contents/section{i}.xml");
        zip.start_file(&path, options)?;
        zip.write_all(generate_section_xml(section, i)?.as_bytes())?;
    }

    if doc.sections.is_empty() {
        zip.start_file("Contents/section0.xml", options)?;
        let empty_section = ir::Section { blocks: Vec::new() };
        zip.write_all(generate_section_xml(&empty_section, 0)?.as_bytes())?;
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

fn generate_container_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0">
  <rootfiles>
    <rootfile full-path="Contents/content.hpf" media-type="application/hwpx+xml"/>
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

fn generate_header_xml(doc: &ir::Document) -> Result<String, Hwp2MdError> {
    let title = doc.metadata.title.as_deref().unwrap_or("");
    let author = doc.metadata.author.as_deref().unwrap_or("");

    let mut buf = Cursor::new(Vec::new());
    let mut writer = Writer::new_with_indent(&mut buf, b' ', 2);

    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

    let mut head = BytesStart::new("head");
    head.push_attribute(("xmlns", "http://www.hancom.co.kr/hwpml/2011/head"));
    writer.write_event(Event::Start(head))?;

    writer.write_event(Event::Start(BytesStart::new("title")))?;
    writer.write_event(Event::Text(BytesText::new(title)))?;
    writer.write_event(Event::End(BytesEnd::new("title")))?;

    writer.write_event(Event::Start(BytesStart::new("creator")))?;
    writer.write_event(Event::Text(BytesText::new(author)))?;
    writer.write_event(Event::End(BytesEnd::new("creator")))?;

    writer.write_event(Event::End(BytesEnd::new("head")))?;

    Ok(String::from_utf8(buf.into_inner()).unwrap_or_default())
}

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

fn generate_section_xml(section: &ir::Section, _index: usize) -> Result<String, Hwp2MdError> {
    let mut buf = Cursor::new(Vec::new());
    let mut writer = Writer::new_with_indent(&mut buf, b' ', 2);

    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

    let mut sec = BytesStart::new("hs:sec");
    sec.push_attribute(("xmlns:hs", "http://www.hancom.co.kr/hwpml/2011/section"));
    sec.push_attribute(("xmlns:hp", "http://www.hancom.co.kr/hwpml/2011/paragraph"));
    writer.write_event(Event::Start(sec))?;

    for block in &section.blocks {
        write_block(&mut writer, block)?;
    }

    writer.write_event(Event::End(BytesEnd::new("hs:sec")))?;

    Ok(String::from_utf8(buf.into_inner()).unwrap_or_default())
}

fn write_block<W: Write>(
    writer: &mut Writer<W>,
    block: &ir::Block,
) -> Result<(), quick_xml::Error> {
    match block {
        ir::Block::Heading { level, inlines } => {
            let mut p = BytesStart::new("hp:p");
            p.push_attribute(("hp:styleIDRef", format!("Heading{level}").as_str()));
            writer.write_event(Event::Start(p))?;
            write_inlines(writer, inlines)?;
            writer.write_event(Event::End(BytesEnd::new("hp:p")))?;
        }
        ir::Block::Paragraph { inlines } => {
            writer.write_event(Event::Start(BytesStart::new("hp:p")))?;
            write_inlines(writer, inlines)?;
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
                        write_block(writer, b)?;
                    }
                    writer.write_event(Event::End(BytesEnd::new("hp:tc")))?;
                }
                writer.write_event(Event::End(BytesEnd::new("hp:tr")))?;
            }
            writer.write_event(Event::End(BytesEnd::new("hp:tbl")))?;
        }
        ir::Block::CodeBlock { code, .. } => {
            writer.write_event(Event::Start(BytesStart::new("hp:p")))?;
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
                write_block(writer, b)?;
            }
        }
        ir::Block::List { items, .. } => {
            for item in items {
                for b in &item.blocks {
                    write_block(writer, b)?;
                }
            }
        }
        ir::Block::Image { src, alt } => {
            writer.write_event(Event::Start(BytesStart::new("hp:p")))?;
            let mut img = BytesStart::new("hp:img");
            img.push_attribute(("hp:binaryItemIDRef", src.as_str()));
            img.push_attribute(("alt", alt.as_str()));
            writer.write_event(Event::Empty(img))?;
            writer.write_event(Event::End(BytesEnd::new("hp:p")))?;
        }
        ir::Block::HorizontalRule => {
            writer.write_event(Event::Start(BytesStart::new("hp:p")))?;
            writer.write_event(Event::Text(BytesText::new("───────────────────")))?;
            writer.write_event(Event::End(BytesEnd::new("hp:p")))?;
        }
        ir::Block::Math { tex, .. } => {
            writer.write_event(Event::Start(BytesStart::new("hp:p")))?;
            writer.write_event(Event::Start(BytesStart::new("hp:equation")))?;
            writer.write_event(Event::Text(BytesText::new(tex)))?;
            writer.write_event(Event::End(BytesEnd::new("hp:equation")))?;
            writer.write_event(Event::End(BytesEnd::new("hp:p")))?;
        }
        ir::Block::Footnote { content, .. } => {
            for b in content {
                write_block(writer, b)?;
            }
        }
    }
    Ok(())
}

fn write_inlines<W: Write>(
    writer: &mut Writer<W>,
    inlines: &[ir::Inline],
) -> Result<(), quick_xml::Error> {
    for inline in inlines {
        // Per OWPML schema, hp:run carries no formatting attributes itself.
        // Character properties (bold, italic, etc.) belong on the hp:charPr
        // child element, which must appear before hp:t.
        // Structure: <hp:run> <hp:charPr bold="true" …/> <hp:t>…</hp:t> </hp:run>
        let run = BytesStart::new("hp:run");
        let mut attrs = Vec::new();
        if inline.bold {
            attrs.push(("bold", "true"));
        }
        if inline.italic {
            attrs.push(("italic", "true"));
        }
        if inline.underline {
            attrs.push(("underline", "bottom"));
        }
        if inline.strikethrough {
            attrs.push(("strikeout", "line"));
        }

        writer.write_event(Event::Start(run))?;

        if !attrs.is_empty() {
            let mut char_pr = BytesStart::new("hp:charPr");
            for (k, v) in &attrs {
                char_pr.push_attribute((*k, *v));
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
