use super::*;
use crate::ir::{self, Block, Document, Inline, Metadata, Section};

// ── helpers ───────────────────────────────────────────────────────────────

fn section_xml_with_hf(section: Section) -> String {
    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![section.clone()],
        assets: Vec::new(),
    };
    let tables = RefTables::build(&doc, None);
    let empty_asset_map = ImageAssetMap::new();
    generate_section_xml(&section, 0, &tables, &empty_asset_map)
        .expect("generate_section_xml failed")
}

fn plain_para(text: &str) -> Block {
    Block::Paragraph {
        inlines: vec![Inline::plain(text)],
    }
}

// ── tests ─────────────────────────────────────────────────────────────────

#[test]
fn header_footer_emitted_in_section_xml() {
    let sec = Section {
        blocks: vec![plain_para("body")],
        page_layout: None,
        header: Some(vec![plain_para("Header line")]),
        footer: Some(vec![plain_para("Footer line")]),
        header_footer_type: None,
    };
    let xml = section_xml_with_hf(sec);

    // Both wrapper elements must be present.
    assert!(
        xml.contains("<hp:headerFooter>"),
        "hp:headerFooter open tag missing: {xml}"
    );
    assert!(
        xml.contains("</hp:headerFooter>"),
        "hp:headerFooter close tag missing: {xml}"
    );
    assert!(
        xml.contains("<hp:header>"),
        "hp:header open tag missing: {xml}"
    );
    assert!(
        xml.contains("</hp:header>"),
        "hp:header close tag missing: {xml}"
    );
    assert!(
        xml.contains("<hp:footer>"),
        "hp:footer open tag missing: {xml}"
    );
    assert!(
        xml.contains("</hp:footer>"),
        "hp:footer close tag missing: {xml}"
    );

    // Content must be present.
    assert!(xml.contains("Header line"), "header text missing: {xml}");
    assert!(xml.contains("Footer line"), "footer text missing: {xml}");
    assert!(xml.contains("body"), "body text missing: {xml}");
}

#[test]
fn header_footer_emitted_before_sec_pr() {
    // OWPML convention: headerFooter should appear before secPr.
    let sec = Section {
        blocks: vec![],
        page_layout: None,
        header: Some(vec![plain_para("hdr")]),
        footer: None,
        header_footer_type: None,
    };
    let xml = section_xml_with_hf(sec);

    let hf_pos = xml
        .find("<hp:headerFooter>")
        .expect("hp:headerFooter missing");
    let sec_pr_pos = xml.find("<hp:secPr>").expect("hp:secPr missing");
    assert!(
        hf_pos < sec_pr_pos,
        "headerFooter must appear before secPr: {xml}"
    );
}

#[test]
fn no_header_footer_when_both_none() {
    // When header and footer are both None, no headerFooter element must be emitted.
    let sec = Section {
        blocks: vec![plain_para("body")],
        page_layout: None,
        header: None,
        footer: None,
        header_footer_type: None,
    };
    let xml = section_xml_with_hf(sec);

    assert!(
        !xml.contains("<hp:headerFooter>"),
        "no headerFooter must be emitted when both are None: {xml}"
    );
}

#[test]
fn no_header_footer_when_both_empty_vecs() {
    // When header/footer are Some([]) they contain no blocks, so the element
    // must be omitted (is_empty guard).
    let sec = Section {
        blocks: vec![plain_para("body")],
        page_layout: None,
        header: Some(vec![]),
        footer: Some(vec![]),
        header_footer_type: None,
    };
    let xml = section_xml_with_hf(sec);

    assert!(
        !xml.contains("<hp:headerFooter>"),
        "no headerFooter must be emitted when both vecs are empty: {xml}"
    );
}

#[test]
fn header_only_no_footer_element() {
    // When only header is present, no <hp:footer> element must be emitted.
    let sec = Section {
        blocks: vec![],
        page_layout: None,
        header: Some(vec![plain_para("header only")]),
        footer: None,
        header_footer_type: None,
    };
    let xml = section_xml_with_hf(sec);

    assert!(
        xml.contains("<hp:header>"),
        "hp:header must be emitted: {xml}"
    );
    assert!(
        !xml.contains("<hp:footer>"),
        "hp:footer must NOT be emitted when footer is None: {xml}"
    );
    assert!(xml.contains("header only"), "header text missing: {xml}");
}

#[test]
fn header_footer_type_both() {
    // Test that type="both" is emitted correctly.
    let sec = Section {
        blocks: vec![plain_para("body")],
        page_layout: None,
        header: Some(vec![plain_para("Header")]),
        footer: None,
        header_footer_type: Some(ir::HeaderFooterType::Both),
    };
    let xml = section_xml_with_hf(sec);

    assert!(
        xml.contains(r#"<hp:headerFooter type="both">"#) || xml.contains(r#"type="both"#),
        "type=\"both\" attribute must be present: {xml}"
    );
}

#[test]
fn header_footer_type_even() {
    // Test that type="even" is emitted correctly.
    let sec = Section {
        blocks: vec![plain_para("body")],
        page_layout: None,
        header: Some(vec![plain_para("Header")]),
        footer: Some(vec![plain_para("Footer")]),
        header_footer_type: Some(ir::HeaderFooterType::Even),
    };
    let xml = section_xml_with_hf(sec);

    assert!(
        xml.contains(r#"type="even"#),
        "type=\"even\" attribute must be present: {xml}"
    );
}

#[test]
fn header_footer_type_odd() {
    // Test that type="odd" is emitted correctly.
    let sec = Section {
        blocks: vec![plain_para("body")],
        page_layout: None,
        header: Some(vec![plain_para("Header")]),
        footer: Some(vec![plain_para("Footer")]),
        header_footer_type: Some(ir::HeaderFooterType::Odd),
    };
    let xml = section_xml_with_hf(sec);

    assert!(
        xml.contains(r#"type="odd"#),
        "type=\"odd\" attribute must be present: {xml}"
    );
}

#[test]
fn header_footer_type_none_not_emitted() {
    // Test that when type is None, no type attribute is emitted.
    let sec = Section {
        blocks: vec![plain_para("body")],
        page_layout: None,
        header: Some(vec![plain_para("Header")]),
        footer: None,
        header_footer_type: None,
    };
    let xml = section_xml_with_hf(sec);

    assert!(
        !xml.contains(r#"type="#),
        "no type attribute should be present when header_footer_type is None: {xml}"
    );
}
