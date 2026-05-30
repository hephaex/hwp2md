/// Integration tests for HWPX list handling.
///
/// Covers both the secondary `<ol>/<ul>/<li>` ingestion path (handlers.rs)
/// and the canonical OWPML flat-paragraph encoding (`paraPrIDRef="2"` +
/// optional `numPrIDRef="1"`), plus nested list depth folding.
///
/// Extracted from integration.rs (Sprints 85-87) to keep each test file
/// focused.  MD→HWPX→MD roundtrip tests for ordered/unordered lists remain
/// in integration.rs as they are thematically "roundtrip" rather than
/// "HWPX fixture → IR" tests.
#[path = "fixtures/mod.rs"]
#[allow(dead_code)]
mod fixtures;

use fixtures::{read_fixture, HwpxFixture};
use hwp2md::{ir, md};

// ---------------------------------------------------------------------------
// Sprint 85 P2: ordered/unordered list integration tests
// ---------------------------------------------------------------------------
//
// NOTE: Real OWPML/HWPX files encode lists as flat <hp:p paraPrIDRef numPrIDRef>
// paragraphs (handled in flush.rs via group_list_paragraphs). The <ol>/<ul>/<li>
// form below is a secondary/lenient ingestion path in handlers.rs:81-95.
// Canonical OWPML list coverage lives in reader_tests_list.rs and
// real_hwp_list_accuracy.rs; these tests pin the secondary ol/ul/li handler.

/// `<ol><li>` produces `ir::Block::List { ordered: true }` with item text
/// preserved, and renders as a GFM numbered list in Markdown.
#[test]
fn hwpx_ol_li_produces_ordered_list_block() {
    let list_xml = r#"
        <ol>
            <li><hp:run><hp:t>First item</hp:t></hp:run></li>
            <li><hp:run><hp:t>Second item</hp:t></hp:run></li>
        </ol>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(list_xml));

    // IR layer: must contain an ordered List block.
    let list_block = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find(|b| matches!(b, ir::Block::List { ordered: true, .. }));

    assert!(
        list_block.is_some(),
        "expected Block::List {{ ordered: true }}; blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    let ir::Block::List { items, .. } = list_block.unwrap() else {
        unreachable!()
    };
    assert_eq!(items.len(), 2, "ordered list must have 2 items");

    // Markdown layer: GFM numbered list format with correct sequence.
    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("1. ") && markdown.contains("First item"),
        "markdown must contain '1. First item'; got: {markdown:?}"
    );
    // Assert the second item uses "2." (not "1.") to verify sequential numbering.
    assert!(
        markdown.contains("2. ") && markdown.contains("Second item"),
        "markdown must contain '2. Second item' (sequential); got: {markdown:?}"
    );
}

/// `<ul><li>` produces `ir::Block::List { ordered: false }` and renders as
/// a GFM bullet list (`-`) in Markdown.
#[test]
fn hwpx_ul_li_produces_unordered_list_block() {
    let list_xml = r#"
        <ul>
            <li><hp:run><hp:t>Apple</hp:t></hp:run></li>
            <li><hp:run><hp:t>Banana</hp:t></hp:run></li>
            <li><hp:run><hp:t>Cherry</hp:t></hp:run></li>
        </ul>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(list_xml));

    // IR layer: must contain an unordered List block.
    let list_block = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find(|b| matches!(b, ir::Block::List { ordered: false, .. }));

    assert!(
        list_block.is_some(),
        "expected Block::List {{ ordered: false }}; blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    let ir::Block::List { items, .. } = list_block.unwrap() else {
        unreachable!()
    };
    assert_eq!(items.len(), 3, "unordered list must have 3 items");

    // Markdown layer: GFM bullet list format. The writer hardcodes "-" as
    // the unordered marker (writer.rs uses "-".to_string()), never "*".
    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("- Apple"),
        "markdown must contain '- Apple' (hyphen bullet); got: {markdown:?}"
    );
    assert!(
        markdown.contains("Banana") && markdown.contains("Cherry"),
        "all items must appear; got: {markdown:?}"
    );
}

// ---------------------------------------------------------------------------
// Sprint 86 P2: canonical OWPML flat-paragraph list integration tests
// ---------------------------------------------------------------------------
//
// Real HWPX/OWPML encodes lists as FLAT <hp:p paraPrIDRef="2"> paragraphs with
// optional numPrIDRef="1" for ordered.  The reader groups consecutive list-paragraph
// sentinels via `group_list_paragraphs` (flush.rs/reader.rs).
//
// paraPrIDRef="2" → depth-0 list item (PARA_PR_LIST_D0)
// numPrIDRef="1"  → ordered (absent = unordered)

/// Three flat `<hp:p paraPrIDRef="2" numPrIDRef="1">` paragraphs in section
/// XML → single `Block::List { ordered: true, items: 3 }` after grouping.
#[test]
fn hwpx_canonical_ordered_list_flat_para_produces_list_block() {
    // This is the real-world OWPML encoding (NOT <ol>/<li> elements).
    let list_xml = r#"
        <hp:p paraPrIDRef="2" numPrIDRef="1"><hp:run><hp:t>Alpha</hp:t></hp:run></hp:p>
        <hp:p paraPrIDRef="2" numPrIDRef="1"><hp:run><hp:t>Beta</hp:t></hp:run></hp:p>
        <hp:p paraPrIDRef="2" numPrIDRef="1"><hp:run><hp:t>Gamma</hp:t></hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(list_xml));

    let list_block = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find(|b| matches!(b, ir::Block::List { ordered: true, .. }));

    assert!(
        list_block.is_some(),
        "expected Block::List {{ ordered: true }} from paraPrIDRef=2 numPrIDRef=1; \
         blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    let ir::Block::List { items, .. } = list_block.unwrap() else {
        unreachable!()
    };
    assert_eq!(items.len(), 3, "must have 3 list items");

    // Markdown: sequential numbered items.
    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("1. ") && markdown.contains("Alpha"),
        "must contain '1. Alpha'; got: {markdown:?}"
    );
    assert!(
        markdown.contains("3. ") && markdown.contains("Gamma"),
        "must contain '3. Gamma' (sequential); got: {markdown:?}"
    );
}

/// Three flat `<hp:p paraPrIDRef="2">` paragraphs WITHOUT numPrIDRef
/// → single `Block::List { ordered: false, items: 3 }` (unordered).
#[test]
fn hwpx_canonical_unordered_list_flat_para_produces_list_block() {
    let list_xml = r#"
        <hp:p paraPrIDRef="2"><hp:run><hp:t>Red</hp:t></hp:run></hp:p>
        <hp:p paraPrIDRef="2"><hp:run><hp:t>Green</hp:t></hp:run></hp:p>
        <hp:p paraPrIDRef="2"><hp:run><hp:t>Blue</hp:t></hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(list_xml));

    let list_block = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find(|b| matches!(b, ir::Block::List { ordered: false, .. }));

    assert!(
        list_block.is_some(),
        "expected Block::List {{ ordered: false }} from paraPrIDRef=2 without numPrIDRef; \
         blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    let ir::Block::List { items, .. } = list_block.unwrap() else {
        unreachable!()
    };
    assert_eq!(items.len(), 3, "must have 3 unordered list items");

    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("- Red"),
        "must contain '- Red'; got: {markdown:?}"
    );
    assert!(
        markdown.contains("- Blue"),
        "must contain '- Blue'; got: {markdown:?}"
    );
}

// ---------------------------------------------------------------------------
// Sprint 87 P4: nested list (depth-1, paraPrIDRef="3") integration test
// ---------------------------------------------------------------------------

/// A depth-1 item (`paraPrIDRef="3"`) immediately following a depth-0 item
/// (`paraPrIDRef="2"`) must appear as a child of that item, not a new top-level
/// item. Tests the `build_list` depth folding logic (flush.rs:379-415).
#[test]
fn hwpx_nested_list_depth1_becomes_child_of_parent_item() {
    // Pattern: top → nested → top (should produce 2 top-level items, second with a child)
    let list_xml = r#"
        <hp:p paraPrIDRef="2"><hp:run><hp:t>Parent A</hp:t></hp:run></hp:p>
        <hp:p paraPrIDRef="3"><hp:run><hp:t>Child of A</hp:t></hp:run></hp:p>
        <hp:p paraPrIDRef="2"><hp:run><hp:t>Parent B</hp:t></hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(list_xml));

    let list_block = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find(|b| matches!(b, ir::Block::List { .. }));

    assert!(
        list_block.is_some(),
        "expected Block::List; blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    let ir::Block::List { items, .. } = list_block.unwrap() else {
        unreachable!()
    };

    // Must have exactly 2 top-level items (not 3 — depth-1 is a child).
    assert_eq!(
        items.len(),
        2,
        "depth-1 item must be a child, not a 3rd top-level item; items: {items:?}"
    );

    // First item must have exactly 1 child.
    let parent_a = &items[0];
    assert_eq!(
        parent_a.children.len(),
        1,
        "Parent A must have 1 child; children: {:?}",
        parent_a.children
    );

    // Second item must have no children.
    assert_eq!(
        items[1].children.len(),
        0,
        "Parent B must have no children; children: {:?}",
        items[1].children
    );

    // Markdown layer: verify nested list renders with indented bullet.
    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("Parent A"),
        "Parent A must appear; got: {markdown:?}"
    );
    assert!(
        markdown.contains("Child of A"),
        "Child must appear; got: {markdown:?}"
    );
    assert!(
        markdown.contains("Parent B"),
        "Parent B must appear; got: {markdown:?}"
    );
    // Child must appear after Parent A but before Parent B.
    let pos_a = markdown.find("Parent A").unwrap();
    let pos_child = markdown.find("Child of A").unwrap();
    let pos_b = markdown.find("Parent B").unwrap();
    assert!(
        pos_a < pos_child && pos_child < pos_b,
        "order: Parent A < Child < Parent B; got: {markdown:?}"
    );
}
