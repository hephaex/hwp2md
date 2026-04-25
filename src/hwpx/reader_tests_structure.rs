use super::*;

// -----------------------------------------------------------------------
// Helper: unwrap the section and panic with a descriptive message on error.
// -----------------------------------------------------------------------

fn section(xml: &str) -> ir::Section {
    parse_section_xml(xml).expect("parse_section_xml must not fail")
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
