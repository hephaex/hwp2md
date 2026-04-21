use crate::ir;
use anyhow::Result;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::io::{Cursor, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

pub fn write_hwpx(doc: &ir::Document, output: &Path, _style: Option<&Path>) -> Result<()> {
    let file = std::fs::File::create(output)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("mimetype", SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored))?;
    zip.write_all(b"application/hwpx+zip")?;

    zip.start_file("META-INF/container.xml", options)?;
    zip.write_all(generate_container_xml().as_bytes())?;

    zip.start_file("version.xml", options)?;
    zip.write_all(generate_version_xml().as_bytes())?;

    zip.start_file("Contents/header.xml", options)?;
    zip.write_all(generate_header_xml(doc).as_bytes())?;

    zip.start_file("Contents/content.hpf", options)?;
    zip.write_all(generate_content_hpf(doc).as_bytes())?;

    for (i, section) in doc.sections.iter().enumerate() {
        let path = format!("Contents/section{i}.xml");
        zip.start_file(&path, options)?;
        zip.write_all(generate_section_xml(section, i).as_bytes())?;
    }

    if doc.sections.is_empty() {
        zip.start_file("Contents/section0.xml", options)?;
        let empty_section = ir::Section { blocks: Vec::new() };
        zip.write_all(generate_section_xml(&empty_section, 0).as_bytes())?;
    }

    for asset in &doc.assets {
        let path = format!("BinData/{}", asset.name);
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

fn generate_header_xml(doc: &ir::Document) -> String {
    let title = doc.metadata.title.as_deref().unwrap_or("");
    let author = doc.metadata.author.as_deref().unwrap_or("");

    let mut buf = Cursor::new(Vec::new());
    let mut writer = Writer::new_with_indent(&mut buf, b' ', 2);

    let _ = writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)));

    let mut head = BytesStart::new("head");
    head.push_attribute(("xmlns", "http://www.hancom.co.kr/hwpml/2011/head"));
    let _ = writer.write_event(Event::Start(head));

    let _ = writer.write_event(Event::Start(BytesStart::new("title")));
    let _ = writer.write_event(Event::Text(BytesText::new(title)));
    let _ = writer.write_event(Event::End(BytesEnd::new("title")));

    let _ = writer.write_event(Event::Start(BytesStart::new("creator")));
    let _ = writer.write_event(Event::Text(BytesText::new(author)));
    let _ = writer.write_event(Event::End(BytesEnd::new("creator")));

    let _ = writer.write_event(Event::End(BytesEnd::new("head")));

    String::from_utf8(buf.into_inner()).unwrap_or_default()
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

fn generate_section_xml(section: &ir::Section, _index: usize) -> String {
    let mut buf = Cursor::new(Vec::new());
    let mut writer = Writer::new_with_indent(&mut buf, b' ', 2);

    let _ = writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)));

    let mut sec = BytesStart::new("hs:sec");
    sec.push_attribute(("xmlns:hs", "http://www.hancom.co.kr/hwpml/2011/section"));
    sec.push_attribute(("xmlns:hp", "http://www.hancom.co.kr/hwpml/2011/paragraph"));
    let _ = writer.write_event(Event::Start(sec));

    for block in &section.blocks {
        write_block(&mut writer, block);
    }

    let _ = writer.write_event(Event::End(BytesEnd::new("hs:sec")));

    String::from_utf8(buf.into_inner()).unwrap_or_default()
}

fn write_block<W: Write>(writer: &mut Writer<W>, block: &ir::Block) {
    match block {
        ir::Block::Heading { level, inlines } => {
            let mut p = BytesStart::new("hp:p");
            p.push_attribute(("hp:styleIDRef", format!("Heading{level}").as_str()));
            let _ = writer.write_event(Event::Start(p));
            write_inlines(writer, inlines);
            let _ = writer.write_event(Event::End(BytesEnd::new("hp:p")));
        }
        ir::Block::Paragraph { inlines } => {
            let _ = writer.write_event(Event::Start(BytesStart::new("hp:p")));
            write_inlines(writer, inlines);
            let _ = writer.write_event(Event::End(BytesEnd::new("hp:p")));
        }
        ir::Block::Table { rows, .. } => {
            let _ = writer.write_event(Event::Start(BytesStart::new("hp:tbl")));
            for row in rows {
                let _ = writer.write_event(Event::Start(BytesStart::new("hp:tr")));
                for cell in &row.cells {
                    let _ = writer.write_event(Event::Start(BytesStart::new("hp:tc")));
                    for b in &cell.blocks {
                        write_block(writer, b);
                    }
                    let _ = writer.write_event(Event::End(BytesEnd::new("hp:tc")));
                }
                let _ = writer.write_event(Event::End(BytesEnd::new("hp:tr")));
            }
            let _ = writer.write_event(Event::End(BytesEnd::new("hp:tbl")));
        }
        ir::Block::CodeBlock { code, .. } => {
            let _ = writer.write_event(Event::Start(BytesStart::new("hp:p")));
            let mut run = BytesStart::new("hp:run");
            run.push_attribute(("hp:charPrIDRef", "code"));
            let _ = writer.write_event(Event::Start(run));
            let _ = writer.write_event(Event::Start(BytesStart::new("hp:t")));
            let _ = writer.write_event(Event::Text(BytesText::new(code)));
            let _ = writer.write_event(Event::End(BytesEnd::new("hp:t")));
            let _ = writer.write_event(Event::End(BytesEnd::new("hp:run")));
            let _ = writer.write_event(Event::End(BytesEnd::new("hp:p")));
        }
        ir::Block::BlockQuote { blocks } => {
            for b in blocks {
                write_block(writer, b);
            }
        }
        ir::Block::List { items, .. } => {
            for item in items {
                for b in &item.blocks {
                    write_block(writer, b);
                }
            }
        }
        ir::Block::Image { src, alt } => {
            let _ = writer.write_event(Event::Start(BytesStart::new("hp:p")));
            let mut img = BytesStart::new("hp:img");
            img.push_attribute(("hp:binaryItemIDRef", src.as_str()));
            img.push_attribute(("alt", alt.as_str()));
            let _ = writer.write_event(Event::Empty(img));
            let _ = writer.write_event(Event::End(BytesEnd::new("hp:p")));
        }
        ir::Block::HorizontalRule => {
            let _ = writer.write_event(Event::Start(BytesStart::new("hp:p")));
            let _ = writer.write_event(Event::Text(BytesText::new("───────────────────")));
            let _ = writer.write_event(Event::End(BytesEnd::new("hp:p")));
        }
        ir::Block::Math { tex, .. } => {
            let _ = writer.write_event(Event::Start(BytesStart::new("hp:p")));
            let _ = writer.write_event(Event::Start(BytesStart::new("hp:equation")));
            let _ = writer.write_event(Event::Text(BytesText::new(tex)));
            let _ = writer.write_event(Event::End(BytesEnd::new("hp:equation")));
            let _ = writer.write_event(Event::End(BytesEnd::new("hp:p")));
        }
        ir::Block::Footnote { content, .. } => {
            for b in content {
                write_block(writer, b);
            }
        }
    }
}

fn write_inlines<W: Write>(writer: &mut Writer<W>, inlines: &[ir::Inline]) {
    for inline in inlines {
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

        let _ = writer.write_event(Event::Start(run));

        if !attrs.is_empty() {
            let mut char_pr = BytesStart::new("hp:charPr");
            for (k, v) in &attrs {
                char_pr.push_attribute((*k, *v));
            }
            let _ = writer.write_event(Event::Empty(char_pr));
        }

        let _ = writer.write_event(Event::Start(BytesStart::new("hp:t")));
        let _ = writer.write_event(Event::Text(BytesText::new(&inline.text)));
        let _ = writer.write_event(Event::End(BytesEnd::new("hp:t")));

        let _ = writer.write_event(Event::End(BytesEnd::new("hp:run")));
    }
}
