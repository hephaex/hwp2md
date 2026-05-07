use super::*;
use crate::ir::{Block, Inline, PageLayout, Section};
use crate::style::StyleTemplate;

// ── Phase B-4 tests: page layout (secPr) emission ─────────────────────

/// Helper: generate section XML for a section with the given page layout.
fn section_xml_with_layout(layout: Option<PageLayout>) -> String {
    let blocks = vec![Block::Paragraph {
        inlines: vec![Inline::plain("test")],
    }];
    let doc = crate::ir::Document {
        metadata: crate::ir::Metadata::default(),
        sections: vec![Section {
            blocks: blocks.clone(),
            page_layout: layout,
            ..Default::default()
        }],
        assets: Vec::new(),
    };
    let tables = RefTables::build(&doc, None);
    let sec = &doc.sections[0];
    let empty_asset_map = ImageAssetMap::new();
    generate_section_xml(sec, 0, &tables, &empty_asset_map).expect("generate_section_xml failed")
}

#[test]
fn section_xml_with_no_page_layout_emits_a4_defaults() {
    // When page_layout is None the writer falls back to A4 portrait defaults.
    let xml = section_xml_with_layout(None);
    assert!(
        xml.contains("<hp:secPr>"),
        "secPr element must be present: {xml}"
    );
    assert!(xml.contains("<hp:pagePr"), "pagePr must be present: {xml}");
    // A4 portrait: width=59528 height=84188 landscape=false
    assert!(
        xml.contains(r#"width="59528""#),
        "A4 width must be 59528: {xml}"
    );
    assert!(
        xml.contains(r#"height="84188""#),
        "A4 height must be 84188: {xml}"
    );
    assert!(
        xml.contains(r#"landscape="false""#),
        "portrait: landscape must be false: {xml}"
    );
    // Default margins: left=5670 right=5670
    assert!(
        xml.contains(r#"left="5670""#),
        "default left margin must be 5670: {xml}"
    );
    assert!(
        xml.contains(r#"right="5670""#),
        "default right margin must be 5670: {xml}"
    );
}

#[test]
fn section_xml_with_a4_portrait_layout() {
    let layout = PageLayout::a4_portrait();
    let xml = section_xml_with_layout(Some(layout));
    assert!(xml.contains("<hp:secPr>"), "secPr must be present: {xml}");
    assert!(xml.contains(r#"width="59528""#), "A4 width: {xml}");
    assert!(xml.contains(r#"height="84188""#), "A4 height: {xml}");
    assert!(xml.contains(r#"landscape="false""#), "portrait: {xml}");
    assert!(xml.contains(r#"left="5670""#), "left margin: {xml}");
    assert!(xml.contains(r#"right="5670""#), "right margin: {xml}");
    assert!(xml.contains(r#"top="4252""#), "top margin: {xml}");
    assert!(xml.contains(r#"bottom="4252""#), "bottom margin: {xml}");
}

#[test]
fn section_xml_with_landscape_layout() {
    let layout = PageLayout {
        width: Some(84188),
        height: Some(59528),
        landscape: true,
        margin_left: Some(4000),
        margin_right: Some(4000),
        margin_top: Some(3000),
        margin_bottom: Some(3000),
    };
    let xml = section_xml_with_layout(Some(layout));
    assert!(
        xml.contains(r#"landscape="true""#),
        "landscape must be true: {xml}"
    );
    assert!(xml.contains(r#"width="84188""#), "landscape width: {xml}");
    assert!(xml.contains(r#"height="59528""#), "landscape height: {xml}");
    assert!(xml.contains(r#"left="4000""#), "custom left margin: {xml}");
    assert!(xml.contains(r#"top="3000""#), "custom top margin: {xml}");
}

#[test]
fn section_xml_secpr_appears_before_paragraph_content() {
    // The <hp:secPr> element must be emitted before any <hp:p> paragraph.
    let xml = section_xml_with_layout(None);
    let sec_pr_pos = xml.find("<hp:secPr>").expect("secPr position");
    let para_pos = xml.find("<hp:p ").expect("paragraph position");
    assert!(
        sec_pr_pos < para_pos,
        "secPr must precede paragraph content: {xml}"
    );
}

#[test]
fn section_xml_secpr_has_margin_element() {
    let xml = section_xml_with_layout(None);
    assert!(
        xml.contains("<hp:margin"),
        "margin element must be present: {xml}"
    );
    assert!(
        xml.contains("<hp:pageSize"),
        "pageSize element must be present: {xml}"
    );
}

#[test]
fn section_xml_custom_margins_are_emitted() {
    let layout = PageLayout {
        width: Some(59528),
        height: Some(84188),
        landscape: false,
        margin_left: Some(1000),
        margin_right: Some(2000),
        margin_top: Some(3000),
        margin_bottom: Some(4000),
    };
    let xml = section_xml_with_layout(Some(layout));
    assert!(xml.contains(r#"left="1000""#), "left margin 1000: {xml}");
    assert!(xml.contains(r#"right="2000""#), "right margin 2000: {xml}");
    assert!(xml.contains(r#"top="3000""#), "top margin 3000: {xml}");
    assert!(
        xml.contains(r#"bottom="4000""#),
        "bottom margin 4000: {xml}"
    );
}

// ── Style template integration tests ─────────────────────────────────

fn section_xml_with_style(style_yaml: &str) -> String {
    let blocks = vec![Block::Paragraph {
        inlines: vec![Inline::plain("test")],
    }];
    let doc = crate::ir::Document {
        metadata: crate::ir::Metadata::default(),
        sections: vec![Section {
            blocks: blocks.clone(),
            page_layout: None,
            ..Default::default()
        }],
        assets: Vec::new(),
    };
    let template = StyleTemplate::from_yaml(style_yaml).unwrap();
    let tables = RefTables::build(&doc, Some(template));
    let sec = &doc.sections[0];
    let empty_asset_map = ImageAssetMap::new();
    generate_section_xml(sec, 0, &tables, &empty_asset_map).expect("generate_section_xml failed")
}

#[test]
fn style_template_overrides_page_dimensions() {
    let xml = section_xml_with_style(
        r"
page:
  width: 70000
  height: 90000
  landscape: true
  margin:
    left: 3000
    right: 3000
    top: 2000
    bottom: 2000
",
    );
    assert!(xml.contains(r#"width="70000""#), "custom width: {xml}");
    assert!(xml.contains(r#"height="90000""#), "custom height: {xml}");
    assert!(xml.contains(r#"landscape="true""#), "landscape: {xml}");
    assert!(xml.contains(r#"left="3000""#), "custom left margin: {xml}");
    assert!(xml.contains(r#"top="2000""#), "custom top margin: {xml}");
}

#[test]
fn style_template_partial_overrides_keep_defaults() {
    let xml = section_xml_with_style(
        r"
page:
  width: 70000
  margin:
    left: 3000
",
    );
    assert!(xml.contains(r#"width="70000""#), "custom width: {xml}");
    assert!(
        xml.contains(r#"height="84188""#),
        "default height preserved: {xml}"
    );
    assert!(xml.contains(r#"left="3000""#), "custom left margin: {xml}");
    assert!(
        xml.contains(r#"right="5670""#),
        "default right margin preserved: {xml}"
    );
}

#[test]
fn style_template_custom_code_font_in_header() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = crate::ir::Document {
        metadata: crate::ir::Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline {
                    text: "code".into(),
                    code: true,
                    ..Inline::default()
                }],
            }],
            page_layout: None,
            ..Default::default()
        }],
        assets: Vec::new(),
    };
    let style_yaml = r#"font:
  code: "D2Coding"
"#;
    let style_path = tempfile::NamedTempFile::new().expect("style tmp");
    std::fs::write(style_path.path(), style_yaml).unwrap();
    write_hwpx(&doc, tmp.path(), Some(style_path.path())).expect("write_hwpx");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive.by_name("Contents/header.xml").expect("header.xml");
    let mut content = String::new();
    std::io::Read::read_to_string(&mut entry, &mut content).expect("read");

    assert!(
        content.contains("D2Coding"),
        "custom code font D2Coding must appear in header: {content}"
    );
}

#[test]
fn style_template_heading_line_spacing_in_header() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = crate::ir::Document {
        metadata: crate::ir::Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::plain("Title")],
            }],
            page_layout: None,
            ..Default::default()
        }],
        assets: Vec::new(),
    };
    let style_yaml = r"heading:
  line_spacing: 220
";
    let style_path = tempfile::NamedTempFile::new().expect("style tmp");
    std::fs::write(style_path.path(), style_yaml).unwrap();
    write_hwpx(&doc, tmp.path(), Some(style_path.path())).expect("write_hwpx");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive.by_name("Contents/header.xml").expect("header.xml");
    let mut content = String::new();
    std::io::Read::read_to_string(&mut entry, &mut content).expect("read");

    assert!(
        content.contains(r#"value="220""#),
        "heading line spacing 220 must appear in header: {content}"
    );
}

#[test]
fn style_template_default_font_in_header() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = crate::ir::Document {
        metadata: crate::ir::Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::plain("text")],
            }],
            page_layout: None,
            ..Default::default()
        }],
        assets: Vec::new(),
    };
    let style_yaml = r#"font:
  default: "맑은 고딕"
"#;
    let style_path = tempfile::NamedTempFile::new().expect("style tmp");
    std::fs::write(style_path.path(), style_yaml).unwrap();
    write_hwpx(&doc, tmp.path(), Some(style_path.path())).expect("write_hwpx");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive.by_name("Contents/header.xml").expect("header.xml");
    let mut content = String::new();
    std::io::Read::read_to_string(&mut entry, &mut content).expect("read");

    assert!(
        content.contains("맑은 고딕"),
        "custom default font must appear in header: {content}"
    );
}

#[test]
fn section_page_layout_takes_precedence_over_style() {
    let custom_layout = PageLayout {
        width: Some(42000),
        height: Some(60000),
        landscape: false,
        margin_left: Some(1111),
        margin_right: Some(2222),
        margin_top: Some(3333),
        margin_bottom: Some(4444),
    };
    let blocks = vec![Block::Paragraph {
        inlines: vec![Inline::plain("test")],
    }];
    let doc = crate::ir::Document {
        metadata: crate::ir::Metadata::default(),
        sections: vec![Section {
            blocks,
            page_layout: Some(custom_layout),
            ..Default::default()
        }],
        assets: Vec::new(),
    };
    let style = StyleTemplate::from_yaml(
        r"
page:
  width: 70000
  height: 90000
  margin:
    left: 9999
",
    )
    .unwrap();
    let tables = RefTables::build(&doc, Some(style));
    let sec = &doc.sections[0];
    let empty_asset_map = ImageAssetMap::new();
    let xml =
        generate_section_xml(sec, 0, &tables, &empty_asset_map).expect("generate_section_xml");

    assert!(
        xml.contains(r#"width="42000""#),
        "section layout width must win over style: {xml}"
    );
    assert!(
        xml.contains(r#"left="1111""#),
        "section layout margin must win over style: {xml}"
    );
    assert!(
        !xml.contains(r#"width="70000""#),
        "style template width must NOT appear: {xml}"
    );
}
