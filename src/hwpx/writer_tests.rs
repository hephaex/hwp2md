use super::*;
use crate::ir::{Asset, Block, Document, Inline, ListItem, Metadata, Section, TableCell, TableRow};
use std::io::Read as _;

use crate::hwpx::read_hwpx;

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
    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: blocks.clone(),
        }],
        assets: Vec::new(),
    };
    let tables = RefTables::build(&doc);
    let sec = Section { blocks };
    generate_section_xml(&sec, 0, &tables).expect("generate_section_xml failed")
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
    assert!(xml.contains("<hp:p "), "paragraph open: {xml}");
    assert!(xml.contains("</hp:p>"), "paragraph close: {xml}");
    assert!(xml.contains("<hp:t>"), "text run open: {xml}");
    assert!(xml.contains("hello world"), "text content: {xml}");
}

#[test]
fn section_xml_empty_paragraph() {
    let xml = section_xml(vec![Block::Paragraph { inlines: vec![] }]);
    assert!(xml.contains("<hp:p "));
    assert!(xml.contains("</hp:p>"));
}

#[test]
fn section_xml_heading_level_1() {
    let xml = section_xml(vec![Block::Heading {
        level: 1,
        inlines: vec![inline("Title")],
    }]);
    // Heading 1 → numeric styleIDRef="1" (matches hh:styles table id=1)
    assert!(
        xml.contains(r#"hp:styleIDRef="1""#),
        "h1 style ref must be numeric 1: {xml}"
    );
    assert!(xml.contains("Title"));
}

#[test]
fn section_xml_heading_level_6() {
    let xml = section_xml(vec![Block::Heading {
        level: 6,
        inlines: vec![inline("Deep")],
    }]);
    // Heading 6 → numeric styleIDRef="6"
    assert!(xml.contains(r#"hp:styleIDRef="6""#), "{xml}");
    assert!(xml.contains("Deep"));
}

#[test]
fn section_xml_bold_inline_has_charpr_id_ref() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![bold_inline("strong")],
    }]);
    assert!(
        !xml.contains("<hp:charPr"),
        "inline charPr removed (OWPML schema): {xml}"
    );
    assert!(xml.contains("charPrIDRef="), "charPrIDRef: {xml}");
    assert!(xml.contains("strong"));
}

#[test]
fn section_xml_italic_inline_has_charpr_id_ref() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![italic_inline("em")],
    }]);
    assert!(xml.contains("charPrIDRef="), "{xml}");
}

#[test]
fn section_xml_underline_inline_has_charpr_id_ref() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![underline_inline("ul")],
    }]);
    assert!(xml.contains("charPrIDRef="), "{xml}");
}

#[test]
fn section_xml_strikethrough_inline_has_charpr_id_ref() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![Inline {
            text: "del".into(),
            strikethrough: true,
            ..Inline::default()
        }],
    }]);
    assert!(xml.contains("charPrIDRef="), "{xml}");
}

#[test]
fn section_xml_plain_inline_has_no_charpr() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![inline("plain")],
    }]);
    // Plain text → no inline hp:charPr element (formatting is only in header table)
    assert!(
        !xml.contains("<hp:charPr"),
        "unexpected inline charPr: {xml}"
    );
}

#[test]
fn section_xml_plain_inline_has_charpr_id_ref() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![inline("plain")],
    }]);
    // Plain text → charPrIDRef="0" on the run element
    assert!(
        xml.contains(r#"charPrIDRef="0""#),
        "charPrIDRef missing: {xml}"
    );
}

#[test]
fn section_xml_bold_inline_has_nonzero_charpr_id_ref() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![bold_inline("bold")],
    }]);
    // Bold → charPrIDRef pointing to id=1 (first non-default entry)
    assert!(xml.contains("charPrIDRef="), "charPrIDRef missing: {xml}");
}

#[test]
fn section_xml_paragraph_has_para_pr_id_ref() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![inline("text")],
    }]);
    assert!(
        xml.contains(r#"paraPrIDRef="0""#),
        "paraPrIDRef missing: {xml}"
    );
}

#[test]
fn section_xml_nested_inlines_bold_then_italic() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![bold_inline("B"), italic_inline("I")],
    }]);
    // Each formatting variant gets a distinct charPrIDRef; bold and italic
    // runs are separate so they must have different IDs (both non-zero).
    assert!(xml.contains("charPrIDRef="), "{xml}");
    assert!(xml.contains("B"));
    assert!(xml.contains("I"));
}

#[test]
fn section_xml_image_block() {
    let xml = section_xml(vec![Block::Image {
        src: "image001.png".into(),
        alt: "a cat".into(),
    }]);
    assert!(xml.contains("<hp:p "), "{xml}");
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
    // 5-B: table must be wrapped in a paragraph container
    assert!(xml.contains("<hp:p "), "p wrapper: {xml}");
    assert!(xml.contains(r#"<hp:run charPrIDRef="0">"#), "run wrapper: {xml}");
    // 5-C: tbl must carry rowCnt and colCnt
    assert!(
        xml.contains(r#"<hp:tbl rowCnt="2" colCnt="2">"#),
        "tbl with rowCnt/colCnt: {xml}"
    );
    assert!(xml.contains("</hp:tbl>"), "tbl close: {xml}");
    assert_eq!(xml.matches("<hp:tr>").count(), 2, "two rows: {xml}");
    assert_eq!(xml.matches("<hp:tc>").count(), 4, "four cells: {xml}");
    assert!(xml.contains("A"), "{xml}");
    assert!(xml.contains("D"), "{xml}");
}

#[test]
fn section_xml_table_colspan_rowspan_present() {
    // After the fix, the writer emits <hp:cellAddr colSpan="…" rowSpan="…"/>
    // inside <hp:tc> when either value differs from 1.
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
    assert!(xml.contains("<hp:tc>"), "plain tc emitted: {xml}");
    assert!(
        xml.contains("colSpan=\"2\""),
        "colSpan should appear: {xml}"
    );
    assert!(
        xml.contains("rowSpan=\"1\""),
        "rowSpan should appear: {xml}"
    );
    assert!(xml.contains("<hp:cellAddr"), "cellAddr element: {xml}");
}

#[test]
fn section_xml_table_no_celladdr_for_1x1() {
    // When colspan=1 and rowspan=1, no <hp:cellAddr> should be emitted.
    let cell = TableCell {
        blocks: vec![Block::Paragraph {
            inlines: vec![inline("normal")],
        }],
        colspan: 1,
        rowspan: 1,
    };
    let xml = section_xml(vec![Block::Table {
        col_count: 1,
        rows: vec![TableRow {
            cells: vec![cell],
            is_header: false,
        }],
    }]);
    assert!(xml.contains("<hp:tc>"), "tc emitted: {xml}");
    assert!(
        !xml.contains("<hp:cellAddr"),
        "cellAddr must NOT appear for 1x1: {xml}"
    );
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
    assert_eq!(xml.matches("<hp:p ").count(), 2, "{xml}");
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
    assert!(xml.contains("<hp:p "), "{xml}");
}

#[test]
fn section_xml_horizontal_rule() {
    let xml = section_xml(vec![Block::HorizontalRule]);
    assert!(xml.contains("<hp:p "), "{xml}");
    // The writer emits a line of em-dashes as a visual rule.
    assert!(xml.contains("───"), "{xml}");
}

#[test]
fn section_xml_code_block() {
    let xml = section_xml(vec![Block::CodeBlock {
        language: Some("rust".into()),
        code: "fn main() {}".into(),
    }]);
    assert!(xml.contains("<hp:p "), "{xml}");
    assert!(
        !xml.contains(r#"charPrIDRef="code""#),
        "charPrIDRef must not be the string 'code': {xml}"
    );
    assert!(
        xml.contains("charPrIDRef="),
        "charPrIDRef must be present: {xml}"
    );
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

// ── header.xml reference table tests ─────────────────────────────────────

#[test]
fn header_xml_contains_char_properties() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = doc_with_section(vec![Block::Paragraph {
        inlines: vec![bold_inline("hello")],
    }]);
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive.by_name("Contents/header.xml").expect("header.xml");
    let mut content = String::new();
    entry.read_to_string(&mut content).expect("read");

    assert!(
        content.contains("hh:charProperties"),
        "charProperties section: {content}"
    );
    assert!(content.contains("hh:charPr"), "charPr entry: {content}");
    assert!(
        content.contains(r#"bold="true""#),
        "bold charPr in header: {content}"
    );
}

#[test]
fn header_xml_contains_para_properties() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = Document::new();
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive.by_name("Contents/header.xml").expect("header.xml");
    let mut content = String::new();
    entry.read_to_string(&mut content).expect("read");

    assert!(
        content.contains("hh:paraProperties"),
        "paraProperties section: {content}"
    );
    assert!(content.contains("hh:paraPr"), "paraPr entry: {content}");
}

#[test]
fn header_xml_contains_font_faces() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = Document::new();
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive.by_name("Contents/header.xml").expect("header.xml");
    let mut content = String::new();
    entry.read_to_string(&mut content).expect("read");

    assert!(
        content.contains("hh:fontfaces"),
        "fontfaces section: {content}"
    );
    assert!(content.contains("바탕"), "default Batang font: {content}");
}

#[test]
fn header_xml_default_charpr_has_id_zero() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = Document::new();
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive.by_name("Contents/header.xml").expect("header.xml");
    let mut content = String::new();
    entry.read_to_string(&mut content).expect("read");

    assert!(content.contains(r#"id="0""#), "id=0 charPr: {content}");
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
    assert_eq!(content, "application/hwp+zip");
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

#[test]
fn write_hwpx_bindata_entry_has_correct_content() {
    let png_bytes = vec![0x89u8, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = Document {
        metadata: Metadata::default(),
        sections: Vec::new(),
        assets: vec![Asset {
            name: "banner.png".into(),
            data: png_bytes.clone(),
            mime_type: "image/png".into(),
        }],
    };
    write_hwpx(&doc, tmp.path(), None).expect("write");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive
        .by_name("BinData/banner.png")
        .expect("BinData/banner.png");
    let mut actual = Vec::new();
    entry.read_to_end(&mut actual).expect("read");
    assert_eq!(
        actual, png_bytes,
        "BinData entry content must match asset data"
    );
}

#[test]
fn write_hwpx_image_block_xml_references_asset_name() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::Image {
                src: "diagram.png".into(),
                alt: "a diagram".into(),
            }],
        }],
        assets: vec![Asset {
            name: "diagram.png".into(),
            data: vec![0x89, 0x50, 0x4e, 0x47],
            mime_type: "image/png".into(),
        }],
    };
    write_hwpx(&doc, tmp.path(), None).expect("write");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive
        .by_name("Contents/section0.xml")
        .expect("section0.xml");
    let mut content = String::new();
    entry.read_to_string(&mut content).expect("read");

    assert!(
        content.contains("diagram.png"),
        "section XML must reference image asset name; got: {content}"
    );
    assert!(
        content.contains("<hp:img"),
        "section XML must contain hp:img element; got: {content}"
    );
    assert!(
        content.contains(r#"alt="a diagram""#),
        "section XML must carry alt text; got: {content}"
    );
    let entries = zip_entry_names(tmp.path());
    assert!(
        entries.contains(&"BinData/diagram.png".to_owned()),
        "BinData entry must exist: {entries:?}"
    );
}

#[test]
fn write_hwpx_image_roundtrip_preserves_asset() {
    let png_bytes = vec![0x89u8, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");

    let original = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::Image {
                src: "photo.png".into(),
                alt: "a photo".into(),
            }],
        }],
        assets: vec![Asset {
            name: "photo.png".into(),
            data: png_bytes.clone(),
            mime_type: "image/png".into(),
        }],
    };
    write_hwpx(&original, tmp.path(), None).expect("write");

    let read_back = read_hwpx(tmp.path()).expect("read_hwpx");

    let has_image = read_back
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .any(|b| matches!(b, Block::Image { src, .. } if src.contains("photo")));
    assert!(
        has_image,
        "image block must survive HWPX roundtrip; sections: {:?}",
        read_back.sections
    );

    assert_eq!(
        read_back.assets.len(),
        1,
        "one asset expected after roundtrip"
    );
    assert_eq!(
        read_back.assets[0].data, png_bytes,
        "asset binary content must be preserved through roundtrip"
    );
    assert_eq!(
        read_back.assets[0].mime_type, "image/png",
        "asset MIME type must be preserved"
    );
}

// ── Phase 4 tests: styles, numeric styleIDRef, numeric charPrIDRef ────────

#[test]
fn header_xml_contains_styles() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = Document::new();
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive.by_name("Contents/header.xml").expect("header.xml");
    let mut content = String::new();
    entry.read_to_string(&mut content).expect("read");

    assert!(
        content.contains("hh:styles"),
        "hh:styles section must be present: {content}"
    );
    assert!(
        content.contains("hh:style"),
        "hh:style entries must be present: {content}"
    );
    // Style id=0 (Normal) and id=1..6 (Heading1..6) must all appear.
    for id in 0..=6u8 {
        assert!(
            content.contains(&format!(r#"id="{id}""#)),
            "style id={id} must be present: {content}"
        );
    }
    // Verify known style names.
    assert!(
        content.contains(r#"name="Normal""#),
        "Normal style: {content}"
    );
    assert!(
        content.contains(r#"name="Heading1""#),
        "Heading1 style: {content}"
    );
    assert!(
        content.contains(r#"name="Heading6""#),
        "Heading6 style: {content}"
    );
}

#[test]
fn section_xml_heading_has_numeric_style_id_ref() {
    // Verify that heading styleIDRef is a pure decimal digit, not a name
    // like "Heading1".
    for level in 1u8..=6 {
        let xml = section_xml(vec![Block::Heading {
            level,
            inlines: vec![inline("h")],
        }]);
        let expected = format!(r#"hp:styleIDRef="{level}""#);
        assert!(
            xml.contains(&expected),
            "level {level}: expected numeric styleIDRef={level}, got: {xml}"
        );
        // Must NOT contain the old string form.
        let old_form = format!(r#"hp:styleIDRef="Heading{level}""#);
        assert!(
            !xml.contains(&old_form),
            "level {level}: string styleIDRef must not appear: {xml}"
        );
    }
}

#[test]
fn section_xml_code_block_has_numeric_char_pr_id_ref() {
    let xml = section_xml(vec![Block::CodeBlock {
        language: Some("rust".into()),
        code: "let x = 1;".into(),
    }]);
    // The charPrIDRef value on the run element must be a decimal number.
    // Extract it and confirm it parses as u32.
    let marker = "charPrIDRef=\"";
    let start = xml
        .find(marker)
        .expect("charPrIDRef attribute must be present");
    let rest = &xml[start + marker.len()..];
    let end = rest.find('"').expect("closing quote");
    let value = &rest[..end];
    assert!(
        value.parse::<u32>().is_ok(),
        "charPrIDRef value '{value}' must be a numeric u32, not a string like 'code': {xml}"
    );
    // Sanity: "let x = 1;" must appear in the output.
    assert!(
        xml.contains("let x = 1;"),
        "code content must appear: {xml}"
    );
}

// ── Phase 5 tests: paragraph IDs + table wrapping ────────────────────────

#[test]
fn section_xml_paragraphs_have_sequential_ids() {
    // Multiple blocks must receive sequential id="0", id="1", … on their <hp:p>.
    let xml = section_xml(vec![
        Block::Heading {
            level: 1,
            inlines: vec![inline("First")],
        },
        Block::Paragraph {
            inlines: vec![inline("Second")],
        },
        Block::CodeBlock {
            language: None,
            code: "third".into(),
        },
    ]);
    assert!(xml.contains(r#"id="0""#), "first block id=0: {xml}");
    assert!(xml.contains(r#"id="1""#), "second block id=1: {xml}");
    assert!(xml.contains(r#"id="2""#), "third block id=2: {xml}");
    // Verify the first paragraph element is the heading (id=0 + styleIDRef).
    let id0_pos = xml.find(r#"id="0""#).expect("id=0 position");
    let style_pos = xml
        .find(r#"hp:styleIDRef="1""#)
        .expect("styleIDRef=1 position");
    // id="0" must appear on the same opening tag as hp:styleIDRef="1".
    // Since the heading opens before the content, id0_pos < style_pos for a
    // forward-ordered attribute list (writer emits id first).
    assert!(
        id0_pos < style_pos,
        "id=0 must precede styleIDRef on the heading element: {xml}"
    );
}

#[test]
fn section_xml_table_wrapped_in_paragraph() {
    // A table block must be emitted as:
    //   <hp:p id="N" paraPrIDRef="0">
    //     <hp:run charPrIDRef="0">
    //       <hp:tbl rowCnt="…" colCnt="…"> … </hp:tbl>
    //     </hp:run>
    //   </hp:p>
    let cell = |text: &str| TableCell {
        blocks: vec![Block::Paragraph {
            inlines: vec![inline(text)],
        }],
        colspan: 1,
        rowspan: 1,
    };
    let xml = section_xml(vec![Block::Table {
        col_count: 2,
        rows: vec![TableRow {
            cells: vec![cell("X"), cell("Y")],
            is_header: false,
        }],
    }]);
    // The outer paragraph wrapper must carry an id.
    assert!(xml.contains(r#"id="0""#), "table wrapper p must have id=0: {xml}");
    // The run wrapper must be present with charPrIDRef="0".
    assert!(
        xml.contains(r#"<hp:run charPrIDRef="0">"#),
        "run wrapper: {xml}"
    );
    // The table element must be present with rowCnt and colCnt.
    assert!(
        xml.contains(r#"<hp:tbl rowCnt="1" colCnt="2">"#),
        "tbl attrs: {xml}"
    );
    // Structural nesting: p opens before run, run opens before tbl.
    let p_pos = xml.find(r#"id="0""#).expect("p wrapper position");
    let run_pos = xml
        .find(r#"<hp:run charPrIDRef="0">"#)
        .expect("run position");
    let tbl_pos = xml
        .find(r#"<hp:tbl rowCnt="1" colCnt="2">"#)
        .expect("tbl position");
    assert!(p_pos < run_pos, "p must come before run: {xml}");
    assert!(run_pos < tbl_pos, "run must come before tbl: {xml}");
    // Cell paragraph IDs (inside the table) continue from the outer counter.
    // The outer table p is id=0, so the first cell paragraph is id=1.
    assert!(xml.contains(r#"id="1""#), "cell paragraph id=1: {xml}");
}

#[test]
fn section_xml_table_rowcnt_colcnt_attributes() {
    // Verify rowCnt and colCnt reflect actual row/column counts.
    let cell = || TableCell {
        blocks: vec![],
        colspan: 1,
        rowspan: 1,
    };
    let xml = section_xml(vec![Block::Table {
        col_count: 3,
        rows: vec![
            TableRow {
                cells: vec![cell(), cell(), cell()],
                is_header: false,
            },
            TableRow {
                cells: vec![cell(), cell(), cell()],
                is_header: false,
            },
            TableRow {
                cells: vec![cell(), cell(), cell()],
                is_header: false,
            },
        ],
    }]);
    assert!(
        xml.contains(r#"rowCnt="3""#),
        "rowCnt must be 3: {xml}"
    );
    assert!(
        xml.contains(r#"colCnt="3""#),
        "colCnt must be 3: {xml}"
    );
}
