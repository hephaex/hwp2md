use super::*;
use crate::ir::{Asset, Block, Document, Inline, ListItem, Metadata, Section, TableCell, TableRow};
use std::io::Read as _;

use crate::hwpx::read_hwpx;
use crate::hwpx::reader::parse_section_xml;

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

fn read_hwpx_section_xml(xml: &str) -> Section {
    parse_section_xml(xml).expect("parse_section_xml failed")
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
    // 6-A: image must be wrapped in <hp:run><hp:pic>...</hp:pic></hp:run>
    assert!(
        xml.contains(r#"<hp:run charPrIDRef="0">"#),
        "image run wrapper missing: {xml}"
    );
    assert!(xml.contains("<hp:pic>"), "hp:pic wrapper missing: {xml}");
    assert!(xml.contains("</hp:pic>"), "hp:pic close missing: {xml}");
    // Verify nesting order: run → pic → img
    let run_pos = xml
        .find(r#"<hp:run charPrIDRef="0">"#)
        .expect("run position");
    let pic_pos = xml.find("<hp:pic>").expect("pic position");
    let img_pos = xml.find("<hp:img").expect("img position");
    assert!(run_pos < pic_pos, "run must precede pic: {xml}");
    assert!(pic_pos < img_pos, "pic must precede img: {xml}");
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
    // 6-B: equation must be wrapped in <hp:run charPrIDRef="0">
    assert!(
        xml.contains(r#"<hp:run charPrIDRef="0">"#),
        "equation run wrapper missing: {xml}"
    );
    // Verify nesting order: run → equation
    let run_pos = xml
        .find(r#"<hp:run charPrIDRef="0">"#)
        .expect("run position");
    let eq_pos = xml.find("<hp:equation>").expect("equation position");
    assert!(run_pos < eq_pos, "run must precede equation: {xml}");
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
    // OWPML schema does not allow bold/italic/underline/strikeout as charPr
    // attributes; bold formatting is expressed through distinct charPr IDs.
    assert!(
        !content.contains(r#"bold="true""#),
        "bold must NOT appear as a charPr attribute (schema violation): {content}"
    );
    // The bold inline must produce a non-default (id != 0) charPr entry.
    assert!(
        content.contains(r#"id="1""#),
        "bold inline must produce a charPr with id=1: {content}"
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

// ── Phase 7 tests: borderFill table ─────────────────────────────────────

#[test]
fn header_xml_contains_border_fills_section() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = Document::new();
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive.by_name("Contents/header.xml").expect("header.xml");
    let mut content = String::new();
    entry.read_to_string(&mut content).expect("read");

    assert!(
        content.contains("hh:borderFills"),
        "borderFills section must be present: {content}"
    );
    assert!(
        content.contains("hh:borderFill"),
        "borderFill entry must be present: {content}"
    );
}

#[test]
fn header_xml_border_fill_has_default_entry() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = Document::new();
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive.by_name("Contents/header.xml").expect("header.xml");
    let mut content = String::new();
    entry.read_to_string(&mut content).expect("read");

    // borderFill id=1 with required attributes.
    assert!(
        content.contains(r#"id="1""#),
        "borderFill id=1 must exist: {content}"
    );
    assert!(
        content.contains(r#"threeD="false""#),
        "threeD attribute: {content}"
    );
    assert!(
        content.contains(r#"shadow="false""#),
        "shadow attribute: {content}"
    );
    // Border elements must be present.
    assert!(
        content.contains("hh:leftBorder"),
        "leftBorder: {content}"
    );
    assert!(
        content.contains("hh:rightBorder"),
        "rightBorder: {content}"
    );
    assert!(
        content.contains("hh:topBorder"),
        "topBorder: {content}"
    );
    assert!(
        content.contains("hh:bottomBorder"),
        "bottomBorder: {content}"
    );
    assert!(
        content.contains("hh:diagonal"),
        "diagonal: {content}"
    );
    // Slash and backSlash elements must be present.
    assert!(
        content.contains("hh:slash"),
        "slash element: {content}"
    );
    assert!(
        content.contains("hh:backSlash"),
        "backSlash element: {content}"
    );
}

#[test]
fn header_xml_charpr_has_border_fill_id_ref() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = doc_with_section(vec![Block::Paragraph {
        inlines: vec![inline("text")],
    }]);
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive.by_name("Contents/header.xml").expect("header.xml");
    let mut content = String::new();
    entry.read_to_string(&mut content).expect("read");

    // Every charPr entry must have borderFillIDRef="1".
    assert!(
        content.contains(r#"borderFillIDRef="1""#),
        "charPr must reference borderFill id=1: {content}"
    );
    // Must NOT reference borderFillIDRef="0" (nonexistent entry).
    assert!(
        !content.contains(r#"borderFillIDRef="0""#),
        "borderFillIDRef=0 must not appear (no such entry): {content}"
    );
}

#[test]
fn header_xml_border_fills_precedes_char_properties() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = Document::new();
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive.by_name("Contents/header.xml").expect("header.xml");
    let mut content = String::new();
    entry.read_to_string(&mut content).expect("read");

    let bf_pos = content
        .find("hh:borderFills")
        .expect("borderFills must exist");
    let cp_pos = content
        .find("hh:charProperties")
        .expect("charProperties must exist");
    assert!(
        bf_pos < cp_pos,
        "borderFills must appear before charProperties in header.xml"
    );
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
fn write_hwpx_header_xml_has_version_and_sec_cnt() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = doc_with_section(vec![]);
    write_hwpx(&doc, tmp.path(), None).expect("write");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive.by_name("Contents/header.xml").expect("header.xml");
    let mut content = String::new();
    entry.read_to_string(&mut content).expect("read");
    assert!(
        content.contains(r#"version="1.1""#),
        "version attr: {content}"
    );
    assert!(
        content.contains(r#"secCnt="1""#),
        "secCnt attr: {content}"
    );
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

// ── Phase 8 tests: inline code charPr ───────────────────────────────────

#[test]
fn charpr_key_from_inline_code_sets_code_true_and_monospace_font() {
    let inline = Inline {
        text: "x".into(),
        code: true,
        ..Inline::default()
    };
    let key = CharPrKey::from_inline(&inline);
    assert!(key.code, "code flag must be true");
    assert_eq!(
        key.font_name.as_deref(),
        Some("Courier New"),
        "code inline must use monospace font"
    );
}

#[test]
fn charpr_key_from_inline_code_overrides_custom_font() {
    // Even if the inline has a font_name, code=true forces Courier New.
    let inline = Inline {
        text: "x".into(),
        code: true,
        font_name: Some("Arial".into()),
        ..Inline::default()
    };
    let key = CharPrKey::from_inline(&inline);
    assert_eq!(
        key.font_name.as_deref(),
        Some("Courier New"),
        "code flag must override custom font"
    );
}

#[test]
fn charpr_key_plain_has_code_false() {
    let key = CharPrKey::plain();
    assert!(!key.code, "plain key must not be code");
}

#[test]
fn charpr_key_code_block_has_code_true() {
    let key = CharPrKey::code_block();
    assert!(key.code, "code_block key must have code=true");
    assert_eq!(key.font_name.as_deref(), Some("Courier New"));
}

#[test]
fn inline_code_gets_distinct_charpr_id_from_plain() {
    let doc = doc_with_section(vec![Block::Paragraph {
        inlines: vec![
            inline("normal"),
            Inline {
                text: "code".into(),
                code: true,
                ..Inline::default()
            },
        ],
    }]);
    let tables = RefTables::build(&doc);

    let plain_id = tables.char_pr_id(&CharPrKey::plain());
    let code_key = CharPrKey::from_inline(&Inline {
        text: "code".into(),
        code: true,
        ..Inline::default()
    });
    let code_id = tables.char_pr_id(&code_key);

    assert_ne!(
        plain_id, code_id,
        "inline code must have a different charPr ID than plain text"
    );
}

#[test]
fn section_xml_inline_code_has_monospace_charpr_id_ref() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![
            inline("text "),
            Inline {
                text: "code_val".into(),
                code: true,
                ..Inline::default()
            },
        ],
    }]);

    // There must be at least two <hp:run> elements with different charPrIDRef values.
    let run_count = xml.matches("<hp:run ").count();
    assert!(
        run_count >= 2,
        "must have at least 2 runs (plain + code): {xml}"
    );

    // The code text must be present.
    assert!(xml.contains("code_val"), "code text missing: {xml}");

    // Extract all charPrIDRef values and verify they are not all the same.
    let marker = "charPrIDRef=\"";
    let ids: Vec<&str> = xml
        .match_indices(marker)
        .map(|(pos, _)| {
            let rest = &xml[pos + marker.len()..];
            let end = rest.find('"').unwrap();
            &rest[..end]
        })
        .collect();
    assert!(
        ids.len() >= 2,
        "must have at least 2 charPrIDRef values: {ids:?}"
    );
    // At least one ID must differ from the plain ID (which is "0").
    let has_nonzero = ids.iter().any(|&id| id != "0");
    assert!(
        has_nonzero,
        "inline code must produce a non-zero charPrIDRef: {ids:?}"
    );
}

#[test]
fn header_xml_inline_code_registers_courier_new_font() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = doc_with_section(vec![Block::Paragraph {
        inlines: vec![Inline {
            text: "code".into(),
            code: true,
            ..Inline::default()
        }],
    }]);
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive.by_name("Contents/header.xml").expect("header.xml");
    let mut content = String::new();
    entry.read_to_string(&mut content).expect("read");

    assert!(
        content.contains("Courier New"),
        "Courier New font must be in header for inline code: {content}"
    );
}

// ── Phase 8 tests: metadata in content.hpf ──────────────────────────────

#[test]
fn content_hpf_with_title_and_author() {
    let doc = Document {
        metadata: Metadata {
            title: Some("Test Title".into()),
            author: Some("Test Author".into()),
            ..Metadata::default()
        },
        sections: vec![Section {
            blocks: vec![Block::Paragraph {
                inlines: vec![inline("hello")],
            }],
        }],
        assets: Vec::new(),
    };
    let hpf = generate_content_hpf(&doc);

    assert!(
        hpf.contains("<hp:docInfo>"),
        "docInfo section must be present: {hpf}"
    );
    assert!(
        hpf.contains("<hp:title>Test Title</hp:title>"),
        "title must be present: {hpf}"
    );
    assert!(
        hpf.contains("<hp:author>Test Author</hp:author>"),
        "author must be present: {hpf}"
    );
    assert!(
        hpf.contains("</hp:docInfo>"),
        "docInfo closing tag: {hpf}"
    );
}

#[test]
fn content_hpf_without_metadata_has_no_docinfo() {
    let doc = Document::new();
    let hpf = generate_content_hpf(&doc);

    assert!(
        !hpf.contains("<hp:docInfo>"),
        "docInfo must NOT appear when metadata is empty: {hpf}"
    );
}

#[test]
fn content_hpf_title_only() {
    let doc = Document {
        metadata: Metadata {
            title: Some("Only Title".into()),
            ..Metadata::default()
        },
        sections: Vec::new(),
        assets: Vec::new(),
    };
    let hpf = generate_content_hpf(&doc);

    assert!(
        hpf.contains("<hp:title>Only Title</hp:title>"),
        "title: {hpf}"
    );
    assert!(
        !hpf.contains("<hp:author>"),
        "author must NOT appear when absent: {hpf}"
    );
}

#[test]
fn content_hpf_author_only() {
    let doc = Document {
        metadata: Metadata {
            author: Some("Only Author".into()),
            ..Metadata::default()
        },
        sections: Vec::new(),
        assets: Vec::new(),
    };
    let hpf = generate_content_hpf(&doc);

    assert!(
        !hpf.contains("<hp:title>"),
        "title must NOT appear: {hpf}"
    );
    assert!(
        hpf.contains("<hp:author>Only Author</hp:author>"),
        "author: {hpf}"
    );
}

#[test]
fn content_hpf_metadata_xml_escaping() {
    let doc = Document {
        metadata: Metadata {
            title: Some("A & B <C>".into()),
            author: Some("D & E".into()),
            ..Metadata::default()
        },
        sections: Vec::new(),
        assets: Vec::new(),
    };
    let hpf = generate_content_hpf(&doc);

    assert!(
        hpf.contains("A &amp; B &lt;C&gt;"),
        "title must be XML-escaped: {hpf}"
    );
    assert!(
        hpf.contains("D &amp; E"),
        "author must be XML-escaped: {hpf}"
    );
}

#[test]
fn write_hwpx_metadata_in_content_hpf() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = Document {
        metadata: Metadata {
            title: Some("HWPX Title".into()),
            author: Some("HWPX Author".into()),
            ..Metadata::default()
        },
        sections: vec![Section {
            blocks: vec![Block::Paragraph {
                inlines: vec![inline("body")],
            }],
        }],
        assets: Vec::new(),
    };
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive
        .by_name("Contents/content.hpf")
        .expect("content.hpf");
    let mut content = String::new();
    entry.read_to_string(&mut content).expect("read");

    assert!(
        content.contains("<hp:title>HWPX Title</hp:title>"),
        "title in content.hpf: {content}"
    );
    assert!(
        content.contains("<hp:author>HWPX Author</hp:author>"),
        "author in content.hpf: {content}"
    );
}

// ── Phase 9 tests: superscript/subscript ──────────────────────────────────

#[test]
fn charpr_key_superscript_gets_distinct_id_from_plain() {
    let doc = doc_with_section(vec![Block::Paragraph {
        inlines: vec![
            inline("normal"),
            Inline {
                text: "sup".into(),
                superscript: true,
                ..Inline::default()
            },
        ],
    }]);
    let tables = RefTables::build(&doc);

    let plain_id = tables.char_pr_id(&CharPrKey::plain());
    let sup_key = CharPrKey::from_inline(&Inline {
        text: "sup".into(),
        superscript: true,
        ..Inline::default()
    });
    let sup_id = tables.char_pr_id(&sup_key);

    assert_ne!(
        plain_id, sup_id,
        "superscript must have a different charPr ID than plain text"
    );
}

#[test]
fn charpr_key_subscript_gets_distinct_id_from_plain() {
    let doc = doc_with_section(vec![Block::Paragraph {
        inlines: vec![
            inline("normal"),
            Inline {
                text: "sub".into(),
                subscript: true,
                ..Inline::default()
            },
        ],
    }]);
    let tables = RefTables::build(&doc);

    let plain_id = tables.char_pr_id(&CharPrKey::plain());
    let sub_key = CharPrKey::from_inline(&Inline {
        text: "sub".into(),
        subscript: true,
        ..Inline::default()
    });
    let sub_id = tables.char_pr_id(&sub_key);

    assert_ne!(
        plain_id, sub_id,
        "subscript must have a different charPr ID than plain text"
    );
}

#[test]
fn section_xml_superscript_inline_gets_correct_charpr_id_ref() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![
            inline("text "),
            Inline {
                text: "sup".into(),
                superscript: true,
                ..Inline::default()
            },
        ],
    }]);

    // Must have at least two runs with different charPrIDRef values.
    let marker = "charPrIDRef=\"";
    let ids: Vec<&str> = xml
        .match_indices(marker)
        .map(|(pos, _)| {
            let rest = &xml[pos + marker.len()..];
            let end = rest.find('"').unwrap();
            &rest[..end]
        })
        .collect();
    assert!(
        ids.len() >= 2,
        "must have at least 2 charPrIDRef values: {ids:?}"
    );
    let has_nonzero = ids.iter().any(|&id| id != "0");
    assert!(
        has_nonzero,
        "superscript inline must produce a non-zero charPrIDRef: {ids:?}"
    );
}

#[test]
fn header_xml_charpr_has_supscript_superscript_attribute() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = doc_with_section(vec![Block::Paragraph {
        inlines: vec![Inline {
            text: "sup".into(),
            superscript: true,
            ..Inline::default()
        }],
    }]);
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive.by_name("Contents/header.xml").expect("header.xml");
    let mut content = String::new();
    entry.read_to_string(&mut content).expect("read");

    assert!(
        content.contains(r#"supscript="superscript""#),
        "charPr must contain supscript=\"superscript\" attribute: {content}"
    );
}

#[test]
fn header_xml_charpr_has_supscript_subscript_attribute() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = doc_with_section(vec![Block::Paragraph {
        inlines: vec![Inline {
            text: "sub".into(),
            subscript: true,
            ..Inline::default()
        }],
    }]);
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive.by_name("Contents/header.xml").expect("header.xml");
    let mut content = String::new();
    entry.read_to_string(&mut content).expect("read");

    assert!(
        content.contains(r#"supscript="subscript""#),
        "charPr must contain supscript=\"subscript\" attribute: {content}"
    );
}

#[test]
fn header_xml_plain_charpr_has_no_supscript_attribute() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = doc_with_section(vec![Block::Paragraph {
        inlines: vec![inline("plain text only")],
    }]);
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive.by_name("Contents/header.xml").expect("header.xml");
    let mut content = String::new();
    entry.read_to_string(&mut content).expect("read");

    // Plain text charPr (id=0) must NOT have a supscript attribute.
    // However heading charPrs are also present, and they also lack supscript.
    // We verify that no charPr has supscript when no inline requests it.
    assert!(
        !content.contains("supscript="),
        "plain doc must not contain any supscript attribute: {content}"
    );
}

// ── Phase 9 tests: hyperlink writer ───────────────────────────────────────

#[test]
fn section_xml_link_inline_produces_field_begin_end() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![Inline {
            text: "click here".into(),
            link: Some("https://example.com".into()),
            ..Inline::default()
        }],
    }]);

    assert!(
        xml.contains("hp:fieldBegin"),
        "fieldBegin must be present: {xml}"
    );
    assert!(
        xml.contains("hp:fieldEnd"),
        "fieldEnd must be present: {xml}"
    );
    assert!(
        xml.contains(r#"type="HYPERLINK""#),
        "field type must be HYPERLINK: {xml}"
    );
    assert!(
        xml.contains(r#"command="https://example.com""#),
        "field command must contain the URL: {xml}"
    );
    assert!(
        xml.contains("click here"),
        "link text must be present: {xml}"
    );
}

#[test]
fn section_xml_field_begin_has_correct_attributes() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![Inline {
            text: "link".into(),
            link: Some("https://rust-lang.org".into()),
            ..Inline::default()
        }],
    }]);

    // fieldBegin must have both type and command attributes.
    let fb_pos = xml.find("hp:fieldBegin").expect("fieldBegin must exist");
    let fb_end = xml[fb_pos..].find("/>").expect("fieldBegin must be self-closing");
    let fb_tag = &xml[fb_pos..fb_pos + fb_end];
    assert!(
        fb_tag.contains(r#"type="HYPERLINK""#),
        "fieldBegin type attr: {fb_tag}"
    );
    assert!(
        fb_tag.contains(r#"command="https://rust-lang.org""#),
        "fieldBegin command attr: {fb_tag}"
    );
}

#[test]
fn section_xml_link_text_appears_between_field_markers() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![Inline {
            text: "Visit".into(),
            link: Some("https://example.com".into()),
            ..Inline::default()
        }],
    }]);

    let begin_pos = xml.find("hp:fieldBegin").expect("fieldBegin");
    let text_pos = xml.find("Visit").expect("link text");
    let end_pos = xml.find("hp:fieldEnd").expect("fieldEnd");

    assert!(
        begin_pos < text_pos,
        "fieldBegin must precede link text: {xml}"
    );
    assert!(
        text_pos < end_pos,
        "link text must precede fieldEnd: {xml}"
    );
}

#[test]
fn section_xml_non_link_inlines_remain_unchanged() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![inline("no link here")],
    }]);

    assert!(
        !xml.contains("hp:fieldBegin"),
        "non-link inlines must not produce fieldBegin: {xml}"
    );
    assert!(
        !xml.contains("hp:fieldEnd"),
        "non-link inlines must not produce fieldEnd: {xml}"
    );
    assert!(xml.contains("no link here"), "text must be present: {xml}");
}

#[test]
fn section_xml_mixed_link_and_non_link_inlines() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![
            inline("before "),
            Inline {
                text: "link".into(),
                link: Some("https://example.com".into()),
                ..Inline::default()
            },
            inline(" after"),
        ],
    }]);

    assert!(xml.contains("before "), "prefix text: {xml}");
    assert!(xml.contains("link"), "link text: {xml}");
    assert!(xml.contains(" after"), "suffix text: {xml}");
    assert!(
        xml.contains("hp:fieldBegin"),
        "fieldBegin for link inline: {xml}"
    );
    assert!(
        xml.contains("hp:fieldEnd"),
        "fieldEnd for link inline: {xml}"
    );

    // "before" must come before fieldBegin, "after" must come after fieldEnd.
    let before_pos = xml.find("before ").unwrap();
    let begin_pos = xml.find("hp:fieldBegin").unwrap();
    let end_pos = xml.find("hp:fieldEnd").unwrap();
    let after_pos = xml.find(" after").unwrap();
    assert!(before_pos < begin_pos, "prefix before fieldBegin");
    assert!(end_pos < after_pos, "fieldEnd before suffix");
}

#[test]
fn section_xml_consecutive_link_inlines_same_url_grouped() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![
            Inline {
                text: "part1".into(),
                link: Some("https://example.com".into()),
                ..Inline::default()
            },
            Inline {
                text: "part2".into(),
                link: Some("https://example.com".into()),
                ..Inline::default()
            },
        ],
    }]);

    // Two consecutive inlines with the same URL should be grouped into a
    // single fieldBegin/fieldEnd pair.
    let begin_count = xml.matches("hp:fieldBegin").count();
    let end_count = xml.matches("hp:fieldEnd").count();
    assert_eq!(begin_count, 1, "one fieldBegin for grouped link: {xml}");
    assert_eq!(end_count, 1, "one fieldEnd for grouped link: {xml}");
    assert!(xml.contains("part1"), "first part: {xml}");
    assert!(xml.contains("part2"), "second part: {xml}");
}

// ── Phase 9 tests: hyperlink reader ───────────────────────────────────────

#[test]
fn reader_parses_hyperlink_from_field_begin_end() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
        xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
  <hp:p id="0" paraPrIDRef="0">
    <hp:run charPrIDRef="0">
      <hp:fieldBegin type="HYPERLINK" command="https://example.com"/>
    </hp:run>
    <hp:run charPrIDRef="0">
      <hp:t>click here</hp:t>
    </hp:run>
    <hp:run charPrIDRef="0">
      <hp:fieldEnd type="HYPERLINK"/>
    </hp:run>
  </hp:p>
</hs:sec>"#;

    let section = read_hwpx_section_xml(xml);
    assert_eq!(section.blocks.len(), 1, "one paragraph");

    let inlines = match &section.blocks[0] {
        Block::Paragraph { inlines } => inlines,
        other => panic!("expected Paragraph, got: {other:?}"),
    };

    assert!(!inlines.is_empty(), "inlines should not be empty");
    let link_inline = &inlines[0];
    assert_eq!(link_inline.text, "click here");
    assert_eq!(
        link_inline.link.as_deref(),
        Some("https://example.com"),
        "link URL must be parsed from fieldBegin command"
    );
}

#[test]
fn reader_non_hyperlink_field_does_not_set_link() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
        xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
  <hp:p id="0" paraPrIDRef="0">
    <hp:run charPrIDRef="0">
      <hp:fieldBegin type="OTHER" command="something"/>
    </hp:run>
    <hp:run charPrIDRef="0">
      <hp:t>not a link</hp:t>
    </hp:run>
    <hp:run charPrIDRef="0">
      <hp:fieldEnd type="OTHER"/>
    </hp:run>
  </hp:p>
</hs:sec>"#;

    let section = read_hwpx_section_xml(xml);
    let inlines = match &section.blocks[0] {
        Block::Paragraph { inlines } => inlines,
        other => panic!("expected Paragraph, got: {other:?}"),
    };

    assert!(!inlines.is_empty());
    assert!(
        inlines[0].link.is_none(),
        "non-HYPERLINK field must not set link: {:?}",
        inlines[0]
    );
}

#[test]
fn reader_text_outside_hyperlink_has_no_link() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
        xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
  <hp:p id="0" paraPrIDRef="0">
    <hp:run charPrIDRef="0">
      <hp:t>before</hp:t>
    </hp:run>
    <hp:run charPrIDRef="0">
      <hp:fieldBegin type="HYPERLINK" command="https://example.com"/>
    </hp:run>
    <hp:run charPrIDRef="0">
      <hp:t>linked</hp:t>
    </hp:run>
    <hp:run charPrIDRef="0">
      <hp:fieldEnd type="HYPERLINK"/>
    </hp:run>
    <hp:run charPrIDRef="0">
      <hp:t>after</hp:t>
    </hp:run>
  </hp:p>
</hs:sec>"#;

    let section = read_hwpx_section_xml(xml);
    let inlines = match &section.blocks[0] {
        Block::Paragraph { inlines } => inlines,
        other => panic!("expected Paragraph, got: {other:?}"),
    };

    assert_eq!(inlines.len(), 3, "three inlines: before, linked, after");
    assert!(inlines[0].link.is_none(), "before must have no link");
    assert_eq!(inlines[0].text, "before");
    assert_eq!(
        inlines[1].link.as_deref(),
        Some("https://example.com"),
        "linked inline must have URL"
    );
    assert_eq!(inlines[1].text, "linked");
    assert!(inlines[2].link.is_none(), "after must have no link");
    assert_eq!(inlines[2].text, "after");
}
