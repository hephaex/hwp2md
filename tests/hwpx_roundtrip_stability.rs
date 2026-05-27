//! HWPX → MD → HWPX → MD stability tests.
//!
//! Verifies that converting HWPX → MD → HWPX → MD is idempotent: the second
//! Markdown output is identical to the first.  Each test builds a synthetic
//! HWPX file from [`HwpxFixture`], runs the four-step pipeline, and asserts
//! that `md1 == md2`.
//!
//! Pipeline:
//! ```text
//! HwpxFixture → write_to_tempfile
//!   → read_hwpx  →  write_markdown          = md1
//!   → parse_markdown → IR
//!   → write_hwpx → tempfile2
//!   → read_hwpx  →  write_markdown          = md2
//!   → assert md1 == md2
//! ```

use hwp2md::hwpx::{read_hwpx, write_hwpx};
use hwp2md::md::{parse_markdown, write_markdown};

#[path = "fixtures/mod.rs"]
mod fixtures;

use fixtures::{heading_xml, para_xml, table_2x2_xml, HwpxFixture};

// ---------------------------------------------------------------------------
// Helper: HWPX path → md1, md2
// ---------------------------------------------------------------------------

/// Runs the HWPX → MD → HWPX → MD pipeline and returns `(md1, md2)`.
///
/// `hwpx_path` must point to a readable HWPX file.  Both temporary HWPX files
/// are owned by the caller via `_guard` (dropped on return, but we use them
/// inline so the guard lifetime doesn't matter here).
fn hwpx_to_md_to_hwpx_to_md(hwpx_path: &std::path::Path) -> (String, String) {
    // Step 1: HWPX → IR → md1
    let doc1 = read_hwpx(hwpx_path).expect("read_hwpx pass 1 must succeed");
    let md1 = write_markdown(&doc1, false);

    // Step 2: md1 → IR → HWPX (second file)
    let doc2 = parse_markdown(&md1);
    let tmp2 = tempfile::NamedTempFile::new().expect("tempfile for pass 2");
    write_hwpx(&doc2, tmp2.path(), None).expect("write_hwpx pass 2 must succeed");

    // Step 3: second HWPX → IR → md2
    let doc3 = read_hwpx(tmp2.path()).expect("read_hwpx pass 2 must succeed");
    let md2 = write_markdown(&doc3, false);

    (md1, md2)
}

// ---------------------------------------------------------------------------
// Test 1: paragraph-only fixture
// ---------------------------------------------------------------------------

/// A fixture with two plain paragraphs.  After two full HWPX ↔ MD trips the
/// Markdown output must be identical (idempotent).
#[test]
fn hwpx_md_hwpx_md_paragraph_stable() {
    let p1 = para_xml("Hello stability world");
    let p2 = para_xml("Second paragraph content");

    let (_guard, path) = HwpxFixture::new()
        .title("Paragraph Stability")
        .section(&p1)
        .section(&p2)
        .write_to_tempfile();

    let (md1, md2) = hwpx_to_md_to_hwpx_to_md(&path);

    // Both passes must carry the key text.
    assert!(
        md1.contains("Hello stability world"),
        "paragraph text missing from md1; md1: {md1:?}"
    );
    assert!(
        md1.contains("Second paragraph"),
        "second paragraph missing from md1; md1: {md1:?}"
    );

    // Stability: second pass output must equal first pass output.
    assert_eq!(
        md1, md2,
        "HWPX→MD→HWPX→MD is not stable for paragraph fixture\n\
         md1:\n{md1}\n\nmd2:\n{md2}"
    );
}

// ---------------------------------------------------------------------------
// Test 2: heading + table fixture
// ---------------------------------------------------------------------------

/// A fixture with an H1 heading and a 2×2 table.  The double roundtrip must
/// preserve all four cell values and the heading text, and md1 == md2.
#[test]
fn hwpx_md_hwpx_md_heading_and_table_stable() {
    let h1 = heading_xml(1, "Stability Report");
    let tbl = table_2x2_xml("Header A", "Header B", "Value 1", "Value 2");

    let (_guard, path) = HwpxFixture::new()
        .title("Heading Table Stability")
        .section(&h1)
        .section(&tbl)
        .write_to_tempfile();

    let (md1, md2) = hwpx_to_md_to_hwpx_to_md(&path);

    // Content checks on the first pass.
    assert!(
        md1.contains("Stability Report"),
        "heading text missing from md1; md1: {md1:?}"
    );
    for cell in &["Header A", "Header B", "Value 1", "Value 2"] {
        assert!(
            md1.contains(cell),
            "table cell {cell:?} missing from md1; md1: {md1:?}"
        );
    }

    // Stability assertion.
    assert_eq!(
        md1, md2,
        "HWPX→MD→HWPX→MD is not stable for heading+table fixture\n\
         md1:\n{md1}\n\nmd2:\n{md2}"
    );
}

// ---------------------------------------------------------------------------
// Test 3: code block fixture
// ---------------------------------------------------------------------------

/// A fixture containing a code-block paragraph preceded by a
/// `<!-- hwp2md:lang:rust -->` XML comment.  The comment causes the reader to
/// surface the paragraph as a `CodeBlock` with `language = Some("rust")` in
/// the first pass; the writer then emits another lang-hint comment, so the
/// second pass reproduces an identical `CodeBlock`.  This exercises the
/// `CodeLangHint::Code(lang)` path end-to-end.
#[test]
fn hwpx_md_hwpx_md_code_block_stable() {
    // The lang-hint comment must appear as a sibling XML node immediately
    // before the paragraph inside <hs:sec>.  Both the comment and the
    // paragraph are injected via `.section()` as raw XML.
    let code_text = "fn add(a: i32, b: i32) -> i32 { a + b }";
    let lang_comment = "<!-- hwp2md:lang:rust -->";
    let p = para_xml(code_text);

    let (_guard, path) = HwpxFixture::new()
        .title("CodeBlock Stability")
        .section(lang_comment)
        .section(&p)
        .write_to_tempfile();

    let (md1, md2) = hwpx_to_md_to_hwpx_to_md(&path);

    // First pass must produce a fenced code block with language annotation.
    assert!(
        md1.contains("```rust"),
        "fenced rust code block missing from md1; md1: {md1:?}"
    );
    // The code text must survive both passes.
    assert!(
        md1.contains("fn add"),
        "code text missing from md1; md1: {md1:?}"
    );
    assert!(
        md2.contains("fn add"),
        "code text missing from md2; md2: {md2:?}"
    );

    // Stability: content must be identical after the second trip.
    assert_eq!(
        md1, md2,
        "HWPX→MD→HWPX→MD is not stable for code-block fixture\n\
         md1:\n{md1}\n\nmd2:\n{md2}"
    );
}
