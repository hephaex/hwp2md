/// Integration tests for table colspan handling and code block without language.
///
/// Covers two Sprint 85 tests that don't naturally fit other files:
/// - colspan → HTML fallback rendering (table-specific feature)
/// - empty language hint → `CodeBlock { language: None }` (code block edge case)
///
/// Extracted from integration.rs (Sprint 85 P3/P4) to keep each file focused.
#[path = "fixtures/mod.rs"]
#[allow(dead_code)]
mod fixtures;

use fixtures::{read_fixture, HwpxFixture};
use hwp2md::{ir, md};

// ---------------------------------------------------------------------------
// Sprint 85 P3: table colspan → HTML fallback integration test
// ---------------------------------------------------------------------------

/// A table cell with `colSpan="2"` triggers the HTML fallback renderer.
/// The Markdown writer must emit an HTML `<table>` with `colspan="2"` when
/// any cell has colspan > 1.
#[test]
fn hwpx_table_with_colspan_renders_html_fallback() {
    // 2×2 table where the first row has a single cell spanning 2 columns.
    // OWPML encodes colspan via <hp:cellSpan colSpan="N"/> child element,
    // not as an attribute on <hp:tc> itself.
    let table_xml = r#"<hp:p paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:tbl rowCnt="2" colCnt="2">
        <hp:tr>
            <hp:tc>
                <hp:cellSpan colSpan="2" rowSpan="1"/>
                <hp:p paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>Merged Header</hp:t></hp:run></hp:p>
            </hp:tc>
        </hp:tr>
        <hp:tr>
            <hp:tc>
                <hp:p paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>Cell A</hp:t></hp:run></hp:p>
            </hp:tc>
            <hp:tc>
                <hp:p paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>Cell B</hp:t></hp:run></hp:p>
            </hp:tc>
        </hp:tr>
    </hp:tbl></hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(table_xml));

    // IR layer: must contain a Table block.
    let table_block = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find(|b| matches!(b, ir::Block::Table { .. }));

    assert!(
        table_block.is_some(),
        "expected a Table block; blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );

    // Check that the merged cell carries colspan=2 in the IR.
    let ir::Block::Table { rows, .. } = table_block.unwrap() else {
        unreachable!()
    };
    let first_row_cell = rows.first().and_then(|r| r.cells.first());
    assert!(
        first_row_cell.is_some(),
        "first row must have at least one cell"
    );
    assert_eq!(
        first_row_cell.unwrap().colspan,
        2,
        "merged cell must carry colspan=2 in the IR"
    );

    // Markdown layer: must fall back to HTML <table> with colspan attribute.
    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("<table>") || markdown.contains("<TABLE>"),
        "colspan table must render as HTML <table>; got: {markdown:?}"
    );
    assert!(
        markdown.contains("colspan=\"2\"") || markdown.contains("colspan=2"),
        "HTML table must include colspan=2; got: {markdown:?}"
    );
    assert!(
        markdown.contains("Merged Header"),
        "merged cell text must appear in output; got: {markdown:?}"
    );
}

// ---------------------------------------------------------------------------
// Sprint 85 P4: no-language code block integration test
// ---------------------------------------------------------------------------

/// `<!-- hwp2md:lang: -->` (empty language) produces a `CodeBlock` with
/// `language = None` and renders as a plain ``` fence in Markdown (no tag).
#[test]
fn hwpx_lang_hint_empty_language_produces_code_block_no_tag() {
    let (_dir, doc) = read_fixture(
        // Pass empty string → <!-- hwp2md:lang: --> → CodeBlock { language: None }
        HwpxFixture::new().with_lang_hint_paragraph("", "no_lang_code()"),
    );

    let code_block = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find(|b| matches!(b, ir::Block::CodeBlock { .. }));

    assert!(
        code_block.is_some(),
        "expected a CodeBlock; blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    let ir::Block::CodeBlock { language, code } = code_block.unwrap() else {
        unreachable!()
    };
    assert!(
        language.is_none(),
        "empty lang hint must produce CodeBlock {{ language: None }}; got: {language:?}"
    );
    assert!(
        code.contains("no_lang_code"),
        "code content must be preserved; got: {code:?}"
    );

    // Markdown layer: ``` fence without a language tag.
    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("```"),
        "markdown must contain code fence; got: {markdown:?}"
    );
    // Must NOT have a language tag immediately after the opening fence.
    assert!(
        !markdown.contains("```no_lang") && !markdown.contains("```None"),
        "no-language fence must not have a language tag; got: {markdown:?}"
    );
}
