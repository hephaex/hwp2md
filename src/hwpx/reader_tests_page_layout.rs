use super::parse_section_xml;
use crate::ir::PageLayout;

// ── Phase B-4 tests: page layout (secPr) reader ──────────────────────

fn section(xml: &str) -> crate::ir::Section {
    parse_section_xml(xml).expect("parse_section_xml must not fail")
}

#[test]
fn section_without_sec_pr_has_no_page_layout() {
    // A minimal section XML with no <hp:secPr> element must produce
    // page_layout = None.
    let xml = r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
                         xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
        <hp:p><hp:run><hp:t>Hello</hp:t></hp:run></hp:p>
    </hs:sec>"#;
    let s = section(xml);
    assert!(
        s.page_layout.is_none(),
        "section without secPr must have page_layout=None, got: {:?}",
        s.page_layout
    );
}

#[test]
fn section_with_a4_portrait_sec_pr() {
    // A typical Korean HWPX A4 portrait section.
    let xml = r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
                         xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
        <hp:secPr>
            <hp:pagePr landscape="false">
                <hp:margin left="5670" right="5670" top="4252" bottom="4252"
                           header="4252" footer="4252" gutter="0"/>
                <hp:pageSize width="59528" height="84188"/>
            </hp:pagePr>
        </hp:secPr>
        <hp:p><hp:run><hp:t>Body text</hp:t></hp:run></hp:p>
    </hs:sec>"#;
    let s = section(xml);
    let layout = s
        .page_layout
        .as_ref()
        .expect("secPr must produce page_layout");
    assert_eq!(layout.width, Some(59528), "A4 width");
    assert_eq!(layout.height, Some(84188), "A4 height");
    assert!(!layout.landscape, "portrait");
    assert_eq!(layout.margin_left, Some(5670), "left margin");
    assert_eq!(layout.margin_right, Some(5670), "right margin");
    assert_eq!(layout.margin_top, Some(4252), "top margin");
    assert_eq!(layout.margin_bottom, Some(4252), "bottom margin");
}

#[test]
fn section_with_landscape_sec_pr() {
    // Landscape A4 page (width and height swapped, landscape="true").
    let xml = r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
                         xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
        <hp:secPr>
            <hp:pagePr landscape="true">
                <hp:margin left="4000" right="4000" top="3000" bottom="3000"
                           header="0" footer="0" gutter="0"/>
                <hp:pageSize width="84188" height="59528"/>
            </hp:pagePr>
        </hp:secPr>
    </hs:sec>"#;
    let s = section(xml);
    let layout = s
        .page_layout
        .as_ref()
        .expect("secPr must produce page_layout");
    assert_eq!(layout.width, Some(84188), "landscape width");
    assert_eq!(layout.height, Some(59528), "landscape height");
    assert!(layout.landscape, "landscape flag");
    assert_eq!(layout.margin_left, Some(4000), "left margin");
    assert_eq!(layout.margin_top, Some(3000), "top margin");
}

#[test]
fn section_page_layout_survives_content() {
    // Page layout must be captured even when content blocks follow secPr.
    let xml = r#"<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
                         xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
        <hp:secPr>
            <hp:pagePr landscape="false">
                <hp:margin left="1234" right="5678" top="2345" bottom="6789"
                           header="0" footer="0" gutter="0"/>
                <hp:pageSize width="50000" height="70000"/>
            </hp:pagePr>
        </hp:secPr>
        <hp:p><hp:run><hp:t>First paragraph</hp:t></hp:run></hp:p>
        <hp:p><hp:run><hp:t>Second paragraph</hp:t></hp:run></hp:p>
    </hs:sec>"#;
    let s = section(xml);
    // Content must still parse correctly.
    assert_eq!(s.blocks.len(), 2, "two paragraphs must be present");
    // Page layout must be captured.
    let layout = s.page_layout.as_ref().expect("page_layout must be Some");
    assert_eq!(layout.width, Some(50000), "custom width");
    assert_eq!(layout.height, Some(70000), "custom height");
    assert_eq!(layout.margin_left, Some(1234), "custom left margin");
    assert_eq!(layout.margin_right, Some(5678), "custom right margin");
    assert_eq!(layout.margin_top, Some(2345), "custom top margin");
    assert_eq!(layout.margin_bottom, Some(6789), "custom bottom margin");
}

#[test]
fn page_layout_a4_portrait_default_values() {
    // Verify the constants of PageLayout::a4_portrait() are correct.
    let layout = PageLayout::a4_portrait();
    assert_eq!(layout.width, Some(59528), "A4 width HWP units");
    assert_eq!(layout.height, Some(84188), "A4 height HWP units");
    assert!(!layout.landscape, "portrait by default");
    assert_eq!(layout.margin_left, Some(5670), "default left margin");
    assert_eq!(layout.margin_right, Some(5670), "default right margin");
    assert_eq!(layout.margin_top, Some(4252), "default top margin");
    assert_eq!(layout.margin_bottom, Some(4252), "default bottom margin");
}
