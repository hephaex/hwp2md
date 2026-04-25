/// Integration tests using real HWPX fixtures built in memory.
///
/// Each test constructs a minimal, valid HWPX ZIP using the `HwpxFixture`
/// builder, writes it to a temporary file, and exercises the library API
/// (`hwpx::read_hwpx`, `md::write_markdown`, `convert::to_markdown`).
///
/// Binary HWP 5.0 is deliberately excluded: its compound-file-binary format
/// requires a real HWP runtime to produce valid files.  HWPX (ZIP + XML) is
/// fully constructable from pure Rust.
// Make the fixture helpers available as `fixtures::HwpxFixture` etc.
#[path = "fixtures/mod.rs"]
mod fixtures;

use fixtures::{heading_xml, para_xml, styled_run_xml, table_2x2_xml, HwpxFixture};
use hwp2md::{hwpx, ir, md};

// ---------------------------------------------------------------------------
// Helper: read an HWPX fixture from a temp file and return the IR Document.
// ---------------------------------------------------------------------------

fn read_fixture(fixture: HwpxFixture) -> (tempfile::TempDir, ir::Document) {
    let (dir, path) = fixture.write_to_tempfile();
    let doc = hwpx::read_hwpx(&path).expect("read_hwpx failed");
    (dir, doc)
}

// ---------------------------------------------------------------------------
// 1. Empty document — no blocks, no crash
// ---------------------------------------------------------------------------

#[test]
fn fixture_empty_document_parses_without_error() {
    let (_dir, doc) = read_fixture(HwpxFixture::new());

    // No sections or empty sections — either is acceptable for an empty fixture.
    let block_count: usize = doc.sections.iter().map(|s| s.blocks.len()).sum();
    assert_eq!(block_count, 0, "empty fixture should produce zero blocks");
}

#[test]
fn fixture_empty_document_to_markdown_is_empty_or_whitespace() {
    let (_dir, doc) = read_fixture(HwpxFixture::new());
    let md = md::write_markdown(&doc, false);
    // An empty document may produce an empty string or a bare newline.
    assert!(
        md.trim().is_empty(),
        "empty fixture markdown should be blank; got: {md:?}"
    );
}

// ---------------------------------------------------------------------------
// 2. Metadata — title and author are round-tripped
// ---------------------------------------------------------------------------

#[test]
fn fixture_metadata_title_and_author_preserved() {
    let (_dir, doc) = read_fixture(
        HwpxFixture::new()
            .title("Test Document")
            .author("Mario Cho"),
    );

    assert_eq!(
        doc.metadata.title.as_deref(),
        Some("Test Document"),
        "title not preserved"
    );
    assert_eq!(
        doc.metadata.author.as_deref(),
        Some("Mario Cho"),
        "author not preserved"
    );
}

#[test]
fn fixture_metadata_in_frontmatter() {
    let (_dir, doc) = read_fixture(
        HwpxFixture::new()
            .title("Frontmatter Doc")
            .author("Test Author"),
    );

    let md = md::write_markdown(&doc, true);
    assert!(
        md.contains("title:"),
        "frontmatter title key missing; md: {md:?}"
    );
    assert!(
        md.contains("Frontmatter Doc"),
        "title value missing; md: {md:?}"
    );
    assert!(
        md.contains("author:"),
        "frontmatter author key missing; md: {md:?}"
    );
}

// ---------------------------------------------------------------------------
// 3. Single paragraph
// ---------------------------------------------------------------------------

#[test]
fn fixture_single_paragraph_text_preserved() {
    let body = para_xml("Hello, world!");
    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&body));

    let text: String = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .filter_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
            } else {
                None
            }
        })
        .collect();

    assert!(
        text.contains("Hello, world!"),
        "paragraph text missing; got: {text:?}"
    );
}

#[test]
fn fixture_paragraph_to_markdown_contains_text() {
    let body = para_xml("Integration paragraph.");
    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&body));
    let md = md::write_markdown(&doc, false);
    assert!(
        md.contains("Integration paragraph."),
        "markdown missing paragraph text; got: {md:?}"
    );
}

// ---------------------------------------------------------------------------
// 4. Heading
// ---------------------------------------------------------------------------

#[test]
fn fixture_heading_level_1_parsed() {
    let body = heading_xml(1, "Top Level");
    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&body));

    let found = doc.sections.iter().flat_map(|s| &s.blocks).any(|b| {
        matches!(b, ir::Block::Heading { level: 1, inlines }
            if inlines.iter().any(|i| i.text.contains("Top Level")))
    });
    assert!(found, "H1 block with 'Top Level' not found");
}

#[test]
fn fixture_heading_level_2_parsed() {
    let body = heading_xml(2, "Sub Section");
    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&body));

    let found = doc.sections.iter().flat_map(|s| &s.blocks).any(|b| {
        matches!(b, ir::Block::Heading { level: 2, inlines }
            if inlines.iter().any(|i| i.text.contains("Sub Section")))
    });
    assert!(found, "H2 block with 'Sub Section' not found");
}

#[test]
fn fixture_heading_to_markdown_uses_atx_prefix() {
    let body = heading_xml(1, "ATX Heading");
    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&body));
    let md = md::write_markdown(&doc, false);

    assert!(
        md.contains("# ATX Heading"),
        "expected '# ATX Heading' in markdown; got: {md:?}"
    );
}

#[test]
fn fixture_multiple_headings_levels_preserved() {
    let body = format!(
        "{}{}{}",
        heading_xml(1, "Title"),
        heading_xml(2, "Chapter"),
        heading_xml(3, "Section"),
    );
    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&body));
    let md = md::write_markdown(&doc, false);

    assert!(md.contains("# Title"), "H1 missing; md: {md:?}");
    assert!(md.contains("## Chapter"), "H2 missing; md: {md:?}");
    assert!(md.contains("### Section"), "H3 missing; md: {md:?}");
}

// ---------------------------------------------------------------------------
// 5. Table
// ---------------------------------------------------------------------------

#[test]
fn fixture_table_rows_and_cells_parsed() {
    let body = table_2x2_xml("Col A", "Col B", "val1", "val2");
    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&body));

    let table = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find(|b| matches!(b, ir::Block::Table { .. }));

    assert!(table.is_some(), "no Table block found in parsed document");

    if let Some(ir::Block::Table { rows, col_count }) = table {
        assert_eq!(*col_count, 2, "expected 2 columns");
        assert_eq!(rows.len(), 2, "expected 2 rows");
    }
}

#[test]
fn fixture_table_cell_text_preserved() {
    let body = table_2x2_xml("Header1", "Header2", "Data1", "Data2");
    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&body));

    let all_text: String = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .filter_map(|b| {
            if let ir::Block::Table { rows, .. } = b {
                Some(
                    rows.iter()
                        .flat_map(|r| &r.cells)
                        .flat_map(|c| &c.blocks)
                        .filter_map(|b| {
                            if let ir::Block::Paragraph { inlines } = b {
                                Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
                            } else {
                                None
                            }
                        })
                        .collect::<String>(),
                )
            } else {
                None
            }
        })
        .collect();

    assert!(
        all_text.contains("Header1"),
        "Header1 missing; got: {all_text:?}"
    );
    assert!(
        all_text.contains("Header2"),
        "Header2 missing; got: {all_text:?}"
    );
    assert!(
        all_text.contains("Data1"),
        "Data1 missing; got: {all_text:?}"
    );
    assert!(
        all_text.contains("Data2"),
        "Data2 missing; got: {all_text:?}"
    );
}

#[test]
fn fixture_table_to_markdown_gfm_format() {
    let body = table_2x2_xml("Col A", "Col B", "row1a", "row1b");
    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&body));
    let md = md::write_markdown(&doc, false);

    // GFM table uses `|` separators.
    assert!(
        md.contains('|'),
        "expected GFM table separators '|'; got: {md:?}"
    );
    assert!(md.contains("Col A"), "Col A missing; got: {md:?}");
    assert!(md.contains("Col B"), "Col B missing; got: {md:?}");
    assert!(md.contains("row1a"), "row1a missing; got: {md:?}");
}

// ---------------------------------------------------------------------------
// 6. Inline formatting
// ---------------------------------------------------------------------------

#[test]
fn fixture_bold_italic_inline_flags_set() {
    let body = styled_run_xml("formatted text");
    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&body));

    let has_bold_italic = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .filter_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                Some(inlines)
            } else {
                None
            }
        })
        .flatten()
        .any(|i| i.bold && i.italic && i.text.contains("formatted text"));

    assert!(
        has_bold_italic,
        "expected bold+italic inline with 'formatted text'"
    );
}

#[test]
fn fixture_bold_italic_to_markdown_syntax() {
    let body = styled_run_xml("styled");
    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&body));
    let md = md::write_markdown(&doc, false);

    // Bold+italic renders as ***text*** or **_text_** etc. — any combination
    // that contains the word is sufficient; the exact delimiters differ by
    // implementation.
    assert!(md.contains("styled"), "text 'styled' missing; got: {md:?}");
    // At a minimum there must be *some* markdown emphasis delimiter.
    assert!(
        md.contains('*') || md.contains('_'),
        "expected emphasis delimiters; got: {md:?}"
    );
}

// ---------------------------------------------------------------------------
// 7. Mixed content document
// ---------------------------------------------------------------------------

#[test]
fn fixture_mixed_content_all_blocks_present() {
    let body = format!(
        "{}{}{}",
        heading_xml(1, "Document Title"),
        para_xml("An introductory paragraph."),
        table_2x2_xml("A", "B", "1", "2"),
    );
    let (_dir, doc) = read_fixture(HwpxFixture::new().title("Mixed Doc").section(&body));

    let blocks: Vec<&ir::Block> = doc.sections.iter().flat_map(|s| &s.blocks).collect();

    let has_heading = blocks
        .iter()
        .any(|b| matches!(b, ir::Block::Heading { level: 1, .. }));
    let has_para = blocks
        .iter()
        .any(|b| matches!(b, ir::Block::Paragraph { .. }));
    let has_table = blocks.iter().any(|b| matches!(b, ir::Block::Table { .. }));

    assert!(has_heading, "heading missing from mixed-content doc");
    assert!(has_para, "paragraph missing from mixed-content doc");
    assert!(has_table, "table missing from mixed-content doc");
}

#[test]
fn fixture_mixed_content_markdown_roundtrip_stable() {
    let body = format!(
        "{}{}",
        heading_xml(2, "Stable Heading"),
        para_xml("Stable paragraph text."),
    );
    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&body));

    // First pass: IR → MD
    let md1 = md::write_markdown(&doc, false);
    // Second pass: MD → IR → MD
    let doc2 = md::parse_markdown(&md1);
    let md2 = md::write_markdown(&doc2, false);

    assert!(
        md1.contains("## Stable Heading"),
        "heading missing from pass 1; md1: {md1:?}"
    );
    assert!(
        md2.contains("## Stable Heading"),
        "heading missing from pass 2; md2: {md2:?}"
    );
    assert!(
        md1.contains("Stable paragraph text."),
        "paragraph missing from pass 1; md1: {md1:?}"
    );
    assert!(
        md2.contains("Stable paragraph text."),
        "paragraph missing from pass 2; md2: {md2:?}"
    );
}

// ---------------------------------------------------------------------------
// 8. convert::to_markdown — file-based API
// ---------------------------------------------------------------------------

#[test]
fn fixture_convert_to_markdown_heading_via_api() {
    let body = heading_xml(1, "API Heading");
    let (dir, path) = HwpxFixture::new().section(&body).write_to_tempfile();

    let out_path = dir.path().join("output.md");
    hwp2md::convert::to_markdown(&path, Some(&out_path), None, false)
        .expect("convert::to_markdown failed");

    assert!(out_path.exists(), "output markdown file not created");
    let content = std::fs::read_to_string(&out_path).expect("read output");
    assert!(
        content.contains("# API Heading"),
        "heading missing from converted markdown; got: {content:?}"
    );
}

#[test]
fn fixture_convert_to_markdown_table_via_api() {
    let body = table_2x2_xml("X", "Y", "1", "2");
    let (dir, path) = HwpxFixture::new().section(&body).write_to_tempfile();

    let out_path = dir.path().join("output.md");
    hwp2md::convert::to_markdown(&path, Some(&out_path), None, false)
        .expect("convert::to_markdown failed");

    let content = std::fs::read_to_string(&out_path).expect("read output");
    assert!(
        content.contains('|'),
        "table separator missing; got: {content:?}"
    );
}

// ---------------------------------------------------------------------------
// 9. ZIP structure sanity — fixture is a valid ZIP file
// ---------------------------------------------------------------------------

#[test]
fn fixture_bytes_are_valid_zip() {
    let bytes = HwpxFixture::new().title("Zip Test").build();
    // ZIP files start with the PK magic bytes (0x50 0x4B).
    assert!(
        bytes.starts_with(b"PK"),
        "fixture bytes do not start with PK magic"
    );
    // The mimetype entry must appear near the start (it is STORED, so uncompressed).
    let content = String::from_utf8_lossy(&bytes);
    assert!(
        content.contains("application/hwpx+zip"),
        "mimetype entry not found in fixture bytes"
    );
}

#[test]
fn fixture_can_be_opened_as_zip_archive() {
    let bytes = HwpxFixture::new().title("Archive Test").build();
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).expect("failed to open as ZIP archive");

    let names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index_raw(i).ok().map(|f| f.name().to_owned()))
        .collect();

    assert!(
        names.contains(&"mimetype".to_owned()),
        "mimetype entry missing"
    );
    assert!(
        names.contains(&"META-INF/container.xml".to_owned()),
        "container.xml missing"
    );
    assert!(
        names.contains(&"Contents/content.hpf".to_owned()),
        "content.hpf missing"
    );
    assert!(
        names.contains(&"Contents/section0.xml".to_owned()),
        "section0.xml missing"
    );
}

// ---------------------------------------------------------------------------
// 10. Edge cases
// ---------------------------------------------------------------------------

#[test]
fn fixture_special_chars_in_paragraph_preserved() {
    // Characters that must survive XML escaping in the fixture builder.
    let text = "a & b < c > d";
    let body = para_xml(text);
    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&body));

    let found_text: String = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .filter_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
            } else {
                None
            }
        })
        .collect();

    assert!(
        found_text.contains("a & b < c > d"),
        "special chars lost; got: {found_text:?}"
    );
}

#[test]
fn fixture_unicode_korean_text_preserved() {
    let korean = "안녕하세요 세계";
    let body = para_xml(korean);
    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&body));

    let found_text: String = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .filter_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
            } else {
                None
            }
        })
        .collect();

    assert!(
        found_text.contains(korean),
        "Korean text lost; got: {found_text:?}"
    );
}

#[test]
fn fixture_empty_title_and_author_do_not_panic() {
    let (_dir, doc) = read_fixture(HwpxFixture::new().title("").author(""));
    // Metadata with empty strings should be normalised to None or empty.
    // The key invariant is: no panic.
    let _ = md::write_markdown(&doc, true);
}
