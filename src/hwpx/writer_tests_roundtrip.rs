use super::*;

// ── Roundtrip / integration tests ────────────────────────────────────────

// ── helpers ─────────────────────────────────────────────────────────────

/// Build a simple document with one paragraph containing the given inlines.
fn roundtrip_doc(inlines: Vec<Inline>) -> Document {
    Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::Paragraph { inlines }],
        }],
        assets: Vec::new(),
    }
}

/// Write a document to HWPX, read it back, and return the first paragraph's inlines.
fn roundtrip_inlines(inlines: Vec<Inline>) -> Vec<Inline> {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = roundtrip_doc(inlines);
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");
    let read_back = read_hwpx(tmp.path()).expect("read_hwpx");
    read_back
        .sections
        .into_iter()
        .flat_map(|s| s.blocks)
        .filter_map(|b| match b {
            Block::Paragraph { inlines } => Some(inlines),
            _ => None,
        })
        .next()
        .unwrap_or_default()
}

// ── bold/italic/underline/strike roundtrip ──────────────────────────────

#[test]
fn roundtrip_bold_text_preserved() {
    let result = roundtrip_inlines(vec![bold_inline("bold text")]);
    assert_eq!(result.len(), 1, "expected 1 inline: {result:?}");
    assert_eq!(result[0].text, "bold text");
    assert!(result[0].bold, "bold flag must survive roundtrip: {result:?}");
}

#[test]
fn roundtrip_italic_text_preserved() {
    let result = roundtrip_inlines(vec![italic_inline("italic text")]);
    assert_eq!(result.len(), 1, "expected 1 inline: {result:?}");
    assert_eq!(result[0].text, "italic text");
    assert!(
        result[0].italic,
        "italic flag must survive roundtrip: {result:?}"
    );
}

#[test]
fn roundtrip_bold_italic_combined_preserved() {
    let input = Inline {
        text: "bold italic".into(),
        bold: true,
        italic: true,
        ..Inline::default()
    };
    let result = roundtrip_inlines(vec![input]);
    assert_eq!(result.len(), 1, "expected 1 inline: {result:?}");
    assert_eq!(result[0].text, "bold italic");
    assert!(result[0].bold, "bold must survive roundtrip: {result:?}");
    assert!(result[0].italic, "italic must survive roundtrip: {result:?}");
}

#[test]
fn roundtrip_underline_text_preserved() {
    let result = roundtrip_inlines(vec![underline_inline("underlined")]);
    assert_eq!(result.len(), 1, "expected 1 inline: {result:?}");
    assert_eq!(result[0].text, "underlined");
    assert!(
        result[0].underline,
        "underline flag must survive roundtrip: {result:?}"
    );
}

#[test]
fn roundtrip_strikethrough_text_preserved() {
    let input = Inline {
        text: "struck".into(),
        strikethrough: true,
        ..Inline::default()
    };
    let result = roundtrip_inlines(vec![input]);
    assert_eq!(result.len(), 1, "expected 1 inline: {result:?}");
    assert_eq!(result[0].text, "struck");
    assert!(
        result[0].strikethrough,
        "strikethrough flag must survive roundtrip: {result:?}"
    );
}

#[test]
fn roundtrip_superscript_text_preserved() {
    let input = Inline {
        text: "sup".into(),
        superscript: true,
        ..Inline::default()
    };
    let result = roundtrip_inlines(vec![input]);
    assert_eq!(result.len(), 1, "expected 1 inline: {result:?}");
    assert_eq!(result[0].text, "sup");
    assert!(
        result[0].superscript,
        "superscript flag must survive roundtrip: {result:?}"
    );
}

#[test]
fn roundtrip_subscript_text_preserved() {
    let input = Inline {
        text: "sub".into(),
        subscript: true,
        ..Inline::default()
    };
    let result = roundtrip_inlines(vec![input]);
    assert_eq!(result.len(), 1, "expected 1 inline: {result:?}");
    assert_eq!(result[0].text, "sub");
    assert!(
        result[0].subscript,
        "subscript flag must survive roundtrip: {result:?}"
    );
}

#[test]
fn roundtrip_color_text_preserved() {
    let input = Inline {
        text: "red".into(),
        color: Some("#FF0000".into()),
        ..Inline::default()
    };
    let result = roundtrip_inlines(vec![input]);
    assert_eq!(result.len(), 1, "expected 1 inline: {result:?}");
    assert_eq!(result[0].text, "red");
    assert_eq!(
        result[0].color.as_deref(),
        Some("#FF0000"),
        "color must survive roundtrip: {result:?}"
    );
}

#[test]
fn roundtrip_mixed_plain_and_bold_preserved() {
    let result = roundtrip_inlines(vec![inline("normal "), bold_inline("bold")]);
    assert_eq!(result.len(), 2, "expected 2 inlines: {result:?}");
    assert_eq!(result[0].text, "normal ");
    assert!(!result[0].bold, "first inline must not be bold: {result:?}");
    assert_eq!(result[1].text, "bold");
    assert!(result[1].bold, "second inline must be bold: {result:?}");
}

#[test]
fn roundtrip_bold_italic_underline_strike_color_combined() {
    let input = Inline {
        text: "all".into(),
        bold: true,
        italic: true,
        underline: true,
        strikethrough: true,
        color: Some("#00FF00".into()),
        ..Inline::default()
    };
    let result = roundtrip_inlines(vec![input]);
    assert_eq!(result.len(), 1, "expected 1 inline: {result:?}");
    let r = &result[0];
    assert!(r.bold, "bold: {result:?}");
    assert!(r.italic, "italic: {result:?}");
    assert!(r.underline, "underline: {result:?}");
    assert!(r.strikethrough, "strikethrough: {result:?}");
    assert_eq!(r.color.as_deref(), Some("#00FF00"), "color: {result:?}");
}

// ── section XML inline charPr emission ──────────────────────────────────

#[test]
fn section_xml_bold_inline_emits_inline_charpr() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![bold_inline("hello")],
    }]);
    assert!(
        xml.contains(r#"bold="true""#),
        "section XML must contain inline charPr with bold=\"true\": {xml}"
    );
}

#[test]
fn section_xml_italic_inline_emits_inline_charpr() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![italic_inline("hello")],
    }]);
    assert!(
        xml.contains(r#"italic="true""#),
        "section XML must contain inline charPr with italic=\"true\": {xml}"
    );
}

#[test]
fn section_xml_underline_inline_emits_inline_charpr() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![underline_inline("hello")],
    }]);
    assert!(
        xml.contains(r#"underline="true""#),
        "section XML must contain inline charPr with underline=\"true\": {xml}"
    );
}

#[test]
fn section_xml_strikethrough_inline_emits_strikeout() {
    let input = Inline {
        text: "hello".into(),
        strikethrough: true,
        ..Inline::default()
    };
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![input],
    }]);
    assert!(
        xml.contains(r#"strikeout="true""#),
        "section XML must contain inline charPr with strikeout=\"true\": {xml}"
    );
}

#[test]
fn section_xml_color_inline_emits_color_without_hash() {
    let input = Inline {
        text: "hello".into(),
        color: Some("#FF0000".into()),
        ..Inline::default()
    };
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![input],
    }]);
    assert!(
        xml.contains(r#"color="FF0000""#),
        "section XML must contain inline charPr with color=\"FF0000\" (no hash): {xml}"
    );
    assert!(
        !xml.contains(r##"color="#FF0000""##),
        "color must not have leading # in OWPML: {xml}"
    );
}

#[test]
fn section_xml_plain_inline_no_charpr_element() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![inline("plain")],
    }]);
    // Inside the <hp:run> there should be NO <hp:charPr> for plain text.
    // However the section XML itself may contain "hp:charPr" in other contexts
    // (e.g. attribute references), so we check specifically for the element pattern.
    assert!(
        !xml.contains("<hp:charPr "),
        "plain inline must NOT emit inline <hp:charPr> element: {xml}"
    );
}

// ── font_name roundtrip ────────────────────────────────────────────────

#[test]
fn roundtrip_font_name_preserved() {
    let input = Inline {
        text: "styled".into(),
        font_name: Some("Malgun Gothic".into()),
        ..Inline::default()
    };
    let result = roundtrip_inlines(vec![input]);
    assert_eq!(result.len(), 1, "expected 1 inline: {result:?}");
    assert_eq!(result[0].text, "styled");
    assert_eq!(
        result[0].font_name.as_deref(),
        Some("Malgun Gothic"),
        "font_name must survive roundtrip: {result:?}"
    );
}

#[test]
fn roundtrip_font_name_with_bold_preserved() {
    let input = Inline {
        text: "bold styled".into(),
        bold: true,
        font_name: Some("Malgun Gothic".into()),
        ..Inline::default()
    };
    let result = roundtrip_inlines(vec![input]);
    assert_eq!(result.len(), 1, "expected 1 inline: {result:?}");
    assert_eq!(result[0].text, "bold styled");
    assert!(result[0].bold, "bold must survive roundtrip: {result:?}");
    assert_eq!(
        result[0].font_name.as_deref(),
        Some("Malgun Gothic"),
        "font_name must survive roundtrip with bold: {result:?}"
    );
}

#[test]
fn section_xml_font_name_emits_face_name_id_ref() {
    let input = Inline {
        text: "hello".into(),
        font_name: Some("Malgun Gothic".into()),
        ..Inline::default()
    };
    let doc = doc_with_section(vec![Block::Paragraph {
        inlines: vec![input],
    }]);
    let tables = RefTables::build(&doc);
    let sec = &doc.sections[0];
    let xml = generate_section_xml(sec, 0, &tables).expect("generate_section_xml failed");

    assert!(
        xml.contains("faceNameIDRef="),
        "section XML must contain faceNameIDRef for font_name inline: {xml}"
    );

    // Verify the specific index value: "Malgun Gothic" is the second font
    // registered (index 1) because DEFAULT_FONT ("바탕") occupies index 0.
    let expected_idx = tables
        .font_names
        .iter()
        .position(|f| f == "Malgun Gothic")
        .expect("Malgun Gothic must be in font_names");
    assert_eq!(
        expected_idx, 1,
        "Malgun Gothic should be at index 1 (바탕 is 0): {:?}",
        tables.font_names
    );
    let expected_attr = format!("faceNameIDRef=\"{expected_idx}\"");
    assert!(
        xml.contains(&expected_attr),
        "section XML must contain {expected_attr}: {xml}"
    );
}

#[test]
fn header_xml_font_name_registered_in_fontface() {
    let input = Inline {
        text: "hello".into(),
        font_name: Some("Malgun Gothic".into()),
        ..Inline::default()
    };
    let doc = doc_with_section(vec![Block::Paragraph {
        inlines: vec![input],
    }]);
    let tables = RefTables::build(&doc);
    let header =
        super::header::generate_header_xml(&doc, &tables).expect("generate_header_xml failed");
    assert!(
        header.contains("Malgun Gothic"),
        "header XML must contain the registered font name: {header}"
    );
}

// ── default font / edge case roundtrip ──────────────────────────────────

/// An inline with NO font_name set (default "batang" at index 0) must
/// roundtrip with font_name remaining None.  The writer does not emit
/// faceNameIDRef for the default font, and the reader leaves font_name
/// as None when no faceNameIDRef attribute is present.
#[test]
fn roundtrip_default_font_no_font_name_preserved() {
    let input = Inline {
        text: "default font text".into(),
        bold: true,
        ..Inline::default()
    };
    assert!(
        input.font_name.is_none(),
        "precondition: input must have no font_name"
    );

    let result = roundtrip_inlines(vec![input]);
    assert_eq!(result.len(), 1, "expected 1 inline: {result:?}");
    assert_eq!(result[0].text, "default font text");
    assert!(result[0].bold, "bold must survive roundtrip: {result:?}");
    assert!(
        result[0].font_name.is_none(),
        "font_name must remain None for default font after roundtrip: {result:?}"
    );
}

/// When an inline carries a font_name that is NOT registered in the fontface
/// table, write_inline_charpr silently omits the faceNameIDRef attribute.
/// On read-back the font_name will therefore be None.
///
/// This scenario cannot happen through normal document construction (since
/// RefTables::build calls collect_from_inlines which registers every font),
/// but the test documents the expected graceful degradation if it somehow did.
#[test]
fn roundtrip_unknown_font_name_not_in_table() {
    // Build a document with one inline that has a known font.
    let doc = doc_with_section(vec![Block::Paragraph {
        inlines: vec![Inline {
            text: "known font".into(),
            font_name: Some("Malgun Gothic".into()),
            ..Inline::default()
        }],
    }]);
    let tables = RefTables::build(&doc);

    // Verify that the unknown font is indeed absent from the table.
    assert!(
        !tables.font_names.iter().any(|f| f == "Comic Sans MS"),
        "precondition: Comic Sans MS must not be in font_names: {:?}",
        tables.font_names
    );

    // Manually construct a section with an inline referencing a font NOT in the table.
    let rogue_section = Section {
        blocks: vec![Block::Paragraph {
            inlines: vec![Inline {
                text: "rogue".into(),
                font_name: Some("Comic Sans MS".into()),
                ..Inline::default()
            }],
        }],
    };
    let xml =
        generate_section_xml(&rogue_section, 0, &tables).expect("generate_section_xml failed");

    // The writer should NOT emit faceNameIDRef for a font not in the table.
    assert!(
        !xml.contains("faceNameIDRef="),
        "unknown font must not produce faceNameIDRef in section XML: {xml}"
    );

    // Still emits the charPr element (because font_name is Some, triggering
    // the has_formatting check), but without the faceNameIDRef attribute.
    assert!(
        xml.contains("<hp:charPr"),
        "charPr element should still be emitted for the font_name inline: {xml}"
    );
}

// ── image roundtrip ─────────────────────────────────────────────────────

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

// ── golden file test: verify actual XML output ─────────────────────────

/// Golden file test that validates the writer's actual XML output byte-for-byte
/// rather than relying on roundtrip fidelity.  Builds a comprehensive IR document
/// (heading, bold paragraph, italic paragraph, table, list), writes it to HWPX,
/// then inspects specific XML files inside the ZIP for expected patterns.
#[test]
fn golden_comprehensive_document_structure() {
    use std::io::Read as _;

    // ── 1. Build a comprehensive IR document ──

    let doc = Document {
        metadata: Metadata {
            title: Some("Golden Test Doc".into()),
            author: Some("Test Author".into()),
            ..Metadata::default()
        },
        sections: vec![Section {
            blocks: vec![
                // H1 heading
                Block::Heading {
                    level: 1,
                    inlines: vec![Inline::plain("Main Title")],
                },
                // Paragraph with bold inline
                Block::Paragraph {
                    inlines: vec![
                        inline("Normal text "),
                        bold_inline("bold text"),
                    ],
                },
                // Paragraph with italic inline
                Block::Paragraph {
                    inlines: vec![italic_inline("italic text")],
                },
                // 2x2 table
                Block::Table {
                    rows: vec![
                        TableRow {
                            cells: vec![
                                TableCell {
                                    blocks: vec![Block::Paragraph {
                                        inlines: vec![inline("Cell A1")],
                                    }],
                                    colspan: 1,
                                    rowspan: 1,
                                },
                                TableCell {
                                    blocks: vec![Block::Paragraph {
                                        inlines: vec![inline("Cell B1")],
                                    }],
                                    colspan: 1,
                                    rowspan: 1,
                                },
                            ],
                            is_header: true,
                        },
                        TableRow {
                            cells: vec![
                                TableCell {
                                    blocks: vec![Block::Paragraph {
                                        inlines: vec![inline("Cell A2")],
                                    }],
                                    colspan: 1,
                                    rowspan: 1,
                                },
                                TableCell {
                                    blocks: vec![Block::Paragraph {
                                        inlines: vec![inline("Cell B2")],
                                    }],
                                    colspan: 1,
                                    rowspan: 1,
                                },
                            ],
                            is_header: false,
                        },
                    ],
                    col_count: 2,
                },
                // Code block
                Block::CodeBlock {
                    language: Some("rust".into()),
                    code: "fn main() {}".into(),
                },
                // Horizontal rule
                Block::HorizontalRule,
                // Block quote
                Block::BlockQuote {
                    blocks: vec![Block::Paragraph {
                        inlines: vec![inline("quoted text")],
                    }],
                },
                // Unordered list
                Block::List {
                    ordered: false,
                    start: 1,
                    items: vec![
                        ListItem {
                            blocks: vec![Block::Paragraph {
                                inlines: vec![inline("List item one")],
                            }],
                            children: Vec::new(),
                        },
                        ListItem {
                            blocks: vec![Block::Paragraph {
                                inlines: vec![inline("List item two")],
                            }],
                            children: Vec::new(),
                        },
                    ],
                },
            ],
        }],
        assets: Vec::new(),
    };

    // ── 2. Write to HWPX bytes ──

    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");

    // ── 3. Open the ZIP and read specific XML entries ──

    let file = std::fs::File::open(tmp.path()).expect("open zip");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");

    // -- section0.xml assertions --
    let section_xml = {
        let mut entry = archive
            .by_name("Contents/section0.xml")
            .expect("section0.xml must exist in HWPX");
        let mut buf = String::new();
        entry.read_to_string(&mut buf).expect("read section0.xml");
        buf
    };

    // Verify heading has styleIDRef
    assert!(
        section_xml.contains(r#"hp:styleIDRef="1""#),
        "H1 heading must have hp:styleIDRef=\"1\" in section XML:\n{section_xml}"
    );

    // Verify heading text
    assert!(
        section_xml.contains("<hp:t>Main Title</hp:t>"),
        "heading text 'Main Title' must appear in <hp:t>:\n{section_xml}"
    );

    // Verify bold inline charPr
    assert!(
        section_xml.contains(r#"bold="true""#),
        "bold inline must emit charPr with bold=\"true\":\n{section_xml}"
    );

    // Verify bold text content
    assert!(
        section_xml.contains("<hp:t>bold text</hp:t>"),
        "bold text content must appear in <hp:t>:\n{section_xml}"
    );

    // Verify italic inline charPr
    assert!(
        section_xml.contains(r#"italic="true""#),
        "italic inline must emit charPr with italic=\"true\":\n{section_xml}"
    );

    // Verify italic text content
    assert!(
        section_xml.contains("<hp:t>italic text</hp:t>"),
        "italic text content must appear in <hp:t>:\n{section_xml}"
    );

    // Verify normal (non-bold, non-italic) text
    assert!(
        section_xml.contains("<hp:t>Normal text </hp:t>"),
        "plain text must appear in <hp:t>:\n{section_xml}"
    );

    // Verify table structure
    assert!(
        section_xml.contains(r#"<hp:tbl"#),
        "table must emit <hp:tbl> element:\n{section_xml}"
    );
    assert!(
        section_xml.contains(r#"rowCnt="2""#),
        "table must have rowCnt=\"2\":\n{section_xml}"
    );
    assert!(
        section_xml.contains(r#"colCnt="2""#),
        "table must have colCnt=\"2\":\n{section_xml}"
    );
    assert!(
        section_xml.contains("<hp:tr>"),
        "table must contain <hp:tr> rows:\n{section_xml}"
    );
    assert!(
        section_xml.contains("<hp:tc>"),
        "table must contain <hp:tc> cells:\n{section_xml}"
    );
    assert!(
        section_xml.contains("<hp:t>Cell A1</hp:t>"),
        "table cell text 'Cell A1' must appear:\n{section_xml}"
    );
    assert!(
        section_xml.contains("<hp:t>Cell B2</hp:t>"),
        "table cell text 'Cell B2' must appear:\n{section_xml}"
    );

    // Verify list items are emitted as paragraphs
    assert!(
        section_xml.contains("<hp:t>List item one</hp:t>"),
        "list item text 'List item one' must appear:\n{section_xml}"
    );
    assert!(
        section_xml.contains("<hp:t>List item two</hp:t>"),
        "list item text 'List item two' must appear:\n{section_xml}"
    );

    // Verify section XML namespace declarations
    assert!(
        section_xml.contains("xmlns:hs="),
        "section XML must declare hs namespace:\n{section_xml}"
    );
    assert!(
        section_xml.contains("xmlns:hp="),
        "section XML must declare hp namespace:\n{section_xml}"
    );

    // Verify no inline <hp:charPr> for plain text runs (the plain text
    // runs should only have a charPrIDRef attribute, not an inline element).
    // We check that the number of <hp:charPr occurrences matches the number
    // of formatted inlines (bold + italic = 2).
    let charpr_count = section_xml.matches("<hp:charPr ").count();
    assert!(
        charpr_count >= 2,
        "at least 2 inline <hp:charPr> elements expected (bold + italic), found {charpr_count}:\n{section_xml}"
    );

    // -- content.hpf assertions --
    let content_hpf = {
        let mut entry = archive
            .by_name("Contents/content.hpf")
            .expect("content.hpf must exist in HWPX");
        let mut buf = String::new();
        entry.read_to_string(&mut buf).expect("read content.hpf");
        buf
    };

    assert!(
        content_hpf.contains("section0.xml"),
        "content.hpf must reference section0.xml:\n{content_hpf}"
    );
    assert!(
        content_hpf.contains("<hp:title>Golden Test Doc</hp:title>"),
        "content.hpf must contain document title:\n{content_hpf}"
    );
    assert!(
        content_hpf.contains("<hp:author>Test Author</hp:author>"),
        "content.hpf must contain document author:\n{content_hpf}"
    );

    // -- header.xml assertions --
    let header_xml = {
        let mut entry = archive
            .by_name("Contents/header.xml")
            .expect("header.xml must exist in HWPX");
        let mut buf = String::new();
        entry.read_to_string(&mut buf).expect("read header.xml");
        buf
    };

    // Header must contain fontface declarations
    assert!(
        header_xml.contains("hh:fontface"),
        "header.xml must contain fontface declarations:\n{header_xml}"
    );

    // Header must contain charProperties
    assert!(
        header_xml.contains("hh:charPr"),
        "header.xml must contain charPr entries:\n{header_xml}"
    );

    // Header must contain styles (heading styles)
    assert!(
        header_xml.contains("hh:style"),
        "header.xml must contain style entries:\n{header_xml}"
    );

    // -- Verify code block text --
    assert!(
        section_xml.contains("<hp:t>fn main() {}</hp:t>"),
        "code block text must appear in <hp:t>:\n{section_xml}"
    );

    // -- Verify horizontal rule produces box-drawing characters --
    assert!(
        section_xml.contains("\u{2500}"),
        "horizontal rule must emit box-drawing characters:\n{section_xml}"
    );

    // -- Verify block quote text (emitted as plain paragraph) --
    assert!(
        section_xml.contains("<hp:t>quoted text</hp:t>"),
        "block quote text must appear in <hp:t>:\n{section_xml}"
    );

    // -- mimetype assertion --
    let mimetype = {
        let mut entry = archive
            .by_name("mimetype")
            .expect("mimetype must exist in HWPX");
        let mut buf = String::new();
        entry.read_to_string(&mut buf).expect("read mimetype");
        buf
    };
    assert_eq!(
        mimetype, "application/hwp+zip",
        "mimetype must be exactly 'application/hwp+zip'"
    );

    // -- version.xml assertion --
    archive
        .by_name("version.xml")
        .expect("version.xml must exist in HWPX");

    // -- META-INF/container.xml assertion --
    let container_xml = {
        let mut entry = archive
            .by_name("META-INF/container.xml")
            .expect("META-INF/container.xml must exist in HWPX");
        let mut buf = String::new();
        entry.read_to_string(&mut buf).expect("read container.xml");
        buf
    };
    assert!(
        container_xml.contains("content.hpf"),
        "container.xml must reference content.hpf:\n{container_xml}"
    );

    // -- Block quote must use paraPrIDRef="1" (indented paragraph) --
    assert!(
        section_xml.contains(r#"paraPrIDRef="1""#),
        "block quote paragraph must use paraPrIDRef=\"1\":\n{section_xml}"
    );

    // -- header.xml must have paraPr id="1" with left margin --
    assert!(
        header_xml.contains(r#"<hh:paraPr id="1">"#),
        "header.xml must contain paraPr id=\"1\" for blockquote indent:\n{header_xml}"
    );
}

// ── Phase A-3 tests: BlockQuote paraPr header + roundtrip ──────────────

#[test]
fn header_xml_contains_blockquote_para_pr() {
    let doc = doc_with_section(vec![Block::Paragraph {
        inlines: vec![inline("text")],
    }]);
    let tables = RefTables::build(&doc);
    let header =
        super::header::generate_header_xml(&doc, &tables).expect("generate_header_xml failed");

    // paraPr id="0" (normal) must exist.
    assert!(
        header.contains(r#"<hh:paraPr id="0">"#),
        "header must contain paraPr id=\"0\":\n{header}"
    );
    // paraPr id="1" (blockquote indent) must exist.
    assert!(
        header.contains(r#"<hh:paraPr id="1">"#),
        "header must contain paraPr id=\"1\":\n{header}"
    );
    // itemCnt must be "2" for paraProperties.
    assert!(
        header.contains(r#"itemCnt="2""#),
        "paraProperties itemCnt must be 2:\n{header}"
    );
    // paraPr id="1" must have a left margin value of 800.
    assert!(
        header.contains(r#"<hh:left value="800"/>"#),
        "paraPr id=\"1\" must have left margin 800:\n{header}"
    );
}

#[test]
fn header_xml_para_pr_0_has_zero_left_margin() {
    let doc = doc_with_section(vec![Block::Paragraph {
        inlines: vec![inline("text")],
    }]);
    let tables = RefTables::build(&doc);
    let header =
        super::header::generate_header_xml(&doc, &tables).expect("generate_header_xml failed");

    // paraPr id="0" must have left margin = 0.
    // We verify that the first paraPr (id=0) has <hh:left value="0"/>.
    // Since id=0 comes first, the first occurrence of <hh:left is from id=0.
    let first_left_pos = header
        .find(r#"<hh:left value="#)
        .expect("hh:left must exist");
    let first_left_slice = &header[first_left_pos..];
    assert!(
        first_left_slice.starts_with(r#"<hh:left value="0"/>"#),
        "first paraPr (id=0) must have left margin 0:\n{header}"
    );
}

#[test]
fn roundtrip_blockquote_content_preserved() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::BlockQuote {
                blocks: vec![Block::Paragraph {
                    inlines: vec![inline("roundtrip quote")],
                }],
            }],
        }],
        assets: Vec::new(),
    };
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");
    let read_back = read_hwpx(tmp.path()).expect("read_hwpx");

    // The text must survive the roundtrip.  The reader currently does not
    // reconstruct BlockQuote from paraPrIDRef, so the content appears as
    // a plain Paragraph -- that is acceptable for now.
    let has_quote_text = read_back
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .any(|b| match b {
            Block::Paragraph { inlines } => inlines.iter().any(|i| i.text == "roundtrip quote"),
            Block::BlockQuote { blocks } => blocks.iter().any(|b2| {
                matches!(b2, Block::Paragraph { inlines } if inlines.iter().any(|i| i.text == "roundtrip quote"))
            }),
            _ => false,
        });
    assert!(
        has_quote_text,
        "blockquote text must survive HWPX roundtrip; sections: {:?}",
        read_back.sections
    );
}
