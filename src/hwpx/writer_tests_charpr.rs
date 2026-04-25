use super::*;

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
    assert!(content.contains("hh:leftBorder"), "leftBorder: {content}");
    assert!(content.contains("hh:rightBorder"), "rightBorder: {content}");
    assert!(content.contains("hh:topBorder"), "topBorder: {content}");
    assert!(
        content.contains("hh:bottomBorder"),
        "bottomBorder: {content}"
    );
    assert!(content.contains("hh:diagonal"), "diagonal: {content}");
    // Slash and backSlash elements must be present.
    assert!(content.contains("hh:slash"), "slash element: {content}");
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

// ── Phase 4 tests: styles ────────────────────────────────────────────────

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

// ── Phase 8 tests: CharPrKey unit tests ──────────────────────────────────

#[test]
fn charpr_key_from_inline_code_sets_code_true_and_monospace_font() {
    let inline_val = Inline {
        text: "x".into(),
        code: true,
        ..Inline::default()
    };
    let key = CharPrKey::from_inline(&inline_val);
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
    let inline_val = Inline {
        text: "x".into(),
        code: true,
        font_name: Some("Arial".into()),
        ..Inline::default()
    };
    let key = CharPrKey::from_inline(&inline_val);
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

#[test]
fn bold_superscript_combo_gets_unique_charpr_and_supscript_attribute() {
    // An inline with both bold=true AND superscript=true must receive a
    // charPrIDRef that is not 0 (distinct from the plain entry), and the
    // header.xml must contain a charPr with supscript="superscript".
    let bold_sup_inline = Inline {
        text: "X".into(),
        bold: true,
        superscript: true,
        ..Inline::default()
    };

    // Verify charPrIDRef is non-zero via section XML.
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![bold_sup_inline.clone()],
    }]);
    let marker = "charPrIDRef=\"";
    let start = xml.find(marker).expect("charPrIDRef must be present");
    let rest = &xml[start + marker.len()..];
    let end = rest.find('"').expect("closing quote");
    let value = &rest[..end];
    assert_ne!(
        value, "0",
        "bold+superscript charPrIDRef must not be 0 (plain): {xml}"
    );

    // Verify header.xml carries the supscript attribute via write_hwpx.
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = doc_with_section(vec![Block::Paragraph {
        inlines: vec![bold_sup_inline],
    }]);
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive.by_name("Contents/header.xml").expect("header.xml");
    let mut content = String::new();
    entry.read_to_string(&mut content).expect("read");

    assert!(
        content.contains(r#"supscript="superscript""#),
        "header must contain supscript=\"superscript\" for bold+superscript inline: {content}"
    );
}
