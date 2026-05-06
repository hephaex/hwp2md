use super::*;

// ── Roundtrip: list content survives MD → HWPX ──────────────────────────────

/// An ordered list survives the write path: the text content of each item
/// is present in the HWPX section XML.
#[test]
fn roundtrip_ordered_list_text_preserved_in_hwpx() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::List {
                ordered: true,
                start: 1,
                items: vec![
                    ListItem {
                        blocks: vec![Block::Paragraph {
                            inlines: vec![inline("step one")],
                        }],
                        children: vec![],
                        checked: None,
                    },
                    ListItem {
                        blocks: vec![Block::Paragraph {
                            inlines: vec![inline("step two")],
                        }],
                        children: vec![],
                        checked: None,
                    },
                ],
            }],

            page_layout: None,
            ..Default::default()
        }],
        assets: Vec::new(),
    };
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");

    // Read back the section XML from the HWPX ZIP.
    let file = std::fs::File::open(tmp.path()).expect("open zip");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive
        .by_name("Contents/section0.xml")
        .expect("section0.xml must exist");
    let mut xml = String::new();
    std::io::Read::read_to_string(&mut entry, &mut xml).expect("read section0.xml");

    assert!(
        xml.contains("step one"),
        "ordered list item 'step one' must survive write: {xml}"
    );
    assert!(
        xml.contains("step two"),
        "ordered list item 'step two' must survive write: {xml}"
    );
    assert!(
        xml.contains(r#"numPrIDRef="1""#),
        "ordered list items must have numPrIDRef=\"1\" (DIGIT, id=1) in HWPX: {xml}"
    );
}

/// An unordered list survives the write path with bullet numPrIDRef.
#[test]
fn roundtrip_unordered_list_text_preserved_in_hwpx() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::List {
                ordered: false,
                start: 1,
                items: vec![
                    ListItem {
                        blocks: vec![Block::Paragraph {
                            inlines: vec![inline("alpha")],
                        }],
                        children: vec![],
                        checked: None,
                    },
                    ListItem {
                        blocks: vec![Block::Paragraph {
                            inlines: vec![inline("beta")],
                        }],
                        children: vec![],
                        checked: None,
                    },
                ],
            }],

            page_layout: None,
            ..Default::default()
        }],
        assets: Vec::new(),
    };
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");

    let file = std::fs::File::open(tmp.path()).expect("open zip");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive
        .by_name("Contents/section0.xml")
        .expect("section0.xml must exist");
    let mut xml = String::new();
    std::io::Read::read_to_string(&mut entry, &mut xml).expect("read section0.xml");

    assert!(
        xml.contains("alpha"),
        "unordered item 'alpha' must survive: {xml}"
    );
    assert!(
        xml.contains("beta"),
        "unordered item 'beta' must survive: {xml}"
    );
    assert!(
        !xml.contains("numPrIDRef"),
        "unordered list items must NOT carry numPrIDRef in HWPX (no BULLET numbering): {xml}"
    );
}

/// header.xml in the HWPX ZIP must contain the numberingList element.
#[test]
fn roundtrip_header_xml_contains_numbering_list() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::List {
                ordered: false,
                start: 1,
                items: vec![ListItem {
                    blocks: vec![Block::Paragraph {
                        inlines: vec![inline("item")],
                    }],
                    children: vec![],
                    checked: None,
                }],
            }],

            page_layout: None,
            ..Default::default()
        }],
        assets: Vec::new(),
    };
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");

    let file = std::fs::File::open(tmp.path()).expect("open zip");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive
        .by_name("Contents/header.xml")
        .expect("header.xml must exist");
    let mut header = String::new();
    std::io::Read::read_to_string(&mut entry, &mut header).expect("read header.xml");

    assert!(
        header.contains("<hh:numberings"),
        "header.xml in HWPX must contain <hh:numberings>:\n{header}"
    );
    // Only DIGIT numbering is registered; BULLET is not a valid OWPML numFormat.
    assert!(
        !header.contains(r#"numFormat="BULLET""#),
        "header.xml must NOT contain BULLET numbering (invalid in OWPML schema):\n{header}"
    );
    assert!(
        header.contains(r#"numFormat="DIGIT""#),
        "header.xml must contain DIGIT numbering:\n{header}"
    );
}

// ── Task list (checked/unchecked items) ─────────────────────────────────────

fn task_item(text: &str, checked: Option<bool>) -> ListItem {
    ListItem {
        blocks: vec![Block::Paragraph {
            inlines: vec![inline(text)],
        }],
        children: vec![],
        checked,
    }
}

#[test]
fn section_xml_unchecked_task_item_has_unchecked_checkbox_char() {
    let xml = section_xml(vec![Block::List {
        ordered: false,
        start: 1,
        items: vec![task_item("todo", Some(false))],
    }]);
    assert!(
        xml.contains("☐"),
        "unchecked task item must emit '☐' checkbox character:\n{xml}"
    );
    assert!(xml.contains("todo"), "item text must be present: {xml}");
}

#[test]
fn section_xml_checked_task_item_has_checked_checkbox_char() {
    let xml = section_xml(vec![Block::List {
        ordered: false,
        start: 1,
        items: vec![task_item("done", Some(true))],
    }]);
    assert!(
        xml.contains("☑"),
        "checked task item must emit '☑' checkbox character:\n{xml}"
    );
    assert!(xml.contains("done"), "item text must be present: {xml}");
}

#[test]
fn section_xml_normal_item_has_no_checkbox_char() {
    let xml = section_xml(vec![Block::List {
        ordered: false,
        start: 1,
        items: vec![task_item("plain", None)],
    }]);
    assert!(
        !xml.contains("☐") && !xml.contains("☑"),
        "normal list item must NOT emit a checkbox character:\n{xml}"
    );
    assert!(xml.contains("plain"), "item text must be present: {xml}");
}

#[test]
fn section_xml_task_list_mixed_items() {
    let xml = section_xml(vec![Block::List {
        ordered: false,
        start: 1,
        items: vec![
            task_item("done", Some(true)),
            task_item("todo", Some(false)),
            task_item("plain", None),
        ],
    }]);
    assert!(xml.contains("☑"), "checked item must have ☑: {xml}");
    assert!(xml.contains("☐"), "unchecked item must have ☐: {xml}");
    assert!(xml.contains("done"), "done text: {xml}");
    assert!(xml.contains("todo"), "todo text: {xml}");
    assert!(xml.contains("plain"), "plain text: {xml}");
    // Plain item must not duplicate checkbox chars — count occurrences
    let checked_count = xml.matches('☑').count();
    let unchecked_count = xml.matches('☐').count();
    assert_eq!(
        checked_count, 1,
        "exactly one ☑ for one checked item: {xml}"
    );
    assert_eq!(
        unchecked_count, 1,
        "exactly one ☐ for one unchecked item: {xml}"
    );
}
