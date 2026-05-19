use super::*;

// ── Table writer tests ────────────────────────────────────────────────────
//
// Verify that Block::Table is emitted as a fully-compliant OWPML `<hp:tbl>`
// element containing all required child elements, and that cell text is
// preserved across a write-then-read roundtrip.

// ── helpers ───────────────────────────────────────────────────────────────

/// Build a `Document` containing a single table with the given rows.
fn table_doc(rows: Vec<ir::TableRow>) -> Document {
    let col_count = rows.iter().map(|r| r.cells.len()).max().unwrap_or(0);
    Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::Table { rows, col_count }],
            page_layout: None,
            ..Default::default()
        }],
        assets: Vec::new(),
    }
}

/// Construct a plain-text `TableCell` with a single paragraph.
fn text_cell(text: &str) -> ir::TableCell {
    ir::TableCell {
        blocks: vec![Block::Paragraph {
            inlines: vec![Inline::plain(text)],
        }],
        colspan: 1,
        rowspan: 1,
    }
}

/// Construct a `TableRow` from a slice of cell text strings.
fn text_row(texts: &[&str], is_header: bool) -> ir::TableRow {
    ir::TableRow {
        cells: texts.iter().map(|t| text_cell(t)).collect(),
        is_header,
    }
}

// ── Test: required OWPML elements present in section XML ─────────────────

/// A 2-column × 3-row table must produce section XML that contains every
/// mandatory OWPML child element of `<hp:tbl>`.
#[test]
fn write_table_2x3_has_required_elements() {
    let rows = vec![
        text_row(&["A1", "A2"], true),
        text_row(&["B1", "B2"], false),
        text_row(&["C1", "C2"], false),
    ];
    let doc = table_doc(rows);
    let tables = RefTables::build(&doc, None);
    let sec = &doc.sections[0];
    let asset_map = ImageAssetMap::new();
    let xml =
        generate_section_xml(sec, 0, &tables, &asset_map).expect("generate_section_xml failed");

    assert!(xml.contains("<hp:tbl "), "must contain <hp:tbl>: {xml}");
    assert!(
        xml.contains("<hp:tblPr>"),
        "must contain <hp:tblPr>: {xml}"
    );
    assert!(
        xml.contains("<hp:inMargin "),
        "must contain <hp:inMargin>: {xml}"
    );
    assert!(
        xml.contains(r#"borderFillIDRef="2""#),
        "tbl must reference borderFill id=2: {xml}"
    );
    assert!(
        xml.contains(r#"noAdjust="0""#),
        "tbl must have noAdjust=\"0\": {xml}"
    );
    assert!(xml.contains("<hp:sz "), "must contain <hp:sz>: {xml}");
    assert!(xml.contains("<hp:pos "), "must contain <hp:pos>: {xml}");
    assert!(
        xml.contains("<hp:trHeight "),
        "must contain <hp:trHeight>: {xml}"
    );
    assert!(
        xml.contains("<hp:cellAddr "),
        "must contain <hp:cellAddr>: {xml}"
    );
    assert!(
        xml.contains("<hp:cellSpan "),
        "must contain <hp:cellSpan>: {xml}"
    );
    assert!(
        xml.contains("<hp:cellSz "),
        "must contain <hp:cellSz>: {xml}"
    );
    assert!(
        xml.contains("<hp:cellMargin "),
        "must contain <hp:cellMargin>: {xml}"
    );
    assert!(
        xml.contains("<hp:subList>"),
        "must contain <hp:subList>: {xml}"
    );
    assert!(
        xml.contains("</hp:subList>"),
        "must close </hp:subList>: {xml}"
    );
}

/// The `<hp:tbl>` element must carry the correct `rowCnt` and `colCnt`
/// attributes derived from the IR table dimensions.
#[test]
fn write_table_row_col_count_attributes() {
    let rows = vec![
        text_row(&["A1", "A2", "A3"], true),
        text_row(&["B1", "B2", "B3"], false),
    ];
    let doc = table_doc(rows);
    let tables = RefTables::build(&doc, None);
    let sec = &doc.sections[0];
    let asset_map = ImageAssetMap::new();
    let xml =
        generate_section_xml(sec, 0, &tables, &asset_map).expect("generate_section_xml failed");

    assert!(
        xml.contains(r#"rowCnt="2""#),
        "rowCnt must be 2: {xml}"
    );
    assert!(
        xml.contains(r#"colCnt="3""#),
        "colCnt must be 3: {xml}"
    );
}

/// Cell address attributes `colAddr` and `rowAddr` must reflect the
/// zero-based column and row indices of each cell.
#[test]
fn write_table_cell_addr_indices() {
    let rows = vec![
        text_row(&["R0C0", "R0C1"], true),
        text_row(&["R1C0", "R1C1"], false),
    ];
    let doc = table_doc(rows);
    let tables = RefTables::build(&doc, None);
    let sec = &doc.sections[0];
    let asset_map = ImageAssetMap::new();
    let xml =
        generate_section_xml(sec, 0, &tables, &asset_map).expect("generate_section_xml failed");

    assert!(
        xml.contains(r#"colAddr="0" rowAddr="0""#),
        "first cell must have colAddr=0 rowAddr=0: {xml}"
    );
    assert!(
        xml.contains(r#"colAddr="1" rowAddr="0""#),
        "second cell of first row must have colAddr=1 rowAddr=0: {xml}"
    );
    assert!(
        xml.contains(r#"colAddr="0" rowAddr="1""#),
        "first cell of second row must have colAddr=0 rowAddr=1: {xml}"
    );
}

/// header.xml must contain a second `<hh:borderFill>` entry with id="2"
/// (the solid-border entry referenced by table cells).
#[test]
fn header_xml_contains_border_fill_id_2() {
    let rows = vec![text_row(&["cell"], false)];
    let doc = table_doc(rows);
    let tables = RefTables::build(&doc, None);
    let header =
        super::header::generate_header_xml(&doc, &tables).expect("generate_header_xml failed");

    assert!(
        header.contains(r#"id="2""#),
        "header must contain borderFill id=\"2\": {header}"
    );
    assert!(
        header.contains(r#"itemCnt="2""#),
        "borderFills must declare itemCnt=\"2\": {header}"
    );
}

// ── Test: roundtrip cell text preservation ────────────────────────────────

/// Write a table document to HWPX, read it back, and verify that the text
/// content of all cells is preserved.
#[test]
fn write_table_roundtrip_cell_text() {
    let rows = vec![
        text_row(&["Alpha", "Beta"], true),
        text_row(&["Gamma", "Delta"], false),
    ];
    let doc = table_doc(rows);

    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");

    let read_back = read_hwpx(tmp.path()).expect("read_hwpx");

    // Find the first Table block in the read-back document.
    let table_block = read_back
        .sections
        .into_iter()
        .flat_map(|s| s.blocks)
        .find_map(|b| match b {
            Block::Table { rows, .. } => Some(rows),
            _ => None,
        })
        .expect("no Table block found after roundtrip");

    assert_eq!(table_block.len(), 2, "expected 2 rows: {table_block:?}");
    assert_eq!(
        table_block[0].cells.len(),
        2,
        "first row must have 2 cells: {table_block:?}"
    );

    // Extract flat text from a cell's block content.
    let cell_text = |cell: &ir::TableCell| -> String {
        cell.blocks
            .iter()
            .flat_map(|b| match b {
                Block::Paragraph { inlines } => inlines.iter().map(|i| i.text.as_str()).collect::<Vec<_>>(),
                _ => vec![],
            })
            .collect::<Vec<_>>()
            .join("")
    };

    assert_eq!(
        cell_text(&table_block[0].cells[0]),
        "Alpha",
        "cell [0][0] text mismatch"
    );
    assert_eq!(
        cell_text(&table_block[0].cells[1]),
        "Beta",
        "cell [0][1] text mismatch"
    );
    assert_eq!(
        cell_text(&table_block[1].cells[0]),
        "Gamma",
        "cell [1][0] text mismatch"
    );
    assert_eq!(
        cell_text(&table_block[1].cells[1]),
        "Delta",
        "cell [1][1] text mismatch"
    );
}

/// A cell with `colspan=2, rowspan=1` must emit `<hp:cellSz width="16000"` (2 × 8000).
///
/// The writer scales `width` by `colspan` so that merged cells span the correct
/// horizontal extent in OWPML.  A normal cell (`colspan=1`) must still have
/// `width="8000"`.
#[test]
fn write_table_colspan_cellsz_scaled() {
    // Row 0: one merged cell (colspan=2) followed by a normal cell (colspan=1).
    let merged_cell = ir::TableCell {
        blocks: vec![Block::Paragraph {
            inlines: vec![Inline::plain("merged")],
        }],
        colspan: 2,
        rowspan: 1,
    };
    let normal_cell = ir::TableCell {
        blocks: vec![Block::Paragraph {
            inlines: vec![Inline::plain("normal")],
        }],
        colspan: 1,
        rowspan: 1,
    };
    let rows = vec![ir::TableRow {
        cells: vec![merged_cell, normal_cell],
        is_header: false,
    }];
    // col_count reflects the *logical* column count (3 columns: 2 merged + 1).
    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::Table { rows, col_count: 3 }],
            page_layout: None,
            ..Default::default()
        }],
        assets: Vec::new(),
    };
    let tables = RefTables::build(&doc, None);
    let sec = &doc.sections[0];
    let asset_map = ImageAssetMap::new();
    let xml =
        generate_section_xml(sec, 0, &tables, &asset_map).expect("generate_section_xml failed");

    // Merged cell (colspan=2) must emit exactly one width="16000".
    let count_16000 = xml.matches(r#"width="16000""#).count();
    assert_eq!(
        count_16000, 1,
        "expected exactly 1 occurrence of width=\"16000\" (merged cell): {xml}"
    );
    // Normal cell (colspan=1) must emit exactly one width="8000".
    let count_8000 = xml.matches(r#"width="8000""#).count();
    assert_eq!(
        count_8000, 1,
        "expected exactly 1 occurrence of width=\"8000\" (normal cell): {xml}"
    );
}

/// A cell with `colspan=2, rowspan=1` must emit `<hp:cellSpan colSpan="2" rowSpan="1"/>`.
#[test]
fn write_table_colspan_emitted_in_cell_span() {
    let rows = vec![ir::TableRow {
        cells: vec![ir::TableCell {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::plain("merged")],
            }],
            colspan: 2,
            rowspan: 1,
        }],
        is_header: false,
    }];
    let doc = table_doc(rows);
    let tables = RefTables::build(&doc, None);
    let sec = &doc.sections[0];
    let asset_map = ImageAssetMap::new();
    let xml =
        generate_section_xml(sec, 0, &tables, &asset_map).expect("generate_section_xml failed");

    assert!(
        xml.contains(r#"<hp:cellSpan colSpan="2" rowSpan="1""#),
        "cellSpan must reflect colspan=2: {xml}"
    );
}

/// The `<hp:tbl>` element must contain a `<hp:tblPr>` child with an
/// `<hp:inMargin>` specifying the inner cell gap.
#[test]
fn write_table_has_tblpr() {
    let rows = vec![text_row(&["A", "B"], false)];
    let doc = table_doc(rows);
    let tables = RefTables::build(&doc, None);
    let sec = &doc.sections[0];
    let asset_map = ImageAssetMap::new();
    let xml =
        generate_section_xml(sec, 0, &tables, &asset_map).expect("generate_section_xml failed");

    assert!(
        xml.contains("<hp:tblPr>"),
        "must contain <hp:tblPr>: {xml}"
    );
    assert!(
        xml.contains("</hp:tblPr>"),
        "must close </hp:tblPr>: {xml}"
    );
    assert!(
        xml.contains("<hp:inMargin "),
        "must contain <hp:inMargin>: {xml}"
    );
    // Extract the inMargin element to check all four attributes without
    // false positives from cellMargin which also uses "141".
    let im_start = xml.find("<hp:inMargin ").expect("<hp:inMargin> missing");
    let im_end = xml[im_start..].find("/>").expect("/>") + im_start;
    let im_tag = &xml[im_start..=im_end + 1];
    assert!(im_tag.contains(r#"left="141""#),   "inMargin left must be 141: {im_tag}");
    assert!(im_tag.contains(r#"right="141""#),  "inMargin right must be 141: {im_tag}");
    assert!(im_tag.contains(r#"top="141""#),    "inMargin top must be 141: {im_tag}");
    assert!(im_tag.contains(r#"bottom="141""#), "inMargin bottom must be 141: {im_tag}");

    // tblPr must appear before sz (OWPML child-order requirement).
    let tblpr_pos = xml.find("<hp:tblPr>").expect("<hp:tblPr> missing");
    let sz_pos = xml.find("<hp:sz ").expect("<hp:sz> missing");
    assert!(
        tblpr_pos < sz_pos,
        "<hp:tblPr> must appear before <hp:sz>: tblPr@{tblpr_pos}, sz@{sz_pos}"
    );
}
