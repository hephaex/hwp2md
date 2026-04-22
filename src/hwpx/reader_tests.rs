use super::*;

// -----------------------------------------------------------------------
// Existing parse_heading_style tests
// -----------------------------------------------------------------------

#[test]
fn parse_heading_style_heading1() {
    assert_eq!(parse_heading_style("Heading1"), Some(1));
}

#[test]
fn parse_heading_style_heading6() {
    assert_eq!(parse_heading_style("Heading6"), Some(6));
}

#[test]
fn parse_heading_style_korean_title() {
    // "제목1" -> 1
    assert_eq!(parse_heading_style("제목1"), Some(1));
}

#[test]
fn parse_heading_style_korean_outline_3() {
    // "개요3" -> 3
    assert_eq!(parse_heading_style("개요3"), Some(3));
}

#[test]
fn parse_heading_style_normal_is_none() {
    assert_eq!(parse_heading_style("Normal"), None);
}

#[test]
fn parse_heading_style_body_text_is_none() {
    assert_eq!(parse_heading_style("BodyText"), None);
}

#[test]
fn parse_heading_style_heading_no_digit_defaults_to_1() {
    // "Heading" without a trailing digit -> defaults to level 1.
    assert_eq!(parse_heading_style("Heading"), Some(1));
}

#[test]
fn parse_heading_style_case_insensitive() {
    assert_eq!(parse_heading_style("HEADING2"), Some(2));
}

// -----------------------------------------------------------------------
// parse_section_xml -- helper for asserting on the returned Section
// -----------------------------------------------------------------------

/// Unwrap the section and panic with a descriptive message on error.
fn section(xml: &str) -> ir::Section {
    parse_section_xml(xml).expect("parse_section_xml must not fail")
}

// -----------------------------------------------------------------------
// parse_section_xml -- empty / minimal documents
// -----------------------------------------------------------------------

#[test]
fn empty_document_produces_no_blocks() {
    let s = section("<root/>");
    assert!(s.blocks.is_empty(), "expected no blocks for empty XML");
}

#[test]
fn empty_paragraph_produces_no_blocks() {
    // A paragraph element with no run content must be silently dropped.
    let xml = r#"<root><hp:p></hp:p></root>"#;
    let s = section(xml);
    assert!(
        s.blocks.is_empty(),
        "empty paragraph must not produce a block"
    );
}

// -----------------------------------------------------------------------
// parse_section_xml -- simple paragraph
// -----------------------------------------------------------------------

#[test]
fn simple_paragraph_text() {
    // Compact XML -- no whitespace text nodes between tags (matches real HWPX).
    let xml = r#"<root><hp:p><hp:run><hp:t>Hello World</hp:t></hp:run></hp:p></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            assert_eq!(inlines.len(), 1);
            assert_eq!(inlines[0].text, "Hello World");
            assert!(!inlines[0].bold);
            assert!(!inlines[0].italic);
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

#[test]
fn paragraph_without_hp_prefix() {
    // Bare element names (no namespace prefix) must also parse correctly.
    let xml = r#"<root><p><run><t>bare prefix</t></run></p></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            assert_eq!(inlines[0].text, "bare prefix");
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

#[test]
fn multiple_runs_in_one_paragraph_produce_multiple_inlines() {
    let xml = r#"<root><hp:p><hp:run><hp:t>first</hp:t></hp:run><hp:run><hp:t>second</hp:t></hp:run></hp:p></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            assert_eq!(inlines.len(), 2);
            assert_eq!(inlines[0].text, "first");
            assert_eq!(inlines[1].text, "second");
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

// -----------------------------------------------------------------------
// parse_section_xml -- heading via styleIDRef
// -----------------------------------------------------------------------

#[test]
fn heading_level2_via_style_id_ref() {
    let xml = r#"<root><hp:p styleIDRef="Heading2"><hp:run><hp:t>Chapter title</hp:t></hp:run></hp:p></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::Heading { level, inlines } => {
            assert_eq!(*level, 2);
            assert_eq!(inlines[0].text, "Chapter title");
        }
        other => panic!("expected Heading, got {other:?}"),
    }
}

#[test]
fn heading_level3_korean_style() {
    let xml =
        r#"<root><hp:p styleIDRef="제목3"><hp:run><hp:t>소제목</hp:t></hp:run></hp:p></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::Heading { level, inlines } => {
            assert_eq!(*level, 3);
            assert_eq!(inlines[0].text, "소제목");
        }
        other => panic!("expected Heading, got {other:?}"),
    }
}

// -----------------------------------------------------------------------
// parse_section_xml -- bold / italic via Start-element charPr
// -----------------------------------------------------------------------

#[test]
fn bold_text_via_charpr_start_element() {
    // Start-element charPr (non-self-closing) -- handled by handle_start_element.
    let xml = r#"<root><hp:p><hp:run><hp:charPr bold="true" italic="false"></hp:charPr><hp:t>bold text</hp:t></hp:run></hp:p></root>"#;
    let s = section(xml);
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            assert!(inlines[0].bold, "inline must be bold");
            assert!(!inlines[0].italic, "inline must not be italic");
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

#[test]
fn italic_text_via_charpr_empty_element() {
    // Self-closing charPr -- handled by handle_empty_element.
    let xml = r#"<root><hp:p><hp:run><hp:charPr bold="false" italic="true"/><hp:t>italic text</hp:t></hp:run></hp:p></root>"#;
    let s = section(xml);
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            assert!(!inlines[0].bold, "inline must not be bold");
            assert!(inlines[0].italic, "inline must be italic");
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

#[test]
fn bold_and_italic_via_empty_charpr() {
    let xml = r#"<root><hp:p><hp:run><hp:charPr bold="true" italic="true"/><hp:t>strong em</hp:t></hp:run></hp:p></root>"#;
    let s = section(xml);
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            assert!(inlines[0].bold);
            assert!(inlines[0].italic);
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

#[test]
fn charpr_numeric_1_means_bold() {
    // The parser accepts "1" as well as "true" for boolean attributes.
    let xml = r#"<root><hp:p><hp:run><hp:charPr bold="1"/><hp:t>numeric bold</hp:t></hp:run></hp:p></root>"#;
    let s = section(xml);
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            assert!(inlines[0].bold, "bold=\"1\" must be treated as true");
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

#[test]
fn charpr_resets_between_runs() {
    // Second run has no charPr, so bold must revert to false.
    // Compact XML avoids spurious whitespace-only text nodes.
    let xml = r#"<root><hp:p><hp:run><hp:charPr bold="true"/><hp:t>bold</hp:t></hp:run><hp:run><hp:t>plain</hp:t></hp:run></hp:p></root>"#;
    let s = section(xml);
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            assert_eq!(inlines.len(), 2);
            assert!(inlines[0].bold, "first inline must be bold");
            // The run end event resets bold to false.
            assert!(!inlines[1].bold, "second inline must not be bold");
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

// -----------------------------------------------------------------------
// parse_section_xml -- lineBreak (empty element)
// -----------------------------------------------------------------------

#[test]
fn line_break_appends_newline_to_inline_text() {
    // lineBreak is an empty element that appends \n to current_text.
    // It lives outside a <t> so flush_paragraph picks it up at paragraph end.
    let xml = r#"<root><hp:p><hp:run><hp:t>line one</hp:t><hp:lineBreak/></hp:run></hp:p></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            let full: String = inlines.iter().map(|i| i.text.as_str()).collect();
            assert!(
                full.contains('\n'),
                "paragraph inlines must contain a newline from lineBreak; got: {full:?}"
            );
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

// -----------------------------------------------------------------------
// parse_section_xml -- image (empty element)
// -----------------------------------------------------------------------

#[test]
fn image_element_produces_image_block() {
    let xml = r#"<root>
        <hp:img src="image1.png" alt="photo"/>
    </root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::Image { src, alt } => {
            assert_eq!(src, "image1.png");
            assert_eq!(alt, "photo");
        }
        other => panic!("expected Image, got {other:?}"),
    }
}

#[test]
fn image_with_empty_src_is_ignored() {
    // An img element with no src attribute must not produce a block.
    let xml = r#"<root><hp:img alt="no src"/></root>"#;
    let s = section(xml);
    assert!(s.blocks.is_empty(), "img without src must be dropped");
}

// -----------------------------------------------------------------------
// parse_section_xml -- equation
// -----------------------------------------------------------------------

#[test]
fn equation_element_produces_math_block() {
    let xml = r#"<root><hp:equation>x^2 + y^2</hp:equation></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::Math { display, tex } => {
            assert!(*display, "equation must be display mode");
            assert_eq!(tex, "x^2 + y^2");
        }
        other => panic!("expected Math, got {other:?}"),
    }
}

#[test]
fn empty_equation_produces_no_block() {
    // An equation element with no text content must be silently dropped.
    let xml = r#"<root><hp:equation></hp:equation></root>"#;
    let s = section(xml);
    assert!(
        s.blocks.is_empty(),
        "empty equation must not produce a block"
    );
}

#[test]
fn eqedit_alias_also_produces_math_block() {
    // The parser accepts both <hp:equation> and <hp:eqEdit>.
    let xml = r#"<root><hp:eqEdit>a + b = c</hp:eqEdit></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::Math { tex, .. } => assert_eq!(tex, "a + b = c"),
        other => panic!("expected Math, got {other:?}"),
    }
}

// -----------------------------------------------------------------------
// parse_section_xml -- table
// -----------------------------------------------------------------------

#[test]
fn simple_table_two_rows_two_cols() {
    let xml = concat!(
        r#"<root><hp:tbl colCnt="2">"#,
        r#"<hp:tr><hp:tc><hp:p><hp:run><hp:t>A1</hp:t></hp:run></hp:p></hp:tc>"#,
        r#"<hp:tc><hp:p><hp:run><hp:t>A2</hp:t></hp:run></hp:p></hp:tc></hp:tr>"#,
        r#"<hp:tr><hp:tc><hp:p><hp:run><hp:t>B1</hp:t></hp:run></hp:p></hp:tc>"#,
        r#"<hp:tc><hp:p><hp:run><hp:t>B2</hp:t></hp:run></hp:p></hp:tc></hp:tr>"#,
        r#"</hp:tbl></root>"#,
    );
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::Table { rows, col_count } => {
            assert_eq!(*col_count, 2);
            assert_eq!(rows.len(), 2);
            // First row is always the header row.
            assert!(rows[0].is_header, "first row must be is_header=true");
            assert!(!rows[1].is_header, "second row must be is_header=false");
            // Verify cell text content.
            let text_of = |row: usize, col: usize| -> String {
                match &rows[row].cells[col].blocks[0] {
                    ir::Block::Paragraph { inlines } => inlines[0].text.clone(),
                    other => panic!("unexpected block {other:?}"),
                }
            };
            assert_eq!(text_of(0, 0), "A1");
            assert_eq!(text_of(0, 1), "A2");
            assert_eq!(text_of(1, 0), "B1");
            assert_eq!(text_of(1, 1), "B2");
        }
        other => panic!("expected Table, got {other:?}"),
    }
}

#[test]
fn table_col_count_from_colcnt_attribute() {
    // colCnt="3" but only 2 cells per row -- col_count must be max(3, 2) = 3.
    let xml = concat!(
        r#"<root><hp:tbl colCnt="3"><hp:tr>"#,
        r#"<hp:tc><hp:p><hp:run><hp:t>X</hp:t></hp:run></hp:p></hp:tc>"#,
        r#"<hp:tc><hp:p><hp:run><hp:t>Y</hp:t></hp:run></hp:p></hp:tc>"#,
        r#"</hp:tr></hp:tbl></root>"#,
    );
    let s = section(xml);
    match &s.blocks[0] {
        ir::Block::Table { col_count, .. } => {
            assert_eq!(*col_count, 3);
        }
        other => panic!("expected Table, got {other:?}"),
    }
}

#[test]
fn table_cell_colspan_from_celladdr() {
    // cellAddr is a self-closing child of tc that sets colspan/rowspan.
    let xml = concat!(
        r#"<root><hp:tbl colCnt="3"><hp:tr>"#,
        r#"<hp:tc><hp:cellAddr colSpan="2" rowSpan="1"/><hp:p><hp:run><hp:t>merged</hp:t></hp:run></hp:p></hp:tc>"#,
        r#"<hp:tc><hp:p><hp:run><hp:t>single</hp:t></hp:run></hp:p></hp:tc>"#,
        r#"</hp:tr></hp:tbl></root>"#,
    );
    let s = section(xml);
    match &s.blocks[0] {
        ir::Block::Table { rows, .. } => {
            let cells = &rows[0].cells;
            assert_eq!(cells[0].colspan, 2, "first cell must have colspan=2");
            assert_eq!(cells[0].rowspan, 1, "first cell must have rowspan=1");
            assert_eq!(
                cells[1].colspan, 1,
                "second cell must have default colspan=1"
            );
        }
        other => panic!("expected Table, got {other:?}"),
    }
}

#[test]
fn table_cell_rowspan_from_celladdr() {
    let xml = concat!(
        r#"<root><hp:tbl colCnt="1"><hp:tr>"#,
        r#"<hp:tc><hp:cellAddr colSpan="1" rowSpan="3"/><hp:p><hp:run><hp:t>tall</hp:t></hp:run></hp:p></hp:tc>"#,
        r#"</hp:tr></hp:tbl></root>"#,
    );
    let s = section(xml);
    match &s.blocks[0] {
        ir::Block::Table { rows, .. } => {
            assert_eq!(rows[0].cells[0].rowspan, 3);
        }
        other => panic!("expected Table, got {other:?}"),
    }
}

#[test]
fn table_default_colspan_rowspan_without_celladdr() {
    // When no cellAddr is present the defaults must be colspan=1, rowspan=1.
    let xml = concat!(
        r#"<root><hp:tbl colCnt="1"><hp:tr>"#,
        r#"<hp:tc><hp:p><hp:run><hp:t>cell</hp:t></hp:run></hp:p></hp:tc>"#,
        r#"</hp:tr></hp:tbl></root>"#,
    );
    let s = section(xml);
    match &s.blocks[0] {
        ir::Block::Table { rows, .. } => {
            assert_eq!(rows[0].cells[0].colspan, 1);
            assert_eq!(rows[0].cells[0].rowspan, 1);
        }
        other => panic!("expected Table, got {other:?}"),
    }
}

#[test]
fn nested_paragraph_inside_table_cell() {
    // Text inside a table cell must end up in cell blocks, not section blocks.
    let xml = concat!(
        r#"<root><hp:tbl colCnt="1"><hp:tr>"#,
        r#"<hp:tc><hp:p><hp:run><hp:t>cell content</hp:t></hp:run></hp:p></hp:tc>"#,
        r#"</hp:tr></hp:tbl></root>"#,
    );
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::Table { rows, .. } => {
            let cell = &rows[0].cells[0];
            assert_eq!(cell.blocks.len(), 1);
            match &cell.blocks[0] {
                ir::Block::Paragraph { inlines } => {
                    assert_eq!(inlines[0].text, "cell content");
                }
                other => panic!("expected Paragraph inside cell, got {other:?}"),
            }
        }
        other => panic!("expected Table, got {other:?}"),
    }
}

#[test]
fn image_inside_table_cell_goes_to_cell_blocks() {
    let xml = concat!(
        r#"<root><hp:tbl colCnt="1"><hp:tr>"#,
        r#"<hp:tc><hp:img src="fig.png" alt="figure"/></hp:tc>"#,
        r#"</hp:tr></hp:tbl></root>"#,
    );
    let s = section(xml);
    match &s.blocks[0] {
        ir::Block::Table { rows, .. } => {
            let cell = &rows[0].cells[0];
            assert_eq!(cell.blocks.len(), 1);
            match &cell.blocks[0] {
                ir::Block::Image { src, alt } => {
                    assert_eq!(src, "fig.png");
                    assert_eq!(alt, "figure");
                }
                other => panic!("expected Image inside cell, got {other:?}"),
            }
        }
        other => panic!("expected Table, got {other:?}"),
    }
}

// -----------------------------------------------------------------------
// parse_section_xml -- list
// -----------------------------------------------------------------------

#[test]
fn ordered_list_without_li_produces_no_block() {
    // The current parser recognises <ol>/<ul> open/close but has no <li>
    // handler, so items will be empty.  The block is only pushed when
    // list_items is non-empty, so an empty ol must produce no block.
    // This test documents the current behaviour explicitly.
    let xml = r#"<root><ol></ol></root>"#;
    let s = section(xml);
    assert!(
        s.blocks.is_empty(),
        "empty <ol> without <li> children must produce no block (current behaviour)"
    );
}

#[test]
fn underline_via_empty_charpr() {
    let xml = r#"<root><hp:p><hp:run><hp:charPr underline="solid"/><hp:t>underlined</hp:t></hp:run></hp:p></root>"#;
    let s = section(xml);
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            assert!(inlines[0].underline, "inline must be underlined");
            assert!(!inlines[0].bold);
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

#[test]
fn strikeout_via_empty_charpr() {
    let xml = r#"<root><hp:p><hp:run><hp:charPr strikeout="true"/><hp:t>struck</hp:t></hp:run></hp:p></root>"#;
    let s = section(xml);
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            assert!(inlines[0].strikethrough, "inline must be strikethrough");
            assert!(!inlines[0].bold);
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

#[test]
fn colspan_zero_defaults_to_one() {
    let xml = concat!(
        r#"<root><hp:tbl colCnt="1"><hp:tr>"#,
        r#"<hp:tc><hp:cellAddr colSpan="0" rowSpan="0"/><hp:p><hp:run><hp:t>x</hp:t></hp:run></hp:p></hp:tc>"#,
        r#"</hp:tr></hp:tbl></root>"#,
    );
    let s = section(xml);
    match &s.blocks[0] {
        ir::Block::Table { rows, .. } => {
            assert_eq!(rows[0].cells[0].colspan, 1, "colSpan=0 must default to 1");
            assert_eq!(rows[0].cells[0].rowspan, 1, "rowSpan=0 must default to 1");
        }
        other => panic!("expected Table, got {other:?}"),
    }
}

// -----------------------------------------------------------------------
// guess_mime_from_name -- all extensions
// -----------------------------------------------------------------------

#[test]
fn guess_mime_png() {
    assert_eq!(guess_mime_from_name("image.png"), "image/png");
}

#[test]
fn guess_mime_jpg() {
    assert_eq!(guess_mime_from_name("photo.jpg"), "image/jpeg");
}

#[test]
fn guess_mime_jpeg() {
    assert_eq!(guess_mime_from_name("photo.jpeg"), "image/jpeg");
}

#[test]
fn guess_mime_gif() {
    assert_eq!(guess_mime_from_name("anim.gif"), "image/gif");
}

#[test]
fn guess_mime_bmp() {
    assert_eq!(guess_mime_from_name("bitmap.bmp"), "image/bmp");
}

#[test]
fn guess_mime_svg() {
    assert_eq!(guess_mime_from_name("vector.svg"), "image/svg+xml");
}

#[test]
fn guess_mime_wmf() {
    assert_eq!(guess_mime_from_name("metafile.wmf"), "image/x-wmf");
}

#[test]
fn guess_mime_emf() {
    assert_eq!(guess_mime_from_name("enhanced.emf"), "image/x-emf");
}

#[test]
fn guess_mime_unknown_extension_falls_back_to_octet_stream() {
    assert_eq!(
        guess_mime_from_name("archive.xyz"),
        "application/octet-stream"
    );
}

#[test]
fn guess_mime_no_extension_falls_back_to_octet_stream() {
    assert_eq!(
        guess_mime_from_name("nodotfile"),
        "application/octet-stream"
    );
}

#[test]
fn guess_mime_case_insensitive_uppercase_png() {
    assert_eq!(guess_mime_from_name("PHOTO.PNG"), "image/png");
}

#[test]
fn guess_mime_case_insensitive_mixed_jpg() {
    assert_eq!(guess_mime_from_name("Photo.Jpg"), "image/jpeg");
}

#[test]
fn guess_mime_case_insensitive_svg() {
    assert_eq!(guess_mime_from_name("LOGO.SVG"), "image/svg+xml");
}

// -----------------------------------------------------------------------
// BinData reference resolution -- resolve_bin_refs + build_bin_map
// -----------------------------------------------------------------------

/// Helper: build a section containing a single top-level Image block.
fn make_image_section(src: &str) -> ir::Section {
    ir::Section {
        blocks: vec![ir::Block::Image {
            src: src.to_string(),
            alt: String::new(),
        }],
    }
}

#[test]
fn resolve_bin_refs_replaces_image_src() {
    // An Image whose src matches a BinData stem must be updated to the
    // full ZIP path, including the extension.
    let bin_files = vec!["BinData/BIN0001.png".to_string()];
    let bin_map = build_bin_map(&bin_files);

    let mut section = make_image_section("BIN0001");
    resolve_bin_refs(&mut section, &bin_map);

    match &section.blocks[0] {
        ir::Block::Image { src, .. } => {
            assert_eq!(
                src, "BinData/BIN0001.png",
                "src must be resolved to full path"
            );
        }
        other => panic!("expected Image, got {other:?}"),
    }
}

#[test]
fn resolve_bin_refs_no_match_leaves_src_unchanged() {
    // An Image with a src that has no entry in the bin_map must not be
    // modified -- e.g. when src is already a full filename or an URL.
    let bin_files = vec!["BinData/BIN0001.png".to_string()];
    let bin_map = build_bin_map(&bin_files);

    let mut section = make_image_section("img.png");
    resolve_bin_refs(&mut section, &bin_map);

    match &section.blocks[0] {
        ir::Block::Image { src, .. } => {
            assert_eq!(src, "img.png", "unmatched src must remain unchanged");
        }
        other => panic!("expected Image, got {other:?}"),
    }
}

#[test]
fn resolve_bin_refs_inside_table_cell() {
    // resolve_block_bin_refs must recurse into Table -> rows -> cells -> blocks.
    let bin_files = vec!["BinData/BIN0002.jpg".to_string()];
    let bin_map = build_bin_map(&bin_files);

    let cell_image = ir::Block::Image {
        src: "BIN0002".to_string(),
        alt: String::new(),
    };
    let cell = ir::TableCell {
        blocks: vec![cell_image],
        colspan: 1,
        rowspan: 1,
    };
    let row = ir::TableRow {
        cells: vec![cell],
        is_header: false,
    };
    let mut section = ir::Section {
        blocks: vec![ir::Block::Table {
            rows: vec![row],
            col_count: 1,
        }],
    };

    resolve_bin_refs(&mut section, &bin_map);

    match &section.blocks[0] {
        ir::Block::Table { rows, .. } => match &rows[0].cells[0].blocks[0] {
            ir::Block::Image { src, .. } => {
                assert_eq!(
                    src, "BinData/BIN0002.jpg",
                    "image inside table cell must be resolved"
                );
            }
            other => panic!("expected Image inside cell, got {other:?}"),
        },
        other => panic!("expected Table, got {other:?}"),
    }
}

#[test]
fn bin_map_from_bin_files() {
    // build_bin_map must produce a map with stem keys and full-path values.
    // It must handle both prefixes (BinData/ and Contents/BinData/).
    let bin_files = vec![
        "BinData/BIN0001.png".to_string(),
        "BinData/BIN0002.jpg".to_string(),
        "Contents/BinData/BIN0003.emf".to_string(),
    ];
    let map = build_bin_map(&bin_files);

    assert_eq!(
        map.get("BIN0001").map(String::as_str),
        Some("BinData/BIN0001.png")
    );
    assert_eq!(
        map.get("BIN0002").map(String::as_str),
        Some("BinData/BIN0002.jpg")
    );
    assert_eq!(
        map.get("BIN0003").map(String::as_str),
        Some("Contents/BinData/BIN0003.emf")
    );
    assert_eq!(map.len(), 3, "map must contain exactly 3 entries");
}
// -----------------------------------------------------------------------
// parse_section_xml -- footnote / endnote parsing
// -----------------------------------------------------------------------

fn first_footnote(s: &ir::Section) -> (&str, &[ir::Block]) {
    match &s.blocks[0] {
        ir::Block::Footnote { id, content } => (id.as_str(), content.as_slice()),
        other => panic!("expected Block::Footnote, got {other:?}"),
    }
}

#[test]
fn footnote_produces_footnote_block() {
    let xml = r#"<root><hp:fn id="1"><hp:p><hp:run><hp:t>note text</hp:t></hp:run></hp:p></hp:fn></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1, "one footnote block expected");
    let (id, content) = first_footnote(&s);
    assert_eq!(id, "1");
    assert_eq!(
        content.len(),
        1,
        "footnote must have exactly one inner block"
    );
    match &content[0] {
        ir::Block::Paragraph { inlines } => {
            assert_eq!(inlines[0].text, "note text");
        }
        other => panic!("expected Paragraph inside footnote, got {other:?}"),
    }
}

#[test]
fn endnote_produces_footnote_block() {
    let xml =
        r#"<root><hp:en id="2"><hp:p><hp:run><hp:t>end note</hp:t></hp:run></hp:p></hp:en></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    let (id, content) = first_footnote(&s);
    assert_eq!(id, "2");
    match &content[0] {
        ir::Block::Paragraph { inlines } => {
            assert_eq!(inlines[0].text, "end note");
        }
        other => panic!("expected Paragraph inside endnote block, got {other:?}"),
    }
}

#[test]
fn footnote_alt_tag_name() {
    let xml = r#"<root><hp:footnote id="3"><hp:p><hp:run><hp:t>alt tag</hp:t></hp:run></hp:p></hp:footnote></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    let (id, content) = first_footnote(&s);
    assert_eq!(id, "3");
    match &content[0] {
        ir::Block::Paragraph { inlines } => {
            assert_eq!(inlines[0].text, "alt tag");
        }
        other => panic!("expected Paragraph inside footnote (alt tag), got {other:?}"),
    }
}

#[test]
fn note_ref_produces_footnote_ref_inline() {
    // <hp:noteRef noteId="1"/> produces an Inline with footnote_ref set and empty text.
    let xml = r#"<root><hp:p><hp:noteRef noteId="1"/></hp:p></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1, "one paragraph block expected");
    match &s.blocks[0] {
        ir::Block::Paragraph { inlines } => {
            assert_eq!(inlines.len(), 1, "one inline expected");
            assert_eq!(
                inlines[0].footnote_ref.as_deref(),
                Some("1"),
                "inline must carry footnote_ref=\"1\""
            );
            assert!(
                inlines[0].text.is_empty(),
                "footnote_ref inline must have empty text"
            );
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

#[test]
fn empty_footnote_ignored() {
    let xml = r#"<root><hp:fn id="1"></hp:fn></root>"#;
    let s = section(xml);
    assert!(
        s.blocks.is_empty(),
        "empty footnote must not produce a Block::Footnote"
    );
}

// -----------------------------------------------------------------------
// Cross-cutting: context x element combinations
// -----------------------------------------------------------------------

#[test]
fn image_inside_footnote_goes_to_footnote_blocks() {
    let xml = r#"<root><hp:fn id="1"><hp:img src="fig.png" alt="fn-img"/></hp:fn></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::Footnote { content, .. } => {
            assert!(
                content
                    .iter()
                    .any(|b| matches!(b, ir::Block::Image { src, .. } if src == "fig.png")),
                "footnote must contain the image block"
            );
        }
        other => panic!("expected Footnote, got {other:?}"),
    }
}

#[test]
fn image_inside_list_item_goes_to_list_item_blocks() {
    let xml = r#"<root><ul><li><hp:img src="pic.png" alt="li-img"/></li></ul></root>"#;
    let s = section(xml);
    assert_eq!(s.blocks.len(), 1);
    match &s.blocks[0] {
        ir::Block::List { items, .. } => {
            assert_eq!(items.len(), 1);
            assert!(
                items[0]
                    .blocks
                    .iter()
                    .any(|b| matches!(b, ir::Block::Image { src, .. } if src == "pic.png")),
                "list item must contain the image block"
            );
        }
        other => panic!("expected List, got {other:?}"),
    }
}

#[test]
fn linebreak_inside_list_item_appends_newline() {
    let xml = r#"<root><ul><li><hp:p><hp:run><hp:t>before</hp:t><hp:lineBreak/></hp:run></hp:p></li></ul></root>"#;
    let s = section(xml);
    match &s.blocks[0] {
        ir::Block::List { items, .. } => {
            let text: String = items[0]
                .blocks
                .iter()
                .filter_map(|b| match b {
                    ir::Block::Paragraph { inlines } => {
                        Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
                    }
                    _ => None,
                })
                .collect();
            assert!(
                text.contains('\n'),
                "lineBreak in list item must produce newline; got: {text:?}"
            );
        }
        other => panic!("expected List, got {other:?}"),
    }
}

#[test]
fn resolve_bin_refs_inside_footnote() {
    let bin_map: HashMap<String, String> =
        [("BIN0002".to_string(), "BinData/BIN0002.jpg".to_string())]
            .into_iter()
            .collect();
    let mut section = ir::Section {
        blocks: vec![ir::Block::Footnote {
            id: "1".to_string(),
            content: vec![ir::Block::Image {
                src: "BIN0002".to_string(),
                alt: String::new(),
            }],
        }],
    };
    resolve_bin_refs(&mut section, &bin_map);
    match &section.blocks[0] {
        ir::Block::Footnote { content, .. } => match &content[0] {
            ir::Block::Image { src, .. } => {
                assert_eq!(src, "BinData/BIN0002.jpg");
            }
            other => panic!("expected Image, got {other:?}"),
        },
        other => panic!("expected Footnote, got {other:?}"),
    }
}

#[test]
fn resolve_bin_refs_inside_list() {
    let bin_map: HashMap<String, String> =
        [("BIN0003".to_string(), "BinData/BIN0003.png".to_string())]
            .into_iter()
            .collect();
    let mut section = ir::Section {
        blocks: vec![ir::Block::List {
            ordered: false,
            start: 1,
            items: vec![ir::ListItem {
                blocks: vec![ir::Block::Image {
                    src: "BIN0003".to_string(),
                    alt: String::new(),
                }],
                children: Vec::new(),
            }],
        }],
    };
    resolve_bin_refs(&mut section, &bin_map);
    match &section.blocks[0] {
        ir::Block::List { items, .. } => match &items[0].blocks[0] {
            ir::Block::Image { src, .. } => {
                assert_eq!(src, "BinData/BIN0003.png");
            }
            other => panic!("expected Image, got {other:?}"),
        },
        other => panic!("expected List, got {other:?}"),
    }
}
