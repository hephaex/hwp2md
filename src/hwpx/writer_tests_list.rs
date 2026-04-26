use super::*;

// ── Phase A-2: List writer — bullet / numbering markup ────────────────────
//
// These tests verify that Block::List items are emitted as OWPML paragraphs
// carrying `numPrIDRef` (numbering definition reference) and a list-specific
// `paraPrIDRef` (paragraph property reference for left-indentation), and that
// the header.xml contains the matching `<hh:numberingList>` definitions.

// ── Header: numberingList definitions ───────────────────────────────────────

/// header.xml must contain a `<hh:numberings>` element with exactly one
/// numbering definition (id=1 for ordered/digit lists).
///
/// Unordered (bullet) lists do not use a `<hh:numbering>` entry in the
/// OWPML schema because `numFormat="BULLET"` is not a valid enum value;
/// instead bullet-list paragraphs rely on the paragraph-property indent alone.
#[test]
fn header_xml_contains_numbering_list() {
    let doc = doc_with_section(vec![Block::List {
        ordered: false,
        start: 1,
        items: vec![ListItem {
            blocks: vec![Block::Paragraph {
                inlines: vec![inline("item")],
            }],
            children: vec![],
        }],
    }]);
    let tables = RefTables::build(&doc);
    let header =
        super::header::generate_header_xml(&doc, &tables).expect("generate_header_xml failed");

    assert!(
        header.contains("<hh:numberings"),
        "header.xml must contain <hh:numberings>:\n{header}"
    );
    assert!(
        header.contains("</hh:numberings>"),
        "header.xml must close </hh:numberings>:\n{header}"
    );
    assert!(
        header.contains(r#"<hh:numberings itemCnt="1""#),
        "numberings must have itemCnt=\"1\" (ordered only):\n{header}"
    );
}

/// header.xml must always contain the numbering definitions even when the
/// document contains no lists (definitions are emitted unconditionally).
#[test]
fn header_xml_numbering_list_always_present() {
    let doc = doc_with_section(vec![Block::Paragraph {
        inlines: vec![inline("no lists here")],
    }]);
    let tables = RefTables::build(&doc);
    let header =
        super::header::generate_header_xml(&doc, &tables).expect("generate_header_xml failed");

    assert!(
        header.contains("<hh:numberings"),
        "numberings must appear even in documents without lists:\n{header}"
    );
}

/// header.xml must have exactly one `<hh:numbering>` (id=1, DIGIT) since the
/// OWPML schema does not support `numFormat="BULLET"` in `<hh:numberings>`.
/// Unordered (bullet) lists emit paragraphs without a `numPrIDRef` attribute.
#[test]
fn header_xml_numbering_id1_is_bullet() {
    let doc = doc_with_section(vec![]);
    let tables = RefTables::build(&doc);
    let header =
        super::header::generate_header_xml(&doc, &tables).expect("generate_header_xml failed");

    // id=1 must be the DIGIT (ordered) numbering — bullet uses no numbering entry.
    let id1_pos = header
        .find(r#"<hh:numbering id="1""#)
        .expect("numbering id=1 must exist");

    // id=2 must NOT exist (only one numbering definition).
    assert!(
        !header.contains(r#"<hh:numbering id="2""#),
        "numbering id=2 must NOT exist; only one entry is registered:\n{header}"
    );

    let slice_id1 = &header[id1_pos..];
    assert!(
        slice_id1.contains(r#"numFormat="DIGIT""#),
        "numbering id=1 must have numFormat=\"DIGIT\":\n{slice_id1}"
    );
    assert!(
        slice_id1.contains("%d."),
        "numbering id=1 must contain \"%d.\" format string:\n{slice_id1}"
    );
}

/// header.xml must have the digit numbering (id=1) — the single registered
/// `<hh:numbering>` entry used for ordered lists.
///
/// The old "id=2" (digit) test is renamed to reflect the new numbering scheme
/// where ordered lists use id=1 (the only registered numbering).
#[test]
fn header_xml_numbering_id2_is_digit() {
    let doc = doc_with_section(vec![]);
    let tables = RefTables::build(&doc);
    let header =
        super::header::generate_header_xml(&doc, &tables).expect("generate_header_xml failed");

    // Only id=1 (DIGIT) is registered now.
    let id1_pos = header
        .find(r#"<hh:numbering id="1""#)
        .expect("numbering id=1 (DIGIT) must exist");

    let slice_id1 = &header[id1_pos..];
    assert!(
        slice_id1.contains(r#"numFormat="DIGIT""#),
        "numbering id=1 must have numFormat=\"DIGIT\":\n{slice_id1}"
    );
    assert!(
        slice_id1.contains("%d."),
        "numbering id=1 must contain \"%d.\" format string:\n{slice_id1}"
    );
}

/// header.xml paraProperties must have itemCnt="4" (id=0 default, id=1 blockquote,
/// id=2 list-depth-0, id=3 list-depth-1+).
#[test]
fn header_xml_para_properties_has_four_entries() {
    let doc = doc_with_section(vec![]);
    let tables = RefTables::build(&doc);
    let header =
        super::header::generate_header_xml(&doc, &tables).expect("generate_header_xml failed");

    assert!(
        header.contains(r#"<hh:paraProperties itemCnt="4""#),
        "paraProperties must have itemCnt=\"4\":\n{header}"
    );
    // All four IDs must be present.
    assert!(
        header.contains(r#"<hh:paraPr id="0""#),
        "paraPr id=0 must exist:\n{header}"
    );
    assert!(
        header.contains(r#"<hh:paraPr id="1""#),
        "paraPr id=1 must exist:\n{header}"
    );
    assert!(
        header.contains(r#"<hh:paraPr id="2""#),
        "paraPr id=2 must exist:\n{header}"
    );
    assert!(
        header.contains(r#"<hh:paraPr id="3""#),
        "paraPr id=3 must exist:\n{header}"
    );
}

// ── Section XML: unordered list ──────────────────────────────────────────────

/// An unordered list item paragraph must NOT carry a `numPrIDRef` attribute
/// because `numFormat="BULLET"` is not valid in the OWPML schema.
/// Indentation is conveyed via `paraPrIDRef` alone.
#[test]
fn section_xml_unordered_list_item_has_num_pr_id_ref_1() {
    let xml = section_xml(vec![Block::List {
        ordered: false,
        start: 1,
        items: vec![ListItem {
            blocks: vec![Block::Paragraph {
                inlines: vec![inline("bullet item")],
            }],
            children: vec![],
        }],
    }]);
    assert!(
        !xml.contains("numPrIDRef"),
        "unordered list item must NOT have a numPrIDRef attribute (no BULLET numbering):\n{xml}"
    );
    assert!(
        xml.contains("bullet item"),
        "item text must be present: {xml}"
    );
}

/// An unordered list item must use `paraPrIDRef="2"` (list-depth-0 indent).
#[test]
fn section_xml_unordered_list_item_has_para_pr_id_ref_2() {
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
    assert!(
        xml.contains(r#"paraPrIDRef="2""#),
        "unordered list item must have paraPrIDRef=\"2\":\n{xml}"
    );
    // Must NOT use the default paragraph style.
    assert!(
        !xml.contains(r#"paraPrIDRef="0""#),
        "list item must NOT use paraPrIDRef=\"0\":\n{xml}"
    );
}

/// Multiple unordered list items must each NOT carry a `numPrIDRef` attribute
/// because the OWPML schema has no valid bullet numFormat.
#[test]
fn section_xml_multiple_unordered_items_all_have_num_pr_id_ref_1() {
    let xml = section_xml(vec![Block::List {
        ordered: false,
        start: 1,
        items: vec![
            ListItem {
                blocks: vec![Block::Paragraph {
                    inlines: vec![inline("alpha")],
                }],
                children: vec![],
            },
            ListItem {
                blocks: vec![Block::Paragraph {
                    inlines: vec![inline("beta")],
                }],
                children: vec![],
            },
            ListItem {
                blocks: vec![Block::Paragraph {
                    inlines: vec![inline("gamma")],
                }],
                children: vec![],
            },
        ],
    }]);
    assert!(
        !xml.contains("numPrIDRef"),
        "unordered list items must NOT carry numPrIDRef (no BULLET numbering): {xml}"
    );
    assert!(xml.contains("alpha"), "{xml}");
    assert!(xml.contains("beta"), "{xml}");
    assert!(xml.contains("gamma"), "{xml}");
}

// ── Section XML: ordered list ─────────────────────────────────────────────────

/// An ordered list item paragraph must carry `numPrIDRef="1"` (digit, the
/// sole registered numbering definition in the OWPML schema).
#[test]
fn section_xml_ordered_list_item_has_num_pr_id_ref_2() {
    let xml = section_xml(vec![Block::List {
        ordered: true,
        start: 1,
        items: vec![ListItem {
            blocks: vec![Block::Paragraph {
                inlines: vec![inline("first")],
            }],
            children: vec![],
        }],
    }]);
    assert!(
        xml.contains(r#"numPrIDRef="1""#),
        "ordered list item must have numPrIDRef=\"1\" (DIGIT, id=1):\n{xml}"
    );
    assert!(xml.contains("first"), "item text must be present: {xml}");
}

/// An ordered list item must use `paraPrIDRef="2"` (list-depth-0 indent).
#[test]
fn section_xml_ordered_list_item_has_para_pr_id_ref_2() {
    let xml = section_xml(vec![Block::List {
        ordered: true,
        start: 1,
        items: vec![ListItem {
            blocks: vec![Block::Paragraph {
                inlines: vec![inline("numbered")],
            }],
            children: vec![],
        }],
    }]);
    assert!(
        xml.contains(r#"paraPrIDRef="2""#),
        "ordered list item must have paraPrIDRef=\"2\":\n{xml}"
    );
}

/// Multiple ordered list items must each carry `numPrIDRef="1"` (DIGIT, id=1).
#[test]
fn section_xml_multiple_ordered_items_all_have_num_pr_id_ref_2() {
    let xml = section_xml(vec![Block::List {
        ordered: true,
        start: 1,
        items: vec![
            ListItem {
                blocks: vec![Block::Paragraph {
                    inlines: vec![inline("one")],
                }],
                children: vec![],
            },
            ListItem {
                blocks: vec![Block::Paragraph {
                    inlines: vec![inline("two")],
                }],
                children: vec![],
            },
        ],
    }]);
    let count = xml.matches(r#"numPrIDRef="1""#).count();
    assert_eq!(
        count, 2,
        "two ordered list items must each have numPrIDRef=\"1\"; found {count}: {xml}"
    );
}

// ── numPrIDRef distinction ───────────────────────────────────────────────────

/// Ordered and unordered lists must be distinguishable in the XML output.
/// Ordered: carries `numPrIDRef="1"` (DIGIT, id=1).
/// Unordered: carries NO `numPrIDRef` (BULLET not valid in OWPML schema).
#[test]
fn section_xml_ordered_and_unordered_use_different_num_pr_id_refs() {
    // Ordered list in the same document as unordered list.
    let xml = section_xml(vec![
        Block::List {
            ordered: false,
            start: 1,
            items: vec![ListItem {
                blocks: vec![Block::Paragraph {
                    inlines: vec![inline("bullet")],
                }],
                children: vec![],
            }],
        },
        Block::List {
            ordered: true,
            start: 1,
            items: vec![ListItem {
                blocks: vec![Block::Paragraph {
                    inlines: vec![inline("number")],
                }],
                children: vec![],
            }],
        },
    ]);
    // Ordered list item uses numPrIDRef="1" (DIGIT).
    assert!(
        xml.contains(r#"numPrIDRef="1""#),
        "ordered list item numPrIDRef=\"1\" must appear:\n{xml}"
    );
    // Both items must have their text.
    assert!(
        xml.contains("bullet"),
        "unordered text 'bullet' must appear: {xml}"
    );
    assert!(
        xml.contains("number"),
        "ordered text 'number' must appear: {xml}"
    );
}

// ── Paragraph count ────────────────────────────────────────────────────────

/// Each list item produces exactly one `<hp:p>` paragraph.
#[test]
fn section_xml_list_items_produce_correct_paragraph_count() {
    let xml = section_xml(vec![Block::List {
        ordered: false,
        start: 1,
        items: vec![
            ListItem {
                blocks: vec![Block::Paragraph {
                    inlines: vec![inline("a")],
                }],
                children: vec![],
            },
            ListItem {
                blocks: vec![Block::Paragraph {
                    inlines: vec![inline("b")],
                }],
                children: vec![],
            },
            ListItem {
                blocks: vec![Block::Paragraph {
                    inlines: vec![inline("c")],
                }],
                children: vec![],
            },
        ],
    }]);
    let p_count = xml.matches("<hp:p ").count();
    assert_eq!(
        p_count, 3,
        "three list items must produce three <hp:p> elements; found {p_count}: {xml}"
    );
}

// ── Edge cases ─────────────────────────────────────────────────────────────

/// An empty list (no items) produces no paragraph elements.
#[test]
fn section_xml_empty_list_produces_no_paragraphs() {
    let xml = section_xml(vec![Block::List {
        ordered: false,
        start: 1,
        items: vec![],
    }]);
    // The section wrapper is present but no <hp:p> elements inside.
    assert!(
        !xml.contains("<hp:p "),
        "empty list must produce no <hp:p> elements:\n{xml}"
    );
}

/// A list item with bold inline text retains the inline charPr formatting
/// attribute. Unordered items have NO numPrIDRef (bullet not valid in OWPML).
#[test]
fn section_xml_list_item_with_bold_inline_has_both_num_pr_and_charpr() {
    let xml = section_xml(vec![Block::List {
        ordered: false,
        start: 1,
        items: vec![ListItem {
            blocks: vec![Block::Paragraph {
                inlines: vec![bold_inline("important")],
            }],
            children: vec![],
        }],
    }]);
    assert!(
        !xml.contains("numPrIDRef"),
        "unordered bold list item must NOT have numPrIDRef (no BULLET numbering):\n{xml}"
    );
    assert!(
        xml.contains(r#"bold="true""#),
        "bold attribute must be present on inline charPr:\n{xml}"
    );
    assert!(
        xml.contains("important"),
        "text content must be present: {xml}"
    );
}

/// A list item with no inline content (empty paragraph) is emitted without error.
#[test]
fn section_xml_list_item_empty_paragraph_no_panic() {
    let xml = section_xml(vec![Block::List {
        ordered: true,
        start: 1,
        items: vec![ListItem {
            blocks: vec![Block::Paragraph { inlines: vec![] }],
            children: vec![],
        }],
    }]);
    assert!(
        xml.contains(r#"numPrIDRef="1""#),
        "empty paragraph ordered list item must have numPrIDRef=\"1\" (DIGIT):\n{xml}"
    );
    assert!(
        xml.contains("<hp:p "),
        "paragraph element must be emitted: {xml}"
    );
}

// ── Nested lists (ListItem.children) ─────────────────────────────────────────

/// A list item with children emits both the parent item and child items,
/// with child items using paraPrIDRef="3" (depth-1 indent).
#[test]
fn section_xml_nested_list_via_children_uses_para_pr_id_ref_3() {
    let xml = section_xml(vec![Block::List {
        ordered: false,
        start: 1,
        items: vec![ListItem {
            blocks: vec![Block::Paragraph {
                inlines: vec![inline("parent item")],
            }],
            children: vec![ListItem {
                blocks: vec![Block::Paragraph {
                    inlines: vec![inline("child item")],
                }],
                children: vec![],
            }],
        }],
    }]);
    assert!(xml.contains("parent item"), "parent item text: {xml}");
    assert!(xml.contains("child item"), "child item text: {xml}");
    // Parent item: paraPrIDRef="2" (depth=0)
    assert!(
        xml.contains(r#"paraPrIDRef="2""#),
        "parent item must have paraPrIDRef=\"2\":\n{xml}"
    );
    // Child item: paraPrIDRef="3" (depth=1)
    assert!(
        xml.contains(r#"paraPrIDRef="3""#),
        "child item must have paraPrIDRef=\"3\":\n{xml}"
    );
}

/// A nested `Block::List` inside a list item (via item.blocks) uses
/// paraPrIDRef="3" for its paragraphs.
#[test]
fn section_xml_nested_block_list_inside_item_uses_para_pr_id_ref_3() {
    let xml = section_xml(vec![Block::List {
        ordered: false,
        start: 1,
        items: vec![ListItem {
            blocks: vec![
                Block::Paragraph {
                    inlines: vec![inline("outer item")],
                },
                // Nested list as a block inside the item.
                Block::List {
                    ordered: true,
                    start: 1,
                    items: vec![ListItem {
                        blocks: vec![Block::Paragraph {
                            inlines: vec![inline("inner item")],
                        }],
                        children: vec![],
                    }],
                },
            ],
            children: vec![],
        }],
    }]);
    assert!(xml.contains("outer item"), "outer item text: {xml}");
    assert!(xml.contains("inner item"), "inner item text: {xml}");
    // Inner list items must be at depth=1, using paraPrIDRef="3".
    assert!(
        xml.contains(r#"paraPrIDRef="3""#),
        "nested block list items must have paraPrIDRef=\"3\":\n{xml}"
    );
}

// ── Paragraph ID counter continuity ─────────────────────────────────────────

/// Paragraph IDs across list items must be sequential and continue the
/// section-level counter correctly.
#[test]
fn section_xml_list_paragraph_ids_are_sequential() {
    let xml = section_xml(vec![Block::List {
        ordered: false,
        start: 1,
        items: vec![
            ListItem {
                blocks: vec![Block::Paragraph {
                    inlines: vec![inline("item 0")],
                }],
                children: vec![],
            },
            ListItem {
                blocks: vec![Block::Paragraph {
                    inlines: vec![inline("item 1")],
                }],
                children: vec![],
            },
        ],
    }]);
    assert!(
        xml.contains(r#"id="0""#),
        "first list paragraph must be id=0: {xml}"
    );
    assert!(
        xml.contains(r#"id="1""#),
        "second list paragraph must be id=1: {xml}"
    );
}

/// When a list follows other blocks, the list paragraph IDs continue from
/// where the preceding blocks left off.
#[test]
fn section_xml_list_paragraph_ids_continue_after_preceding_blocks() {
    let xml = section_xml(vec![
        Block::Paragraph {
            inlines: vec![inline("before")],
        },
        Block::List {
            ordered: false,
            start: 1,
            items: vec![ListItem {
                blocks: vec![Block::Paragraph {
                    inlines: vec![inline("list item")],
                }],
                children: vec![],
            }],
        },
    ]);
    // Paragraph "before" gets id=0; list item gets id=1.
    assert!(
        xml.contains(r#"id="0""#),
        "normal paragraph must be id=0: {xml}"
    );
    assert!(xml.contains(r#"id="1""#), "list item must be id=1: {xml}");
}

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
                    },
                    ListItem {
                        blocks: vec![Block::Paragraph {
                            inlines: vec![inline("step two")],
                        }],
                        children: vec![],
                    },
                ],
            }],

            page_layout: None,
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
                    },
                    ListItem {
                        blocks: vec![Block::Paragraph {
                            inlines: vec![inline("beta")],
                        }],
                        children: vec![],
                    },
                ],
            }],

            page_layout: None,
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
                }],
            }],

            page_layout: None,
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
