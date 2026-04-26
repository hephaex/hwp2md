use super::*;

// ── Roundtrip / integration tests ────────────────────────────────────────
//
// Golden, structural, and blockquote tests have been moved to
// writer_tests_golden.rs to keep each file under the 800-line guideline.
// They are registered in writer_tests.rs as `mod tests_golden`.

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
    assert!(
        result[0].bold,
        "bold flag must survive roundtrip: {result:?}"
    );
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
    assert!(
        result[0].italic,
        "italic must survive roundtrip: {result:?}"
    );
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
    let xml = generate_section_xml(sec, 0, &tables, &ImageAssetMap::new())
        .expect("generate_section_xml failed");

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
    let xml = generate_section_xml(&rogue_section, 0, &tables, &ImageAssetMap::new())
        .expect("generate_section_xml failed");

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
