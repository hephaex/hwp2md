use super::*;

// ── Phase C-1 tests: multiple paraPr entries for paragraph styling ─────────
//
// These tests verify:
//   1. The header emits exactly 5 paraPr entries (id=0..=4).
//   2. Entries 0-3 retain their previous semantics (regression guard).
//   3. Entry id=4 (heading) has 180% line spacing and left margin 0.
//   4. Normal paragraphs (id=0) still use 160% line spacing.
//   5. Headings outside a blockquote use paraPrIDRef="4".
//   6. Headings inside a blockquote still use paraPrIDRef="1".
//   7. The OWPML `<hh:intent>` attribute correctly reflects first-line indent.

// ── Header: five paraPr entries emitted ────────────────────────────────────

/// header.xml must declare exactly 5 paraProperties entries after Phase C-1.
#[test]
fn header_xml_para_properties_item_cnt_is_five() {
    let doc = doc_with_section(vec![]);
    let tables = RefTables::build(&doc);
    let header =
        super::header::generate_header_xml(&doc, &tables).expect("generate_header_xml failed");

    assert!(
        header.contains(r#"<hh:paraProperties itemCnt="5""#),
        "paraProperties must have itemCnt=\"5\" after Phase C-1:\n{header}"
    );
}

/// All five paraPr IDs (0-4) must be present in the header.
#[test]
fn header_xml_all_five_para_pr_ids_present() {
    let doc = doc_with_section(vec![]);
    let tables = RefTables::build(&doc);
    let header =
        super::header::generate_header_xml(&doc, &tables).expect("generate_header_xml failed");

    for id in 0..=4u8 {
        let marker = format!(r#"<hh:paraPr id="{id}">"#);
        assert!(
            header.contains(&marker),
            "paraPr id={id} must exist in header:\n{header}"
        );
    }
}

// ── Regression: existing paraPr entries 0-3 unchanged ──────────────────────

/// paraPr id=0 (default) must have left margin 0 and 160% line spacing.
#[test]
fn header_xml_para_pr_0_default_margins_and_spacing() {
    let doc = doc_with_section(vec![]);
    let tables = RefTables::build(&doc);
    let header =
        super::header::generate_header_xml(&doc, &tables).expect("generate_header_xml failed");

    // Locate id=0 and verify it is followed by left value="0".
    let id0_pos = header
        .find(r#"<hh:paraPr id="0">"#)
        .expect("paraPr id=0 must exist");

    // The first hh:left after id=0 belongs to paraPr id=0.
    let slice = &header[id0_pos..];
    assert!(
        slice.contains(r#"<hh:left value="0"/>"#),
        "paraPr id=0 must have left margin 0:\n{slice}"
    );
    // 160% line spacing.
    assert!(
        slice.contains(r#"<hh:lineSpacing type="PERCENT" value="160"/>"#),
        "paraPr id=0 must have 160% line spacing:\n{slice}"
    );
}

/// paraPr id=1 (blockquote) must have left margin 800 and 160% line spacing.
#[test]
fn header_xml_para_pr_1_blockquote_left_800_and_160_spacing() {
    let doc = doc_with_section(vec![]);
    let tables = RefTables::build(&doc);
    let header =
        super::header::generate_header_xml(&doc, &tables).expect("generate_header_xml failed");

    let id1_pos = header
        .find(r#"<hh:paraPr id="1">"#)
        .expect("paraPr id=1 must exist");
    let slice = &header[id1_pos..];

    assert!(
        slice.contains(r#"<hh:left value="800"/>"#),
        "paraPr id=1 (blockquote) must have left margin 800:\n{slice}"
    );
    assert!(
        slice.contains(r#"<hh:lineSpacing type="PERCENT" value="160"/>"#),
        "paraPr id=1 must have 160% line spacing:\n{slice}"
    );
}

/// paraPr id=2 (list depth-0) must have left margin 400 and 160% line spacing.
#[test]
fn header_xml_para_pr_2_list_d0_left_400_and_160_spacing() {
    let doc = doc_with_section(vec![]);
    let tables = RefTables::build(&doc);
    let header =
        super::header::generate_header_xml(&doc, &tables).expect("generate_header_xml failed");

    let id2_pos = header
        .find(r#"<hh:paraPr id="2">"#)
        .expect("paraPr id=2 must exist");
    let slice = &header[id2_pos..];

    assert!(
        slice.contains(r#"<hh:left value="400"/>"#),
        "paraPr id=2 (list D0) must have left margin 400:\n{slice}"
    );
    assert!(
        slice.contains(r#"<hh:lineSpacing type="PERCENT" value="160"/>"#),
        "paraPr id=2 must have 160% line spacing:\n{slice}"
    );
}

/// paraPr id=3 (list depth-1+) must have left margin 800 and 160% line spacing.
#[test]
fn header_xml_para_pr_3_list_d1_left_800_and_160_spacing() {
    let doc = doc_with_section(vec![]);
    let tables = RefTables::build(&doc);
    let header =
        super::header::generate_header_xml(&doc, &tables).expect("generate_header_xml failed");

    let id3_pos = header
        .find(r#"<hh:paraPr id="3">"#)
        .expect("paraPr id=3 must exist");
    let slice = &header[id3_pos..];

    assert!(
        slice.contains(r#"<hh:left value="800"/>"#),
        "paraPr id=3 (list D1+) must have left margin 800:\n{slice}"
    );
    assert!(
        slice.contains(r#"<hh:lineSpacing type="PERCENT" value="160"/>"#),
        "paraPr id=3 must have 160% line spacing:\n{slice}"
    );
}

// ── New paraPr id=4 (heading) ───────────────────────────────────────────────

/// paraPr id=4 (heading) must have left margin 0 and 180% line spacing.
#[test]
fn header_xml_para_pr_4_heading_left_0_and_180_spacing() {
    let doc = doc_with_section(vec![]);
    let tables = RefTables::build(&doc);
    let header =
        super::header::generate_header_xml(&doc, &tables).expect("generate_header_xml failed");

    let id4_pos = header
        .find(r#"<hh:paraPr id="4">"#)
        .expect("paraPr id=4 must exist");
    let slice = &header[id4_pos..];

    assert!(
        slice.contains(r#"<hh:left value="0"/>"#),
        "paraPr id=4 (heading) must have left margin 0:\n{slice}"
    );
    assert!(
        slice.contains(r#"<hh:lineSpacing type="PERCENT" value="180"/>"#),
        "paraPr id=4 (heading) must have 180% line spacing:\n{slice}"
    );
}

/// paraPr id=4 must have first-line indent of 0 (hh:intent value="0").
#[test]
fn header_xml_para_pr_4_heading_first_indent_zero() {
    let doc = doc_with_section(vec![]);
    let tables = RefTables::build(&doc);
    let header =
        super::header::generate_header_xml(&doc, &tables).expect("generate_header_xml failed");

    let id4_pos = header
        .find(r#"<hh:paraPr id="4">"#)
        .expect("paraPr id=4 must exist");
    let slice = &header[id4_pos..];

    assert!(
        slice.contains(r#"<hh:intent value="0"/>"#),
        "paraPr id=4 must have first-line indent 0 via hh:intent:\n{slice}"
    );
}

/// paraPr id=4 (heading) must appear after id=3 in document order.
#[test]
fn header_xml_para_pr_4_appears_after_id_3() {
    let doc = doc_with_section(vec![]);
    let tables = RefTables::build(&doc);
    let header =
        super::header::generate_header_xml(&doc, &tables).expect("generate_header_xml failed");

    let id3_pos = header
        .find(r#"<hh:paraPr id="3">"#)
        .expect("paraPr id=3 must exist");
    let id4_pos = header
        .find(r#"<hh:paraPr id="4">"#)
        .expect("paraPr id=4 must exist");

    assert!(
        id3_pos < id4_pos,
        "paraPr id=4 must appear after id=3 in header XML (got id3={id3_pos}, id4={id4_pos})"
    );
}

// ── Section XML: heading uses paraPrIDRef="4" ────────────────────────────────

/// A heading paragraph outside any blockquote must use paraPrIDRef="4".
#[test]
fn section_xml_heading_outside_blockquote_uses_para_pr_id_ref_4() {
    for level in 1u8..=6 {
        let xml = section_xml(vec![Block::Heading {
            level,
            inlines: vec![Inline::plain("title")],
        }]);
        assert!(
            xml.contains(r#"paraPrIDRef="4""#),
            "H{level} outside blockquote must use paraPrIDRef=\"4\":\n{xml}"
        );
        assert!(
            !xml.contains(r#"paraPrIDRef="0""#),
            "H{level} must NOT use paraPrIDRef=\"0\" (default para):\n{xml}"
        );
    }
}

/// A heading inside a blockquote must still use paraPrIDRef="1" (indented),
/// NOT paraPrIDRef="4" (heading style).
#[test]
fn section_xml_heading_inside_blockquote_uses_para_pr_id_ref_1() {
    let xml = section_xml(vec![Block::BlockQuote {
        blocks: vec![Block::Heading {
            level: 2,
            inlines: vec![Inline::plain("quoted heading")],
        }],
    }]);

    assert!(
        xml.contains(r#"paraPrIDRef="1""#),
        "heading inside blockquote must use paraPrIDRef=\"1\":\n{xml}"
    );
    assert!(
        !xml.contains(r#"paraPrIDRef="4""#),
        "heading inside blockquote must NOT use paraPrIDRef=\"4\":\n{xml}"
    );
    assert!(
        xml.contains("quoted heading"),
        "heading text must be preserved:\n{xml}"
    );
}

/// A mixed document with heading + normal paragraph must produce both
/// paraPrIDRef="4" (for the heading) and paraPrIDRef="0" (for the paragraph).
#[test]
fn section_xml_heading_and_paragraph_use_distinct_para_pr_ids() {
    let xml = section_xml(vec![
        Block::Heading {
            level: 1,
            inlines: vec![Inline::plain("Title")],
        },
        Block::Paragraph {
            inlines: vec![Inline::plain("Body text.")],
        },
    ]);

    assert!(
        xml.contains(r#"paraPrIDRef="4""#),
        "heading must use paraPrIDRef=\"4\":\n{xml}"
    );
    assert!(
        xml.contains(r#"paraPrIDRef="0""#),
        "normal paragraph must use paraPrIDRef=\"0\":\n{xml}"
    );
}

/// All six heading levels use paraPrIDRef="4" (not just level 1).
#[test]
fn section_xml_all_heading_levels_use_para_pr_id_ref_4() {
    let xml = section_xml(vec![
        Block::Heading {
            level: 1,
            inlines: vec![Inline::plain("H1")],
        },
        Block::Heading {
            level: 2,
            inlines: vec![Inline::plain("H2")],
        },
        Block::Heading {
            level: 3,
            inlines: vec![Inline::plain("H3")],
        },
        Block::Heading {
            level: 4,
            inlines: vec![Inline::plain("H4")],
        },
        Block::Heading {
            level: 5,
            inlines: vec![Inline::plain("H5")],
        },
        Block::Heading {
            level: 6,
            inlines: vec![Inline::plain("H6")],
        },
    ]);

    let count = xml.matches(r#"paraPrIDRef="4""#).count();
    assert_eq!(
        count, 6,
        "all six heading levels must each produce paraPrIDRef=\"4\"; found {count}:\n{xml}"
    );
}

// ── Schema validation: paraPr id=4 passes polaris_dvc ───────────────────────
// (Integration test lives in tests/hwpx_validation.rs; here we just verify
//  the XML structure is well-formed by roundtripping through the reader.)

/// A document with a heading survives MD → HWPX write with no panic and the
/// heading text is recoverable from the written ZIP bytes.
#[test]
fn section_xml_heading_para_pr_4_present_in_zip() {
    use std::io::Read as _;

    let doc = doc_with_section(vec![
        Block::Heading {
            level: 1,
            inlines: vec![Inline::plain("Main Title")],
        },
        Block::Paragraph {
            inlines: vec![Inline::plain("Body paragraph.")],
        },
    ]);

    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");

    let file = std::fs::File::open(tmp.path()).expect("open zip");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");

    // Verify section XML carries paraPrIDRef="4" for the heading.
    let mut section_xml_str = String::new();
    archive
        .by_name("Contents/section0.xml")
        .expect("section0.xml must exist")
        .read_to_string(&mut section_xml_str)
        .expect("read section0.xml");

    assert!(
        section_xml_str.contains(r#"paraPrIDRef="4""#),
        "section0.xml must carry paraPrIDRef=\"4\" for heading:\n{section_xml_str}"
    );

    // Verify header XML has paraPr id=4.
    let mut header_xml_str = String::new();
    archive
        .by_name("Contents/header.xml")
        .expect("header.xml must exist")
        .read_to_string(&mut header_xml_str)
        .expect("read header.xml");

    assert!(
        header_xml_str.contains(r#"<hh:paraPr id="4">"#),
        "header.xml must contain <hh:paraPr id=\"4\">:\n{header_xml_str}"
    );
    assert!(
        header_xml_str.contains(r#"value="180""#),
        "header.xml paraPr id=4 must have 180% line spacing:\n{header_xml_str}"
    );
}
