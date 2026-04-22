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

fn generate_header_xml(doc: &ir::Document) -> Result<String> {
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

fn generate_section_xml(section: &ir::Section, _index: usize) -> Result<String> {
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
mod tests {
    use super::*;
    use crate::ir::{
        Asset, Block, Document, Inline, ListItem, Metadata, Section, TableCell, TableRow,
    };
    use std::io::Read as _;

    // ── helpers ──────────────────────────────────────────────────────────────

    fn inline(text: &str) -> Inline {
        Inline::plain(text)
    }

    fn bold_inline(text: &str) -> Inline {
        Inline {
            text: text.into(),
            bold: true,
            ..Inline::default()
        }
    }

    fn italic_inline(text: &str) -> Inline {
        Inline {
            text: text.into(),
            italic: true,
            ..Inline::default()
        }
    }

    fn underline_inline(text: &str) -> Inline {
        Inline {
            text: text.into(),
            underline: true,
            ..Inline::default()
        }
    }

    fn section_xml(blocks: Vec<Block>) -> String {
        let sec = Section { blocks };
        generate_section_xml(&sec, 0).expect("generate_section_xml failed")
    }

    fn zip_entry_names(path: &std::path::Path) -> Vec<String> {
        let file = std::fs::File::open(path).expect("open zip");
        let mut archive = zip::ZipArchive::new(file).expect("parse zip");
        (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_owned())
            .collect()
    }

    fn doc_with_section(blocks: Vec<Block>) -> Document {
        Document {
            metadata: Metadata::default(),
            sections: vec![Section { blocks }],
            assets: Vec::new(),
        }
    }

    // ── generate_section_xml: structural tests ────────────────────────────

    #[test]
    fn section_xml_empty_section_produces_valid_wrapper() {
        let xml = section_xml(vec![]);
        assert!(xml.contains("<hs:sec"), "root element missing: {xml}");
        assert!(xml.contains("</hs:sec>"), "closing tag missing: {xml}");
        assert!(xml.contains(r#"xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section""#));
    }

    #[test]
    fn section_xml_paragraph_plain_text() {
        let xml = section_xml(vec![Block::Paragraph {
            inlines: vec![inline("hello world")],
        }]);
        assert!(xml.contains("<hp:p>"), "paragraph open: {xml}");
        assert!(xml.contains("</hp:p>"), "paragraph close: {xml}");
        assert!(xml.contains("<hp:t>"), "text run open: {xml}");
        assert!(xml.contains("hello world"), "text content: {xml}");
    }

    #[test]
    fn section_xml_empty_paragraph() {
        let xml = section_xml(vec![Block::Paragraph { inlines: vec![] }]);
        assert!(xml.contains("<hp:p>"));
        assert!(xml.contains("</hp:p>"));
    }

    #[test]
    fn section_xml_heading_level_1() {
        let xml = section_xml(vec![Block::Heading {
            level: 1,
            inlines: vec![inline("Title")],
        }]);
        assert!(
            xml.contains(r#"hp:styleIDRef="Heading1""#),
            "h1 style ref: {xml}"
        );
        assert!(xml.contains("Title"));
    }

    #[test]
    fn section_xml_heading_level_6() {
        let xml = section_xml(vec![Block::Heading {
            level: 6,
            inlines: vec![inline("Deep")],
        }]);
        assert!(xml.contains(r#"hp:styleIDRef="Heading6""#), "{xml}");
        assert!(xml.contains("Deep"));
    }

    #[test]
    fn section_xml_bold_inline_emits_charpr() {
        let xml = section_xml(vec![Block::Paragraph {
            inlines: vec![bold_inline("strong")],
        }]);
        assert!(xml.contains("<hp:charPr"), "charPr element: {xml}");
        assert!(xml.contains(r#"bold="true""#), "bold attr: {xml}");
        assert!(xml.contains("strong"));
    }

    #[test]
    fn section_xml_italic_inline_emits_charpr() {
        let xml = section_xml(vec![Block::Paragraph {
            inlines: vec![italic_inline("em")],
        }]);
        assert!(xml.contains(r#"italic="true""#), "{xml}");
    }

    #[test]
    fn section_xml_underline_inline_emits_charpr() {
        let xml = section_xml(vec![Block::Paragraph {
            inlines: vec![underline_inline("ul")],
        }]);
        assert!(xml.contains(r#"underline="bottom""#), "{xml}");
    }

    #[test]
    fn section_xml_strikethrough_inline_emits_charpr() {
        let xml = section_xml(vec![Block::Paragraph {
            inlines: vec![Inline {
                text: "del".into(),
                strikethrough: true,
                ..Inline::default()
            }],
        }]);
        assert!(xml.contains(r#"strikeout="line""#), "{xml}");
    }

    #[test]
    fn section_xml_plain_inline_has_no_charpr() {
        let xml = section_xml(vec![Block::Paragraph {
            inlines: vec![inline("plain")],
        }]);
        // No formatting → no hp:charPr element should be emitted
        assert!(!xml.contains("<hp:charPr"), "unexpected charPr: {xml}");
    }

    #[test]
    fn section_xml_nested_inlines_bold_then_italic() {
        let xml = section_xml(vec![Block::Paragraph {
            inlines: vec![bold_inline("B"), italic_inline("I")],
        }]);
        assert!(xml.contains(r#"bold="true""#), "{xml}");
        assert!(xml.contains(r#"italic="true""#), "{xml}");
        assert!(xml.contains("B"));
        assert!(xml.contains("I"));
    }

    #[test]
    fn section_xml_image_block() {
        let xml = section_xml(vec![Block::Image {
            src: "image001.png".into(),
            alt: "a cat".into(),
        }]);
        assert!(xml.contains("<hp:p>"), "{xml}");
        assert!(
            xml.contains(r#"hp:binaryItemIDRef="image001.png""#),
            "{xml}"
        );
        assert!(xml.contains(r#"alt="a cat""#), "{xml}");
        assert!(xml.contains("<hp:img"), "{xml}");
    }

    #[test]
    fn section_xml_table_2x2() {
        let cell = |text: &str| TableCell {
            blocks: vec![Block::Paragraph {
                inlines: vec![inline(text)],
            }],
            colspan: 1,
            rowspan: 1,
        };
        let xml = section_xml(vec![Block::Table {
            col_count: 2,
            rows: vec![
                TableRow {
                    cells: vec![cell("A"), cell("B")],
                    is_header: false,
                },
                TableRow {
                    cells: vec![cell("C"), cell("D")],
                    is_header: false,
                },
            ],
        }]);
        assert!(xml.contains("<hp:tbl>"), "tbl open: {xml}");
        assert!(xml.contains("</hp:tbl>"), "tbl close: {xml}");
        assert_eq!(xml.matches("<hp:tr>").count(), 2, "two rows: {xml}");
        assert_eq!(xml.matches("<hp:tc>").count(), 4, "four cells: {xml}");
        assert!(xml.contains("A"), "{xml}");
        assert!(xml.contains("D"), "{xml}");
    }

    #[test]
    fn section_xml_table_colspan_rowspan_present() {
        // Documents the current behavior: colspan/rowspan values exist on
        // the TableCell struct but the writer does NOT emit cellAddr or span
        // attributes on <hp:tc>. This test pins that (missing) behavior so
        // future work that adds span support can be detected as a regression
        // or upgrade.
        let wide_cell = TableCell {
            blocks: vec![],
            colspan: 2,
            rowspan: 1,
        };
        let xml = section_xml(vec![Block::Table {
            col_count: 2,
            rows: vec![TableRow {
                cells: vec![wide_cell],
                is_header: false,
            }],
        }]);
        // The hp:tc element is emitted but without colspan/rowspan attributes.
        assert!(xml.contains("<hp:tc>"), "plain tc emitted: {xml}");
        // Confirm span attributes are absent (expected limitation).
        assert!(!xml.contains("colspan"), "colspan should not appear: {xml}");
        assert!(!xml.contains("rowspan"), "rowspan should not appear: {xml}");
    }

    #[test]
    fn section_xml_math_block() {
        let xml = section_xml(vec![Block::Math {
            display: true,
            tex: r"E = mc^2".into(),
        }]);
        assert!(xml.contains("<hp:equation>"), "equation open: {xml}");
        assert!(xml.contains("</hp:equation>"), "equation close: {xml}");
        assert!(xml.contains(r"E = mc^2"), "{xml}");
    }

    #[test]
    fn section_xml_ordered_list() {
        let xml = section_xml(vec![Block::List {
            ordered: true,
            start: 1,
            items: vec![
                ListItem {
                    blocks: vec![Block::Paragraph {
                        inlines: vec![inline("first")],
                    }],
                    children: vec![],
                },
                ListItem {
                    blocks: vec![Block::Paragraph {
                        inlines: vec![inline("second")],
                    }],
                    children: vec![],
                },
            ],
        }]);
        assert!(xml.contains("first"), "{xml}");
        assert!(xml.contains("second"), "{xml}");
        assert_eq!(xml.matches("<hp:p>").count(), 2, "{xml}");
    }

    #[test]
    fn section_xml_unordered_list() {
        let xml = section_xml(vec![Block::List {
            ordered: false,
            start: 1,
            items: vec![ListItem {
                blocks: vec![Block::Paragraph {
                    inlines: vec![inline("bullet")],
                }],
                children: vec![],
            }],
        }]);
        assert!(xml.contains("bullet"), "{xml}");
    }

    #[test]
    fn section_xml_footnote_block() {
        let xml = section_xml(vec![Block::Footnote {
            id: "fn1".into(),
            content: vec![Block::Paragraph {
                inlines: vec![inline("footnote text")],
            }],
        }]);
        assert!(xml.contains("footnote text"), "{xml}");
    }

    #[test]
    fn section_xml_blockquote() {
        let xml = section_xml(vec![Block::BlockQuote {
            blocks: vec![Block::Paragraph {
                inlines: vec![inline("quoted")],
            }],
        }]);
        assert!(xml.contains("quoted"), "{xml}");
        assert!(xml.contains("<hp:p>"), "{xml}");
    }

    #[test]
    fn section_xml_horizontal_rule() {
        let xml = section_xml(vec![Block::HorizontalRule]);
        assert!(xml.contains("<hp:p>"), "{xml}");
        // The writer emits a line of em-dashes as a visual rule.
        assert!(xml.contains("───"), "{xml}");
    }

    #[test]
    fn section_xml_code_block() {
        let xml = section_xml(vec![Block::CodeBlock {
            language: Some("rust".into()),
            code: "fn main() {}".into(),
        }]);
        assert!(xml.contains("<hp:p>"), "{xml}");
        assert!(xml.contains(r#"hp:charPrIDRef="code""#), "{xml}");
        assert!(xml.contains("fn main() {}"), "{xml}");
    }

    #[test]
    fn section_xml_multiple_blocks_ordering() {
        let xml = section_xml(vec![
            Block::Heading {
                level: 2,
                inlines: vec![inline("Section")],
            },
            Block::Paragraph {
                inlines: vec![inline("Body text")],
            },
        ]);
        // The heading must come before the paragraph in document order.
        let heading_pos = xml.find("Section").expect("heading text");
        let para_pos = xml.find("Body text").expect("para text");
        assert!(heading_pos < para_pos, "heading before paragraph: {xml}");
    }

    // ── write_hwpx integration: ZIP entry presence ─────────────────────────

    #[test]
    fn write_hwpx_empty_doc_produces_required_entries() {
        let tmp = tempfile::NamedTempFile::new().expect("tmp file");
        let doc = Document::new();
        write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");

        let entries = zip_entry_names(tmp.path());
        assert!(entries.contains(&"mimetype".to_owned()), "{entries:?}");
        assert!(
            entries.contains(&"META-INF/container.xml".to_owned()),
            "{entries:?}"
        );
        assert!(
            entries.contains(&"Contents/header.xml".to_owned()),
            "{entries:?}"
        );
        assert!(
            entries.contains(&"Contents/content.hpf".to_owned()),
            "{entries:?}"
        );
        assert!(
            entries.contains(&"Contents/section0.xml".to_owned()),
            "{entries:?}"
        );
    }

    #[test]
    fn write_hwpx_mimetype_is_stored_uncompressed() {
        // HWPX spec: mimetype must use Stored (no compression).
        let tmp = tempfile::NamedTempFile::new().expect("tmp file");
        write_hwpx(&Document::new(), tmp.path(), None).expect("write");

        let file = std::fs::File::open(tmp.path()).expect("open");
        let mut archive = zip::ZipArchive::new(file).expect("parse zip");
        let entry = archive.by_name("mimetype").expect("mimetype entry");
        assert_eq!(
            entry.compression(),
            zip::CompressionMethod::Stored,
            "mimetype must be Stored"
        );
    }

    #[test]
    fn write_hwpx_mimetype_content() {
        let tmp = tempfile::NamedTempFile::new().expect("tmp file");
        write_hwpx(&Document::new(), tmp.path(), None).expect("write");

        let file = std::fs::File::open(tmp.path()).expect("open");
        let mut archive = zip::ZipArchive::new(file).expect("parse zip");
        let mut entry = archive.by_name("mimetype").expect("mimetype entry");
        let mut content = String::new();
        entry.read_to_string(&mut content).expect("read");
        assert_eq!(content, "application/hwpx+zip");
    }

    #[test]
    fn write_hwpx_single_section_produces_section0_xml() {
        let tmp = tempfile::NamedTempFile::new().expect("tmp file");
        let doc = doc_with_section(vec![Block::Paragraph {
            inlines: vec![inline("hello")],
        }]);
        write_hwpx(&doc, tmp.path(), None).expect("write");

        let entries = zip_entry_names(tmp.path());
        assert!(
            entries.contains(&"Contents/section0.xml".to_owned()),
            "{entries:?}"
        );
        // With one explicit section there should NOT be a duplicate section0.
        assert_eq!(
            entries.iter().filter(|e| e.contains("section")).count(),
            1,
            "exactly one section entry: {entries:?}"
        );
    }

    #[test]
    fn write_hwpx_two_sections_produces_section0_and_section1() {
        let tmp = tempfile::NamedTempFile::new().expect("tmp file");
        let doc = Document {
            metadata: Metadata::default(),
            sections: vec![
                Section {
                    blocks: vec![Block::Paragraph {
                        inlines: vec![inline("s0")],
                    }],
                },
                Section {
                    blocks: vec![Block::Paragraph {
                        inlines: vec![inline("s1")],
                    }],
                },
            ],
            assets: Vec::new(),
        };
        write_hwpx(&doc, tmp.path(), None).expect("write");

        let entries = zip_entry_names(tmp.path());
        assert!(
            entries.contains(&"Contents/section0.xml".to_owned()),
            "{entries:?}"
        );
        assert!(
            entries.contains(&"Contents/section1.xml".to_owned()),
            "{entries:?}"
        );
    }

    #[test]
    fn write_hwpx_with_bindata_asset_produces_bindata_entry() {
        let tmp = tempfile::NamedTempFile::new().expect("tmp file");
        let doc = Document {
            metadata: Metadata::default(),
            sections: Vec::new(),
            assets: vec![Asset {
                name: "photo.png".into(),
                data: vec![0x89, 0x50, 0x4e, 0x47],
                mime_type: "image/png".into(),
            }],
        };
        write_hwpx(&doc, tmp.path(), None).expect("write");

        let entries = zip_entry_names(tmp.path());
        assert!(
            entries.contains(&"BinData/photo.png".to_owned()),
            "{entries:?}"
        );
    }

    #[test]
    fn write_hwpx_asset_with_path_prefix_uses_basename_only() {
        let tmp = tempfile::NamedTempFile::new().expect("tmp file");
        let doc = Document {
            metadata: Metadata::default(),
            sections: Vec::new(),
            assets: vec![Asset {
                name: "/some/nested/path/image.jpg".into(),
                data: vec![0xFF, 0xD8],
                mime_type: "image/jpeg".into(),
            }],
        };
        write_hwpx(&doc, tmp.path(), None).expect("write");

        let entries = zip_entry_names(tmp.path());
        // Only the basename should be used inside BinData/.
        assert!(
            entries.contains(&"BinData/image.jpg".to_owned()),
            "{entries:?}"
        );
        assert!(
            !entries.iter().any(|e| e.contains("/some/nested/")),
            "path prefix must be stripped: {entries:?}"
        );
    }

    #[test]
    fn write_hwpx_header_xml_contains_title_and_author() {
        let tmp = tempfile::NamedTempFile::new().expect("tmp file");
        let doc = Document {
            metadata: Metadata {
                title: Some("My Title".into()),
                author: Some("Alice".into()),
                ..Metadata::default()
            },
            sections: Vec::new(),
            assets: Vec::new(),
        };
        write_hwpx(&doc, tmp.path(), None).expect("write");

        let file = std::fs::File::open(tmp.path()).expect("open");
        let mut archive = zip::ZipArchive::new(file).expect("parse zip");
        let mut entry = archive.by_name("Contents/header.xml").expect("header.xml");
        let mut content = String::new();
        entry.read_to_string(&mut content).expect("read");
        assert!(content.contains("My Title"), "{content}");
        assert!(content.contains("Alice"), "{content}");
    }

    #[test]
    fn write_hwpx_content_hpf_references_sections() {
        let tmp = tempfile::NamedTempFile::new().expect("tmp file");
        let doc = doc_with_section(vec![]);
        write_hwpx(&doc, tmp.path(), None).expect("write");

        let file = std::fs::File::open(tmp.path()).expect("open");
        let mut archive = zip::ZipArchive::new(file).expect("parse zip");
        let mut entry = archive
            .by_name("Contents/content.hpf")
            .expect("content.hpf");
        let mut content = String::new();
        entry.read_to_string(&mut content).expect("read");
        assert!(content.contains("section0.xml"), "{content}");
    }
}
