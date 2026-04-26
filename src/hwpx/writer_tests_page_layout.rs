use super::*;
use crate::ir::{Block, Inline, PageLayout, Section};

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
        }],
        assets: Vec::new(),
    };
    let tables = RefTables::build(&doc);
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
