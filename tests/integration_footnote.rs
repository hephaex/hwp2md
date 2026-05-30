/// Integration tests for HWPX footnote, endnote, and note-reference handling.
///
/// Covers: `<hp:fn>` footnote blocks, `<hp:en>` endnote blocks, `<hp:noteRef>`
/// inline references, orphan/dangling references, and the alternate
/// `<hp:ctrl id="fn" idRef="X"/>` reference path.
///
/// Extracted from integration.rs (Sprints 83–84) to keep each test file
/// focused.  PageBreak tests (Sprint 83 P3) remain in integration.rs as
/// they are not footnote-specific.
#[path = "fixtures/mod.rs"]
#[allow(dead_code)]
mod fixtures;

use fixtures::HwpxFixture;
use hwp2md::{hwpx, ir, md};

fn read_fixture(fixture: HwpxFixture) -> (tempfile::TempDir, ir::Document) {
    let (dir, path) = fixture.write_to_tempfile();
    let doc = hwpx::read_hwpx(&path).expect("read_hwpx failed");
    (dir, doc)
}

// ---------------------------------------------------------------------------
// Sprint 83 P2: Footnote/endnote full-pipeline integration
// ---------------------------------------------------------------------------

/// A `<hp:fn id="N">` element in a HWPX section produces an `ir::Block::Footnote`
/// with the correct id and content paragraph, and renders as `[^N]: text` in Markdown.
#[test]
fn hwpx_footnote_fn_element_produces_footnote_block_and_markdown() {
    let footnote_xml = r#"
        <hp:p><hp:run><hp:t>Body text.</hp:t></hp:run></hp:p>
        <hp:fn id="1">
            <hp:p><hp:run><hp:t>Note content.</hp:t></hp:run></hp:p>
        </hp:fn>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(footnote_xml));

    // IR layer: must contain a Footnote block.
    let footnote = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Footnote { id, content } = b {
                Some((id.as_str(), content.as_slice()))
            } else {
                None
            }
        });

    assert!(
        footnote.is_some(),
        "expected a Footnote block; blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    let (id, content) = footnote.unwrap();
    assert_eq!(id, "1", "footnote id mismatch");
    assert_eq!(content.len(), 1, "footnote must have exactly one content block");

    // Body paragraph must survive as a separate block alongside the Footnote block.
    let total_blocks: usize = doc.sections.iter().map(|s| s.blocks.len()).sum();
    assert_eq!(
        total_blocks,
        2,
        "section must contain body Paragraph + Footnote (2 blocks total); \
         blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );

    // Markdown layer: must render as [^1]: content
    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("[^1]:"),
        "markdown must contain footnote definition [^1]:; got: {markdown:?}"
    );
    assert!(
        markdown.contains("Note content"),
        "footnote content text must appear in markdown; got: {markdown:?}"
    );
}

/// An endnote (`<hp:en id="N">`) is treated identically to a footnote block.
/// Both produce `ir::Block::Footnote` and render as `[^N]:` in Markdown.
#[test]
fn hwpx_endnote_en_element_produces_footnote_block_and_markdown() {
    let endnote_xml = r#"
        <hp:p><hp:run><hp:t>Body text.</hp:t></hp:run></hp:p>
        <hp:en id="2">
            <hp:p><hp:run><hp:t>End note text.</hp:t></hp:run></hp:p>
        </hp:en>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(endnote_xml));

    let footnote = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Footnote { id, .. } = b {
                Some(id.as_str())
            } else {
                None
            }
        });

    assert_eq!(
        footnote,
        Some("2"),
        "expected Footnote block with id='2'; blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );

    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("[^2]:"),
        "endnote must render as [^2]:; got: {markdown:?}"
    );
    assert!(
        markdown.contains("End note text"),
        "endnote content must appear; got: {markdown:?}"
    );
}

/// A `<hp:noteRef noteId="N"/>` inline must produce an IR inline with
/// `footnote_ref = Some("N")`, rendered as `[^N]` in Markdown.
#[test]
fn hwpx_note_ref_inline_produces_footnote_ref_in_markdown() {
    let note_ref_xml = r#"
        <hp:p>
            <hp:run>
                <hp:t>See footnote</hp:t>
                <hp:noteRef noteId="1"/>
                <hp:t>.</hp:t>
            </hp:run>
        </hp:p>
        <hp:fn id="1">
            <hp:p><hp:run><hp:t>Referenced note.</hp:t></hp:run></hp:p>
        </hp:fn>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(note_ref_xml));

    // IR layer: the paragraph must have an inline with footnote_ref.
    let ref_inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.footnote_ref.is_some())
            } else {
                None
            }
        });

    assert!(
        ref_inline.is_some(),
        "expected an inline with footnote_ref; blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    assert_eq!(
        ref_inline.unwrap().footnote_ref.as_deref(),
        Some("1"),
        "footnote_ref must be '1'"
    );

    // Markdown layer: reference renders as [^1] (in body) AND [^1]: (definition).
    // Check occurrence count: body ref contributes one "[^1]", definition contributes
    // one more "[^1]" (as a prefix of "[^1]:"). Count >= 2 ensures the body ref
    // is present independently of the definition line.
    let markdown = md::write_markdown(&doc, false);
    let ref_count = markdown.matches("[^1]").count();
    assert!(
        ref_count >= 2,
        "markdown must contain both [^1] body reference and [^1]: definition; \
         found {ref_count} occurrence(s); got: {markdown:?}"
    );
    assert!(
        markdown.contains("[^1]:"),
        "markdown must contain [^1]: footnote definition; got: {markdown:?}"
    );
}

// ---------------------------------------------------------------------------
// Sprint 84 P2: orphan noteRef (no matching footnote definition)
// ---------------------------------------------------------------------------

/// A `<hp:noteRef noteId="99"/>` with no matching `<hp:fn id="99">` is a
/// dangling reference. The reader emits the `[^99]` body reference regardless
/// because reference and definition are independent in the current IR model.
/// The Markdown output must contain `[^99]` (body ref) but NOT `[^99]:`
/// (definition) — this pins the graceful-degradation behaviour.
#[test]
fn hwpx_orphan_note_ref_body_rendered_without_definition() {
    let orphan_ref_xml = r#"
        <hp:p>
            <hp:run>
                <hp:t>See note</hp:t>
                <hp:noteRef noteId="99"/>
                <hp:t>.</hp:t>
            </hp:run>
        </hp:p>"#;
    // No <hp:fn id="99"> present — dangling reference.

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(orphan_ref_xml));

    // IR layer: must still produce the footnote_ref inline.
    let ref_inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.footnote_ref.is_some())
            } else {
                None
            }
        });
    assert!(
        ref_inline.is_some(),
        "orphan noteRef must still produce a footnote_ref inline; \
         blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    assert_eq!(ref_inline.unwrap().footnote_ref.as_deref(), Some("99"));

    // Markdown layer: body reference present, definition absent.
    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("[^99]"),
        "orphan reference must appear as [^99] in Markdown; got: {markdown:?}"
    );
    assert!(
        !markdown.contains("[^99]:"),
        "orphan reference must NOT have a [^99]: definition; got: {markdown:?}"
    );
}

// ---------------------------------------------------------------------------
// Sprint 84 P3: <hp:ctrl id="fn" idRef="X"/> alternate footnote-ref path
// ---------------------------------------------------------------------------

/// `<hp:ctrl id="fn" idRef="1"/>` is the alternate mechanism for a footnote
/// reference inline (handlers.rs:487 — the `"ctrl"` element path, as opposed
/// to the `"noteRef"` path at handlers.rs:462).  Both must produce the same
/// `ir::Inline.footnote_ref` result.
#[test]
fn hwpx_ctrl_fn_idref_produces_footnote_ref_inline() {
    let ctrl_fn_xml = r#"
        <hp:p>
            <hp:run>
                <hp:t>Body text</hp:t>
                <hp:ctrl id="fn" idRef="1"/>
            </hp:run>
        </hp:p>
        <hp:fn id="1">
            <hp:p><hp:run><hp:t>Ctrl path note.</hp:t></hp:run></hp:p>
        </hp:fn>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(ctrl_fn_xml));

    // IR: footnote_ref inline must exist with id="1".
    let ref_inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.footnote_ref.is_some())
            } else {
                None
            }
        });
    assert!(
        ref_inline.is_some(),
        "<hp:ctrl id='fn' idRef='1'/> must produce footnote_ref inline; \
         blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    assert_eq!(
        ref_inline.unwrap().footnote_ref.as_deref(),
        Some("1"),
        "footnote_ref id mismatch"
    );

    // Markdown: [^1] reference + [^1]: definition.
    let markdown = md::write_markdown(&doc, false);
    let count = markdown.matches("[^1]").count();
    assert!(
        count >= 2,
        "must have both [^1] body ref and [^1]: definition; \
         count={count}; got: {markdown:?}"
    );
    assert!(
        markdown.contains("[^1]:"),
        "must have [^1]: definition; got: {markdown:?}"
    );
}
