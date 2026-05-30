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

use fixtures::{
    heading_xml, lang_hint_comment, para_xml, styled_run_xml, table_2x2_xml, HwpxFixture,
};
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

    if let Some(ir::Block::Table {
        rows, col_count, ..
    }) = table
    {
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
        content.contains("application/hwp+zip"),
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

// ---------------------------------------------------------------------------
// 11. Tier-4 Korean regulation heading detection via HWPX pipeline
// ---------------------------------------------------------------------------

/// 편(Part) paragraphs with no `styleIDRef` fall through to tier-4 text-pattern
/// detection and must be promoted to H1 through the full HWPX pipeline.
///
/// Tier-4 fires when tier-1 (styleIDRef), tier-2 (bold+large font), and
/// tier-3 (font-size) all produce nothing.  A plain `<hp:p>` with no style
/// attribute satisfies this precondition.
#[test]
fn pyeon_detected_as_h1_via_tier4() {
    let hwpx = HwpxFixture::new()
        .section(&para_xml("제1편 총칙"))
        .section(&para_xml("제1장 일반규정"))
        .section(&para_xml("일반 본문입니다"))
        .build();

    let (_dir, path) = {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("pyeon.hwpx");
        std::fs::write(&path, &hwpx).expect("write hwpx fixture");
        (dir, path)
    };
    let doc = hwpx::read_hwpx(&path).expect("read_hwpx failed");

    let blocks: Vec<&ir::Block> = doc.sections.iter().flat_map(|s| &s.blocks).collect();

    assert_eq!(
        blocks.len(),
        3,
        "expected exactly 3 blocks; got {}",
        blocks.len()
    );

    // 제1편 총칙 → H1 (tier-4: 편 marker)
    assert!(
        matches!(blocks[0], ir::Block::Heading { level: 1, .. }),
        "제1편 총칙 should be H1 via tier-4; got {:?}",
        blocks[0]
    );

    // 제1장 일반규정 → H1 (tier-4: 장 marker)
    assert!(
        matches!(blocks[1], ir::Block::Heading { level: 1, .. }),
        "제1장 일반규정 should be H1 via tier-4; got {:?}",
        blocks[1]
    );

    // 일반 본문입니다 → Paragraph (no regulation marker)
    assert!(
        matches!(blocks[2], ir::Block::Paragraph { .. }),
        "body text should be Paragraph; got {:?}",
        blocks[2]
    );
}

/// 편 heading text is preserved in the IR inlines.
#[test]
fn pyeon_heading_text_preserved() {
    let hwpx = HwpxFixture::new().section(&para_xml("제3편 채권")).build();

    let (_dir, path) = {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("pyeon_text.hwpx");
        std::fs::write(&path, &hwpx).expect("write hwpx fixture");
        (dir, path)
    };
    let doc = hwpx::read_hwpx(&path).expect("read_hwpx failed");

    let heading = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find(|b| matches!(b, ir::Block::Heading { level: 1, .. }));

    assert!(
        heading.is_some(),
        "expected an H1 heading block; none found"
    );
    let ir::Block::Heading { inlines, .. } = heading.unwrap() else {
        unreachable!()
    };
    let text: String = inlines.iter().map(|i| i.text.as_str()).collect();
    assert_eq!(text, "제3편 채권", "heading text mismatch: got {:?}", text);
}

/// 편 → H1 renders as `# 제N편 …` in Markdown output.
#[test]
fn pyeon_rendered_as_atx_h1_in_markdown() {
    let hwpx = HwpxFixture::new().section(&para_xml("제2편 물권")).build();

    let (_dir, path) = {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("pyeon_md.hwpx");
        std::fs::write(&path, &hwpx).expect("write hwpx fixture");
        (dir, path)
    };
    let doc = hwpx::read_hwpx(&path).expect("read_hwpx failed");
    let md = md::write_markdown(&doc, false);

    assert!(
        md.contains("# 제2편 물권"),
        "expected '# 제2편 물권' in markdown; got: {md:?}"
    );
}

// ---------------------------------------------------------------------------
// 12. DRM-protected HWP file detection
// ---------------------------------------------------------------------------

fn create_drm_protected_hwp_fixture() -> (tempfile::TempDir, std::path::PathBuf) {
    use std::io::Write;

    let dir = tempfile::tempdir().expect("create tempdir");
    let path = dir.path().join("drm_protected.hwp");

    let file = std::fs::File::create(&path).expect("create file");
    let mut cfb = cfb::CompoundFile::create(file).expect("create CFB");

    let mut stream = cfb
        .create_stream("/FileHeader")
        .expect("create FileHeader stream");

    let mut header_data = [0u8; 256];
    let sig = b"HWP Document File";
    header_data[..sig.len()].copy_from_slice(sig);
    header_data[32] = 0; // version extra
    header_data[33] = 0; // version micro
    header_data[34] = 1; // version minor
    header_data[35] = 5; // version major
    header_data[36..40].copy_from_slice(&0x10u32.to_le_bytes()); // has_drm

    stream.write_all(&header_data).expect("write FileHeader");
    drop(stream);
    cfb.flush().expect("flush CFB");
    drop(cfb);

    (dir, path)
}

#[test]
fn drm_protected_hwp_returns_error() {
    let (_dir, path) = create_drm_protected_hwp_fixture();

    let result = hwp2md::convert::to_markdown(&path, None, None, false);

    assert!(
        result.is_err(),
        "expected DrmProtected error for encrypted HWP; got Ok"
    );

    if let Err(hwp2md::Hwp2MdError::DrmProtected { path: err_path }) = result {
        assert_eq!(err_path, path);
    } else {
        panic!("expected DrmProtected error variant; got {result:?}");
    }
}

#[test]
fn drm_protected_hwp_error_message_contains_drm() {
    let (_dir, path) = create_drm_protected_hwp_fixture();

    let result = hwp2md::convert::to_markdown(&path, None, None, false);
    assert!(result.is_err(), "expected error for DRM-protected file");

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.to_lowercase().contains("drm"),
        "error message should mention DRM; got: {error_msg:?}"
    );
}

#[test]
fn read_hwpx_with_bindata_populates_assets() {
    let png_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]; // PNG magic
    let fixture = HwpxFixture::new()
        .section(&para_xml("Has image"))
        .bin_data("test_image.png", png_data.clone());
    let (_dir, path) = fixture.write_to_tempfile();

    let doc = hwp2md::hwpx::read_hwpx(&path).expect("read fixture");
    assert!(
        !doc.assets.is_empty(),
        "assets should contain the BinData entry"
    );
    assert_eq!(doc.assets[0].name, "test_image.png");
    assert_eq!(doc.assets[0].data, png_data);
}

#[test]
fn write_assets_extracts_bindata_to_disk() {
    let png_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let fixture = HwpxFixture::new()
        .section(&para_xml("Doc with image"))
        .bin_data("photo.png", png_data.clone());
    let (dir, path) = fixture.write_to_tempfile();

    let assets_dir = dir.path().join("extracted");
    hwp2md::convert::to_markdown(
        &path,
        Some(&dir.path().join("out.md")),
        Some(&assets_dir),
        false,
    )
    .expect("convert with assets");

    let extracted = assets_dir.join("photo.png");
    assert!(
        extracted.exists(),
        "image should be extracted to assets dir"
    );
    assert_eq!(
        std::fs::read(&extracted).unwrap(),
        png_data,
        "extracted data must match"
    );
}

// ---------------------------------------------------------------------------
// 14. Code block language hint — XML comment round-trip through HWPX fixture
// ---------------------------------------------------------------------------

/// A `<!-- hwp2md:lang:python -->` XML comment immediately before an `<hp:p>`
/// element causes the HWPX reader to interpret that paragraph as a CodeBlock
/// with `language == Some("python")`.
#[test]
fn fixture_lang_hint_comment_produces_codeblock_ir() {
    // The section XML embeds the lang-hint comment directly before the paragraph
    // that carries the code text. This mirrors what the HWPX writer emits when
    // serialising a `Block::CodeBlock { language: Some("python") }`.
    let (_dir, doc) =
        read_fixture(HwpxFixture::new().with_lang_hint_paragraph("python", "print(\"hello\")"));

    let code_block = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find(|b| matches!(b, ir::Block::CodeBlock { .. }));

    assert!(
        code_block.is_some(),
        "expected a CodeBlock; blocks: {:?}",
        doc.sections
            .iter()
            .flat_map(|s| &s.blocks)
            .collect::<Vec<_>>()
    );

    let Some(ir::Block::CodeBlock { language, code }) = code_block else {
        panic!(
            "expected Block::CodeBlock, got: {:?}",
            doc.sections
                .iter()
                .flat_map(|s| &s.blocks)
                .collect::<Vec<_>>()
        );
    };
    assert_eq!(
        language.as_deref(),
        Some("python"),
        "language must be 'python'; got: {language:?}"
    );
    assert!(
        code.contains("print(\"hello\")"),
        "code content must contain print call; got: {code:?}"
    );
}

/// The same fixture as above, converted to Markdown, must produce a fenced
/// code block opened with ` ```python `.
#[test]
fn fixture_lang_hint_comment_renders_python_fence_in_markdown() {
    let (_dir, doc) =
        read_fixture(HwpxFixture::new().with_lang_hint_paragraph("python", "print(\"hello\")"));
    let markdown = md::write_markdown(&doc, false);

    assert!(
        markdown.contains("```python"),
        "markdown must contain ```python fence; got: {markdown:?}"
    );
    assert!(
        markdown.contains("print(\"hello\")"),
        "code content must appear in markdown; got: {markdown:?}"
    );
}

// ---------------------------------------------------------------------------
// 15. Page break — `<hp:ctrl id="newPage"/>` round-trip through HWPX fixture
// ---------------------------------------------------------------------------

/// A fixture whose section XML contains `<hp:ctrl id="newPage"/>` must parse
/// into an IR that includes a `Block::PageBreak` located between the two
/// surrounding paragraph blocks.
#[test]
fn fixture_newpage_ctrl_produces_pagebreak_ir() {
    let body = format!(
        "{}<hp:p><hp:run><hp:ctrl id=\"newPage\"/></hp:run></hp:p>{}",
        para_xml("before"),
        para_xml("after"),
    );
    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&body));

    let blocks: Vec<&ir::Block> = doc.sections.iter().flat_map(|s| &s.blocks).collect();

    let has_pagebreak = blocks.iter().any(|b| matches!(b, ir::Block::PageBreak));
    assert!(
        has_pagebreak,
        "expected a PageBreak block; blocks: {blocks:?}"
    );

    // PageBreak must sit between the two paragraphs.
    let kinds: Vec<&str> = blocks
        .iter()
        .map(|b| match b {
            ir::Block::Paragraph { .. } => "para",
            ir::Block::PageBreak => "pb",
            _ => "other",
        })
        .collect();
    let pb_idx = kinds.iter().position(|k| *k == "pb").unwrap();
    assert!(
        kinds[..pb_idx].contains(&"para") && kinds[pb_idx + 1..].contains(&"para"),
        "PageBreak must sit between the two paragraphs; kinds: {kinds:?}"
    );
}

/// A `<!-- hwp2md:lang:python -->` comment inside a table cell must NOT leak to
/// the next top-level paragraph after the table, AND the cell itself must
/// produce a `CodeBlock` (Sprint 75: nested scopes honour code-fence hints).
#[test]
fn fixture_lang_hint_inside_cell_does_not_leak_to_next_toplevel_paragraph() {
    // 1×1 table whose single cell contains the lang-hint comment before its
    // inner paragraph.  A top-level paragraph follows the table.
    let lh = lang_hint_comment("python");
    let body = format!(
        r#"<hp:p id="10" paraPrIDRef="0"><hp:run charPrIDRef="0">
  <hp:tbl rowCnt="1" colCnt="1">
    <hp:tr>
      <hp:tc>
        {lh}
        <hp:p id="11" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>cell text</hp:t></hp:run></hp:p>
      </hp:tc>
    </hp:tr>
  </hp:tbl>
</hp:run></hp:p>
<hp:p id="12" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>after table</hp:t></hp:run></hp:p>"#
    );

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&body));

    // Sprint 75: the cell itself must contain a CodeBlock (not a Paragraph).
    let cell_code_block = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Table { rows, .. } = b {
                rows.iter()
                    .flat_map(|r| &r.cells)
                    .flat_map(|c| &c.blocks)
                    .find(|cb| matches!(cb, ir::Block::CodeBlock { code, .. } if code.contains("cell text")))
                    .map(|cb| format!("{cb:?}"))
            } else {
                None
            }
        });
    assert!(
        cell_code_block.is_some(),
        "cell must produce a CodeBlock with 'cell text' (Sprint 75); blocks: {:?}",
        doc.sections
            .iter()
            .flat_map(|s| &s.blocks)
            .collect::<Vec<_>>()
    );

    // The top-level paragraph that follows the table must be a plain Paragraph,
    // not a CodeBlock (which it would be if pending_code_lang leaked out of the cell).
    let after_block = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find(|b| match b {
            ir::Block::Paragraph { inlines } => {
                inlines.iter().any(|i| i.text.contains("after table"))
            }
            ir::Block::CodeBlock { code, .. } => code.contains("after table"),
            _ => false,
        });

    assert!(
        after_block.is_some(),
        "'after table' block not found; blocks: {:?}",
        doc.sections
            .iter()
            .flat_map(|s| &s.blocks)
            .collect::<Vec<_>>()
    );
    assert!(
        matches!(after_block.unwrap(), ir::Block::Paragraph { .. }),
        "top-level paragraph after table must be Paragraph (not CodeBlock); \
         pending_code_lang from inside the cell must not leak; got: {:?}",
        after_block.unwrap()
    );
}

/// The same fixture, converted to Markdown, must contain the `<!-- pagebreak -->`
/// HTML comment that the Markdown writer emits for `Block::PageBreak`.
#[test]
fn fixture_newpage_ctrl_renders_pagebreak_marker_in_markdown() {
    let body = format!(
        "{}<hp:p><hp:run><hp:ctrl id=\"newPage\"/></hp:run></hp:p>{}",
        para_xml("before"),
        para_xml("after"),
    );
    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&body));
    let markdown = md::write_markdown(&doc, false);

    assert!(
        markdown.contains("<!-- pagebreak -->"),
        "markdown must contain <!-- pagebreak --> marker; got: {markdown:?}"
    );
}

// ---------------------------------------------------------------------------
// 16. Sprint 75 — nested scope CodeBlock support
// ---------------------------------------------------------------------------

/// A `<!-- hwp2md:lang:python -->` comment before a paragraph inside a table
/// cell must cause that paragraph to become a `CodeBlock` in the cell's block
/// list (not a plain `Paragraph`).
#[test]
fn fixture_lang_hint_in_cell_produces_codeblock_ir() {
    let lh = lang_hint_comment("python");
    let body = format!(
        r#"<hp:p id="20" paraPrIDRef="0"><hp:run charPrIDRef="0">
  <hp:tbl rowCnt="1" colCnt="1">
    <hp:tr>
      <hp:tc>
        {lh}
        <hp:p id="21" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>x = 1</hp:t></hp:run></hp:p>
      </hp:tc>
    </hp:tr>
  </hp:tbl>
</hp:run></hp:p>"#
    );

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&body));

    let cell_block = doc.sections.iter().flat_map(|s| &s.blocks).find_map(|b| {
        if let ir::Block::Table { rows, .. } = b {
            rows.iter()
                .flat_map(|r| &r.cells)
                .flat_map(|c| &c.blocks)
                .find(
                    |cb| matches!(cb, ir::Block::CodeBlock { code, .. } if code.contains("x = 1")),
                )
                .cloned()
        } else {
            None
        }
    });

    assert!(
        cell_block.is_some(),
        "expected a CodeBlock with 'x = 1' inside the table cell; blocks: {:?}",
        doc.sections
            .iter()
            .flat_map(|s| &s.blocks)
            .collect::<Vec<_>>()
    );

    if let Some(ir::Block::CodeBlock { language, code }) = cell_block {
        assert_eq!(
            language.as_deref(),
            Some("python"),
            "cell CodeBlock language must be 'python'; got: {language:?}"
        );
        assert!(
            code.contains("x = 1"),
            "cell CodeBlock code must contain 'x = 1'; got: {code:?}"
        );
    }
}

/// The same cell-with-lang-hint fixture, converted to Markdown, must render
/// the cell's code as a fenced code block inside the GFM table cell.
///
/// NOTE: GFM table cells cannot truly host fenced code blocks — a fenced block
/// is a block-level construct and a GFM table row is a single line.  The
/// renderer serialises the `CodeBlock` IR node via `write_block`, which emits
/// the standard `` ``` `` fence, trims it, and embeds the result inline in the
/// `| … |` cell column.  The markdown is therefore not valid GFM for
/// round-tripping purposes, but it does preserve the code content and the
/// fence markers in the output, which is the best achievable for this case.
///
/// Assertions distinguish a `CodeBlock` output from a plain `Paragraph`:
/// * The code text itself must be present.
/// * The language-tagged backtick fence (`` ```rust ``) must be present,
///   proving the block was serialised as `CodeBlock`, not `Paragraph`.
#[test]
fn fixture_lang_hint_in_cell_renders_as_codeblock_in_table_markdown() {
    let lh = lang_hint_comment("rust");
    let body = format!(
        r#"<hp:p id="30" paraPrIDRef="0"><hp:run charPrIDRef="0">
  <hp:tbl rowCnt="1" colCnt="1">
    <hp:tr>
      <hp:tc>
        {lh}
        <hp:p id="31" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>fn main() {{}}</hp:t></hp:run></hp:p>
      </hp:tc>
    </hp:tr>
  </hp:tbl>
</hp:run></hp:p>"#
    );

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&body));
    let markdown = md::write_markdown(&doc, false);

    // The code content must appear in the markdown output.
    assert!(
        markdown.contains("fn main() {}"),
        "code content must appear in markdown; got: {markdown:?}"
    );

    // The language-tagged code fence must be present.  A plain Paragraph would
    // emit the text without any backtick markers; the presence of "```rust"
    // confirms the block was serialised as a CodeBlock, not a Paragraph.
    assert!(
        markdown.contains("```rust"),
        "markdown must contain the ```rust language fence to confirm CodeBlock serialisation \
         (not plain Paragraph); got: {markdown:?}"
    );

    // A closing fence must also be present, confirming the fence is well-formed.
    let fence_open = markdown.find("```rust").unwrap();
    let after_open = &markdown[fence_open + 7..]; // skip "```rust"
    assert!(
        after_open.contains("```"),
        "a closing ``` fence must follow the opening ```rust; got: {markdown:?}"
    );
}

/// A `<!-- hwp2md:lang:python -->` comment before a paragraph inside a list
/// item must cause that paragraph to become a `CodeBlock` in the item's block
/// list (not a plain `Paragraph`).
///
/// List items in HWPX are represented with `<li>` / `<hp:li>` elements.
#[test]
fn fixture_lang_hint_in_list_item_produces_codeblock_ir() {
    // Build a section with an explicit list item containing a lang-hint comment.
    let lh = lang_hint_comment("rust");
    let body = format!(
        r#"<ul>
  <li>
    {lh}
    <hp:p id="40" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>let x = 42;</hp:t></hp:run></hp:p>
  </li>
</ul>"#
    );

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&body));

    // Walk all list items looking for a CodeBlock with the expected code.
    let found_code_block = doc.sections.iter().flat_map(|s| &s.blocks).any(|b| {
        if let ir::Block::List { items, .. } = b {
            items.iter().flat_map(|item| &item.blocks).any(|ib| {
                    matches!(ib, ir::Block::CodeBlock { code, .. } if code.contains("let x = 42;"))
                })
        } else {
            false
        }
    });

    assert!(
        found_code_block,
        "expected a CodeBlock with 'let x = 42;' inside the list item; blocks: {:?}",
        doc.sections
            .iter()
            .flat_map(|s| &s.blocks)
            .collect::<Vec<_>>()
    );
}

/// A `<!-- hwp2md:lang:python -->` comment before a paragraph inside a
/// footnote must cause that paragraph to become a `CodeBlock` in the
/// footnote's content block list.
///
/// Footnotes in HWPX are represented with `<fn>` / `<hp:fn>` elements
/// containing nested `<hp:p>` paragraphs.
#[test]
fn fixture_lang_hint_in_footnote_produces_codeblock_ir() {
    let lh = lang_hint_comment("python");
    let body = format!(
        r#"<hp:p id="50" paraPrIDRef="0">
  <hp:run charPrIDRef="0">
    <hp:t>main text</hp:t>
    <hp:fn id="fn1">
      {lh}
      <hp:p id="51" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>note code</hp:t></hp:run></hp:p>
    </hp:fn>
  </hp:run>
</hp:p>"#
    );

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&body));

    // Walk all Footnote blocks looking for a CodeBlock in the content.
    let found_code_block = doc.sections.iter().flat_map(|s| &s.blocks).any(|b| {
        if let ir::Block::Footnote { content, .. } = b {
            content.iter().any(
                |fb| matches!(fb, ir::Block::CodeBlock { code, .. } if code.contains("note code")),
            )
        } else {
            false
        }
    });

    assert!(
        found_code_block,
        "expected a CodeBlock with 'note code' inside the footnote content; blocks: {:?}",
        doc.sections
            .iter()
            .flat_map(|s| &s.blocks)
            .collect::<Vec<_>>()
    );
}

/// Regression test combining Sprint 74 (no-leak) and Sprint 75 (cell CodeBlock):
/// the cell must produce a `CodeBlock`, and the top-level paragraph after the
/// table must remain a plain `Paragraph`.
#[test]
fn fixture_lang_hint_inside_cell_does_not_leak_to_next_toplevel_paragraph_cell_is_codeblock() {
    let lh = lang_hint_comment("python");
    let body = format!(
        r#"<hp:p id="60" paraPrIDRef="0"><hp:run charPrIDRef="0">
  <hp:tbl rowCnt="1" colCnt="1">
    <hp:tr>
      <hp:tc>
        {lh}
        <hp:p id="61" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>import os</hp:t></hp:run></hp:p>
      </hp:tc>
    </hp:tr>
  </hp:tbl>
</hp:run></hp:p>
<hp:p id="62" paraPrIDRef="0"><hp:run charPrIDRef="0"><hp:t>plain paragraph</hp:t></hp:run></hp:p>"#
    );

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&body));
    let all_blocks: Vec<&ir::Block> = doc.sections.iter().flat_map(|s| &s.blocks).collect();

    // The cell must be a CodeBlock.
    let cell_is_code = all_blocks.iter().any(|b| {
        if let ir::Block::Table { rows, .. } = b {
            rows.iter()
                .flat_map(|r| &r.cells)
                .flat_map(|c| &c.blocks)
                .any(|cb| {
                    matches!(cb, ir::Block::CodeBlock { code, .. } if code.contains("import os"))
                })
        } else {
            false
        }
    });
    assert!(
        cell_is_code,
        "cell paragraph with lang hint must produce CodeBlock (Sprint 75); blocks: {all_blocks:?}"
    );

    // The top-level paragraph must remain a Paragraph.
    let top_level_after = all_blocks.iter().find(|b| match b {
        ir::Block::Paragraph { inlines } => {
            inlines.iter().any(|i| i.text.contains("plain paragraph"))
        }
        ir::Block::CodeBlock { code, .. } => code.contains("plain paragraph"),
        _ => false,
    });
    assert!(
        top_level_after.is_some(),
        "'plain paragraph' block not found; blocks: {all_blocks:?}"
    );
    assert!(
        matches!(top_level_after.unwrap(), ir::Block::Paragraph { .. }),
        "top-level paragraph after table must not be promoted to CodeBlock by cell lang hint; \
         got: {:?}",
        top_level_after.unwrap()
    );
}

// ---------------------------------------------------------------------------
// 17. Sprint 75 follow-up — lang-hint CodeBlock in header and footer scopes
// ---------------------------------------------------------------------------

/// A `<!-- hwp2md:lang:python -->` comment inside an HWPX header section must
/// cause the following `<hp:p>` to be parsed as `ir::Block::CodeBlock` in the
/// section's header block list.
///
/// The `flush_nested_scope` header branch routes through `flush_header_paragraph`
/// when `header_footer.active && header_footer.in_header`.  Without the `active`
/// guard (Fix M2), a stale `in_header` flag could cause an incorrect flush path.
#[test]
fn fixture_lang_hint_in_header_produces_codeblock_ir() {
    // The header XML contains a lang-hint comment followed by an <hp:p>.
    let lh = lang_hint_comment("python");
    let header_body = format!("{lh}\n<hp:p id=\"70\" paraPrIDRef=\"0\"><hp:run charPrIDRef=\"0\"><hp:t>header_code()</hp:t></hp:run></hp:p>");

    let (_dir, doc) = read_fixture(
        HwpxFixture::new()
            .section(&para_xml("body paragraph"))
            .with_header_xml(&header_body),
    );

    // The section header must be present and contain a CodeBlock.
    let header_blocks = doc
        .sections
        .iter()
        .find_map(|s| s.header.as_ref())
        .expect("section.header must be Some when header XML is provided");

    let code_block = header_blocks
        .iter()
        .find(|b| matches!(b, ir::Block::CodeBlock { .. }));

    assert!(
        code_block.is_some(),
        "header must contain a CodeBlock when lang-hint comment precedes the paragraph; \
         header_blocks: {header_blocks:?}"
    );

    let Some(ir::Block::CodeBlock { language, code }) = code_block else {
        unreachable!()
    };
    assert_eq!(
        language.as_deref(),
        Some("python"),
        "header CodeBlock language must be 'python'; got: {language:?}"
    );
    assert!(
        code.contains("header_code()"),
        "header CodeBlock code must contain 'header_code()'; got: {code:?}"
    );
}

/// A `<!-- hwp2md:lang:python -->` comment inside an HWPX footer section must
/// cause the following `<hp:p>` to be parsed as `ir::Block::CodeBlock` in the
/// section's footer block list.
///
/// Mirrors `fixture_lang_hint_in_header_produces_codeblock_ir` for the footer
/// path (`flush_footer_paragraph`).
#[test]
fn fixture_lang_hint_in_footer_produces_codeblock_ir() {
    // The footer XML contains a lang-hint comment followed by an <hp:p>.
    let lh = lang_hint_comment("python");
    let footer_body = format!("{lh}\n<hp:p id=\"80\" paraPrIDRef=\"0\"><hp:run charPrIDRef=\"0\"><hp:t>footer_code()</hp:t></hp:run></hp:p>");

    let (_dir, doc) = read_fixture(
        HwpxFixture::new()
            .section(&para_xml("body paragraph"))
            .with_footer_xml(&footer_body),
    );

    // The section footer must be present and contain a CodeBlock.
    let footer_blocks = doc
        .sections
        .iter()
        .find_map(|s| s.footer.as_ref())
        .expect("section.footer must be Some when footer XML is provided");

    let code_block = footer_blocks
        .iter()
        .find(|b| matches!(b, ir::Block::CodeBlock { .. }));

    assert!(
        code_block.is_some(),
        "footer must contain a CodeBlock when lang-hint comment precedes the paragraph; \
         footer_blocks: {footer_blocks:?}"
    );

    let Some(ir::Block::CodeBlock { language, code }) = code_block else {
        unreachable!()
    };
    assert_eq!(
        language.as_deref(),
        Some("python"),
        "footer CodeBlock language must be 'python'; got: {language:?}"
    );
    assert!(
        code.contains("footer_code()"),
        "footer CodeBlock code must contain 'footer_code()'; got: {code:?}"
    );
}

// ---------------------------------------------------------------------------
// Ruby annotation (Sprint 81 P4) — full-pipeline integration test
// ---------------------------------------------------------------------------

/// A HWPX `<hp:ruby>` element must survive the full pipeline:
/// HWPX XML → IR Inline.ruby → Markdown `<ruby>…<rt>…</rt></ruby>`.
#[test]
fn ruby_annotation_survives_full_pipeline() {
    let ruby_xml = r#"<hp:p><hp:run>
        <hp:ruby>
            <hp:rubyText>한자</hp:rubyText>
            <hp:baseText>漢字</hp:baseText>
        </hp:ruby>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(ruby_xml));

    // IR layer: must have a Paragraph with an inline carrying the ruby annotation.
    let ruby_inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.ruby.is_some())
            } else {
                None
            }
        });

    assert!(
        ruby_inline.is_some(),
        "expected at least one inline with ruby annotation; got blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    let ruby_inline = ruby_inline.unwrap();
    assert_eq!(ruby_inline.text, "漢字", "base text mismatch");
    assert_eq!(
        ruby_inline.ruby.as_deref(),
        Some("한자"),
        "annotation text mismatch"
    );

    // Markdown layer: must render as HTML ruby tags.
    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("<ruby>漢字<rt>한자</rt></ruby>"),
        "markdown must contain <ruby>漢字<rt>한자</rt></ruby>; got: {markdown:?}"
    );
}

/// Empty annotation (`<hp:rubyText>` is blank) must not produce a ruby field —
/// the base text renders as plain text without `<ruby>` tags.
#[test]
fn ruby_empty_annotation_renders_as_plain_text() {
    let ruby_xml = r#"<hp:p><hp:run>
        <hp:ruby>
            <hp:rubyText></hp:rubyText>
            <hp:baseText>漢字</hp:baseText>
        </hp:ruby>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(ruby_xml));

    let markdown = md::write_markdown(&doc, false);
    assert!(
        !markdown.contains("<ruby>"),
        "empty annotation must not produce <ruby> tags; got: {markdown:?}"
    );
    assert!(
        markdown.contains("漢字"),
        "base text must appear in output; got: {markdown:?}"
    );
}

/// Ruby annotation containing HTML special chars must be escaped in Markdown output.
/// Pins the `escape_html(annotation)` path in md/writer.rs end-to-end.
#[test]
fn ruby_annotation_html_chars_escaped_in_output() {
    // Annotation contains '<', '>', '&' — should be escaped to &lt;, &gt;, &amp;.
    let ruby_xml = r#"<hp:p><hp:run>
        <hp:ruby>
            <hp:rubyText>&lt;&amp;&gt;</hp:rubyText>
            <hp:baseText>漢字</hp:baseText>
        </hp:ruby>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(ruby_xml));
    let markdown = md::write_markdown(&doc, false);

    // The annotation "<&>" must be escaped; the raw chars must not appear in the rt tag.
    assert!(
        markdown.contains("<ruby>漢字<rt>"),
        "ruby base text must be present; got: {markdown:?}"
    );
    assert!(
        !markdown.contains("<rt><&>"),
        "annotation HTML chars must be escaped; got: {markdown:?}"
    );
}

// ---------------------------------------------------------------------------
// Ruby edge cases (Sprint 82 P2)
// ---------------------------------------------------------------------------

/// Empty base text + non-empty annotation must produce a ruby inline.
/// handlers.rs emits the inline whenever `!base.is_empty() || !annotation.is_empty()`.
/// MD writer renders `<ruby><rt>한자</rt></ruby>` (empty text between ruby tags).
#[test]
fn ruby_empty_base_with_annotation_produces_ruby_inline() {
    let ruby_xml = r#"<hp:p><hp:run>
        <hp:ruby>
            <hp:rubyText>한자</hp:rubyText>
            <hp:baseText></hp:baseText>
        </hp:ruby>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(ruby_xml));
    let markdown = md::write_markdown(&doc, false);

    // The annotation must appear in a <rt> tag even when the base is empty.
    assert!(
        markdown.contains("<rt>한자</rt>"),
        "annotation must appear in <rt> even with empty base; got: {markdown:?}"
    );
}

/// Two ruby elements in the same paragraph produce two ruby inlines in order.
#[test]
fn ruby_multiple_ruby_in_one_paragraph_both_present() {
    let ruby_xml = r#"<hp:p><hp:run>
        <hp:ruby>
            <hp:rubyText>いち</hp:rubyText>
            <hp:baseText>一</hp:baseText>
        </hp:ruby>
        <hp:ruby>
            <hp:rubyText>に</hp:rubyText>
            <hp:baseText>二</hp:baseText>
        </hp:ruby>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(ruby_xml));

    let ruby_inlines: Vec<_> = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .filter_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                Some(inlines.iter().filter(|i| i.ruby.is_some()).collect::<Vec<_>>())
            } else {
                None
            }
        })
        .flatten()
        .collect();

    assert_eq!(
        ruby_inlines.len(),
        2,
        "expected 2 ruby inlines, got {}; blocks: {:?}",
        ruby_inlines.len(),
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    assert_eq!(ruby_inlines[0].text, "一");
    assert_eq!(ruby_inlines[0].ruby.as_deref(), Some("いち"));
    assert_eq!(ruby_inlines[1].text, "二");
    assert_eq!(ruby_inlines[1].ruby.as_deref(), Some("に"));

    // Markdown: both ruby elements must appear.
    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("<ruby>一<rt>いち</rt></ruby>"),
        "first ruby missing; got: {markdown:?}"
    );
    assert!(
        markdown.contains("<ruby>二<rt>に</rt></ruby>"),
        "second ruby missing; got: {markdown:?}"
    );
}

/// Plain text adjacent to a ruby element in the same paragraph must both appear.
#[test]
fn ruby_adjacent_plain_text_and_ruby_both_present() {
    let ruby_xml = r#"<hp:p>
        <hp:run><hp:t>before </hp:t></hp:run>
        <hp:run>
            <hp:ruby>
                <hp:rubyText>ルビ</hp:rubyText>
                <hp:baseText>振り仮名</hp:baseText>
            </hp:ruby>
        </hp:run>
        <hp:run><hp:t> after</hp:t></hp:run>
    </hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(ruby_xml));
    let markdown = md::write_markdown(&doc, false);

    // All three parts must be present.
    assert!(
        markdown.contains("before"),
        "plain text before ruby must appear; got: {markdown:?}"
    );
    assert!(
        markdown.contains("<ruby>振り仮名<rt>ルビ</rt></ruby>"),
        "ruby element must be present; got: {markdown:?}"
    );
    assert!(
        markdown.contains("after"),
        "plain text after ruby must appear; got: {markdown:?}"
    );
    // Ordering: before < ruby < after (prevents swapped output).
    let pos_before = markdown.find("before").expect("'before' in markdown");
    let pos_ruby = markdown.find("<ruby>振り仮名").expect("ruby tag in markdown");
    let pos_after = markdown.find("after").expect("'after' in markdown");
    assert!(
        pos_before < pos_ruby && pos_ruby < pos_after,
        "order must be before · ruby · after; positions: before={pos_before} ruby={pos_ruby} after={pos_after}; markdown: {markdown:?}"
    );
}

// ---------------------------------------------------------------------------
// HWPX hyperlink field begin/end integration (Sprint 82 P3)
// ---------------------------------------------------------------------------

/// A `<hp:fieldBegin type="HYPERLINK" command="url"/>` … `<hp:fieldEnd/>` sequence
/// must produce an inline carrying the link URL, rendered as Markdown `[text](url)`.
#[test]
fn hwpx_hyperlink_field_begin_end_produces_link_inline() {
    let hyperlink_xml = r#"<hp:p><hp:run>
        <hp:fieldBegin type="HYPERLINK" command="https://example.com"/>
        <hp:t>Click here</hp:t>
        <hp:fieldEnd/>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(hyperlink_xml));

    let link_inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.link.is_some())
            } else {
                None
            }
        });

    assert!(
        link_inline.is_some(),
        "expected an inline with link; blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    let link_inline = link_inline.unwrap();
    assert_eq!(link_inline.text, "Click here", "link text mismatch");
    assert_eq!(
        link_inline.link.as_deref(),
        Some("https://example.com"),
        "link URL mismatch"
    );

    // Markdown rendering: [text](url) format.
    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("[Click here](https://example.com)"),
        "markdown must contain [text](url) link; got: {markdown:?}"
    );
}

/// Text after `<hp:fieldEnd/>` must NOT carry the hyperlink.
/// Pins that in_hyperlink is properly cleared on fieldEnd.
#[test]
fn hwpx_hyperlink_text_after_field_end_has_no_link() {
    let hyperlink_xml = r#"<hp:p><hp:run>
        <hp:fieldBegin type="HYPERLINK" command="https://example.com"/>
        <hp:t>linked</hp:t>
        <hp:fieldEnd/>
        <hp:t> plain</hp:t>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(hyperlink_xml));

    let inlines: Vec<_> = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .filter_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                Some(inlines.as_slice())
            } else {
                None
            }
        })
        .flatten()
        .collect();

    // Positive: "linked" must carry the URL (verifies fieldBegin actually worked).
    assert!(
        inlines
            .iter()
            .any(|i| i.text.contains("linked") && i.link.as_deref() == Some("https://example.com")),
        "linked text must carry the URL; inlines: {inlines:?}"
    );
    // Negative: "plain" (after fieldEnd) must have no link.
    assert!(
        inlines
            .iter()
            .any(|i| i.text.contains("plain") && i.link.is_none()),
        "text after fieldEnd must not carry the URL; inlines: {inlines:?}"
    );
}

// ---------------------------------------------------------------------------
// Footnote/endnote full-pipeline integration (Sprint 83 P2)
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
// PageBreak ctrl variants (Sprint 83 P3)
// ---------------------------------------------------------------------------

/// `<hp:ctrl id="pageBreak"/>` (camelCase variant) must also produce PageBreak.
/// Pins the `is_page_break_ctrl` match arm for "pageBreak".
#[test]
fn fixture_pagebreak_ctrl_variant_produces_pagebreak_ir() {
    let body = format!(
        "{}<hp:p><hp:run><hp:ctrl id=\"pageBreak\"/></hp:run></hp:p>{}",
        para_xml("before"),
        para_xml("after"),
    );
    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&body));
    let has_pagebreak = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .any(|b| matches!(b, ir::Block::PageBreak));
    assert!(
        has_pagebreak,
        "pageBreak ctrl variant must produce Block::PageBreak"
    );
}

/// `<hp:ctrl id="cnpb"/>` (column+page break) must also produce PageBreak.
/// Pins the `is_page_break_ctrl` match arm for "cnpb".
#[test]
fn fixture_cnpb_ctrl_variant_produces_pagebreak_ir() {
    let body = format!(
        "{}<hp:p><hp:run><hp:ctrl id=\"cnpb\"/></hp:run></hp:p>{}",
        para_xml("before"),
        para_xml("after"),
    );
    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&body));
    let has_pagebreak = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .any(|b| matches!(b, ir::Block::PageBreak));
    assert!(
        has_pagebreak,
        "cnpb ctrl variant must produce Block::PageBreak"
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

// ---------------------------------------------------------------------------
// Sprint 84 P4: HWPX image block (binaryItemIDRef) integration test
// ---------------------------------------------------------------------------

/// An `<hp:img binaryItemIDRef="photo.png"/>` element must produce an
/// `ir::Block::Image { src: "photo.png" }` and render as `![](photo.png)`
/// in Markdown.  Tests the full pipeline from HWPX XML → IR → Markdown.
#[test]
fn hwpx_img_element_produces_image_block_and_markdown() {
    // Minimal PNG magic bytes — must be in BinData/ for the fixture to be
    // a well-formed HWPX ZIP.
    let png_data = vec![0x89u8, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

    let img_xml = r#"<hp:p><hp:run>
        <hp:img binaryItemIDRef="photo.png"/>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(
        HwpxFixture::new()
            .section(img_xml)
            .bin_data("photo.png", png_data),
    );

    // IR layer: must contain an Image block.
    let image_block = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find(|b| matches!(b, ir::Block::Image { .. }));

    assert!(
        image_block.is_some(),
        "expected an Image block; blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    let ir::Block::Image { src, alt } = image_block.unwrap() else {
        unreachable!("already asserted Image variant")
    };
    assert_eq!(src, "photo.png", "Image src mismatch");
    // binaryItemIDRef has no explicit alt attr → default empty string.
    assert_eq!(alt, "", "Image alt must be empty when not set");

    // Markdown layer: renders as ![](photo.png).
    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("![](photo.png)"),
        "markdown must render as ![](photo.png); got: {markdown:?}"
    );
}

/// `<hp:img src="explicit.png"/>` (using the `src` attribute directly, rather
/// than `binaryItemIDRef`) must also produce an Image block.
/// Pins the alternate `src` attribute path in the HWPX reader.
#[test]
fn hwpx_img_element_src_attr_also_produces_image_block() {
    let img_xml = r#"<hp:p><hp:run>
        <hp:img src="inline.png" alt="A picture"/>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(img_xml));

    let image_block = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find(|b| matches!(b, ir::Block::Image { .. }));

    assert!(
        image_block.is_some(),
        "expected an Image block from src= attr; blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    let ir::Block::Image { src, alt } = image_block.unwrap() else {
        unreachable!("already asserted Image variant")
    };
    assert_eq!(src, "inline.png", "Image src mismatch");
    assert_eq!(alt, "A picture", "Image alt mismatch");
}

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
    // The fence line should be exactly "```" (or "```\n"), not "```python" etc.
    assert!(
        !markdown.contains("```no_lang") && !markdown.contains("```None"),
        "no-language fence must not have a language tag; got: {markdown:?}"
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
// Sprint 86 P3: HWPX equation element → Math block → LaTeX Markdown
// ---------------------------------------------------------------------------

/// `<hp:equation>` with EQEDIT content produces `ir::Block::Math { display: true }`
/// and renders as a display-math `$$..$$` fence in Markdown.
#[test]
fn hwpx_equation_element_produces_math_block_and_latex_markdown() {
    // NOTE: The HWPX <hp:equation> handler (handlers.rs:284-288) stores the raw
    // element text directly as `tex` — it does NOT call `eqedit_to_latex`.
    // That function is only wired into the HWP 5.0 binary reader path.
    // So `tex` is exactly the literal XML text content ("x + y" here).
    let eq_xml = r#"<hp:equation>x + y</hp:equation>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(eq_xml));

    // IR layer: must contain a Math block with display=true.
    let math_block = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find(|b| matches!(b, ir::Block::Math { .. }));

    assert!(
        math_block.is_some(),
        "expected Block::Math from <hp:equation>; blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    let ir::Block::Math { display, tex } = math_block.unwrap() else {
        unreachable!()
    };
    assert!(*display, "HWPX equation must produce display=true Math block");
    assert!(
        tex.contains("x") && tex.contains("y"),
        "LaTeX output must contain 'x' and 'y'; got: {tex:?}"
    );

    // Markdown layer: display math → $$\n..\n$$ format.
    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("$$"),
        "markdown must contain $$ fence for display math; got: {markdown:?}"
    );
    assert!(
        markdown.contains("x") && markdown.contains("y"),
        "math content must appear in markdown; got: {markdown:?}"
    );
}

// ---------------------------------------------------------------------------
// Sprint 86 P4: HorizontalRule and BlockQuote IR → Markdown rendering
// ---------------------------------------------------------------------------
// These blocks originate from the Markdown parser (---/> syntax), not from HWPX.
// Tested here via direct IR → Markdown to pin the writer rendering contract.

/// `ir::Block::HorizontalRule` must render as `---` (thematic break) in Markdown.
#[test]
fn ir_horizontal_rule_renders_as_thematic_break_in_markdown() {
    let doc = ir::Document {
        metadata: ir::Metadata::default(),
        sections: vec![ir::Section {
            blocks: vec![
                ir::Block::Paragraph {
                    inlines: vec![ir::Inline::plain("before".to_string())],
                },
                ir::Block::HorizontalRule,
                ir::Block::Paragraph {
                    inlines: vec![ir::Inline::plain("after".to_string())],
                },
            ],
            ..Default::default()
        }],
        assets: Vec::new(),
    };

    let markdown = md::write_markdown(&doc, false);

    assert!(
        markdown.contains("---"),
        "HorizontalRule must render as '---'; got: {markdown:?}"
    );

    // Ordering: before < HR < after.
    let pos_before = markdown.find("before").expect("'before' in markdown");
    let pos_hr = markdown.find("---").expect("'---' in markdown");
    let pos_after = markdown.find("after").expect("'after' in markdown");
    assert!(
        pos_before < pos_hr && pos_hr < pos_after,
        "order must be before · --- · after; positions: before={pos_before} hr={pos_hr} after={pos_after}"
    );
}

/// `ir::Block::BlockQuote` must render with `> ` prefix in Markdown.
#[test]
fn ir_block_quote_renders_as_quoted_text_in_markdown() {
    let doc = ir::Document {
        metadata: ir::Metadata::default(),
        sections: vec![ir::Section {
            blocks: vec![ir::Block::BlockQuote {
                blocks: vec![ir::Block::Paragraph {
                    inlines: vec![ir::Inline::plain("Quoted text.".to_string())],
                }],
            }],
            ..Default::default()
        }],
        assets: Vec::new(),
    };

    let markdown = md::write_markdown(&doc, false);

    // The writer (writer.rs:127-135) emits "> " + content on the same line.
    // Assert the exact line format produced rather than just presence.
    assert!(
        markdown.contains("> Quoted text."),
        "BlockQuote must render as '> Quoted text.' line; got: {markdown:?}"
    );
}

// ---------------------------------------------------------------------------
// Sprint 87 P2: Table roundtrip (IR → HWPX write → read_hwpx → IR)
// ---------------------------------------------------------------------------

/// An IR table survives a write→read round-trip through HWPX format:
/// the table structure (row/cell count, column count, cell text) must be
/// preserved after `write_hwpx` + `read_hwpx`.
#[test]
fn hwpx_table_roundtrip_preserves_structure() {
    let doc = ir::Document {
        metadata: ir::Metadata::default(),
        sections: vec![ir::Section {
            blocks: vec![ir::Block::Table {
                rows: vec![
                    ir::TableRow {
                        cells: vec![
                            ir::TableCell {
                                blocks: vec![ir::Block::Paragraph {
                                    inlines: vec![ir::Inline::plain("R0C0".to_string())],
                                }],
                                colspan: 1,
                                rowspan: 1,
                            },
                            ir::TableCell {
                                blocks: vec![ir::Block::Paragraph {
                                    inlines: vec![ir::Inline::plain("R0C1".to_string())],
                                }],
                                colspan: 1,
                                rowspan: 1,
                            },
                        ],
                        is_header: true,
                    },
                    ir::TableRow {
                        cells: vec![
                            ir::TableCell {
                                blocks: vec![ir::Block::Paragraph {
                                    inlines: vec![ir::Inline::plain("R1C0".to_string())],
                                }],
                                colspan: 1,
                                rowspan: 1,
                            },
                            ir::TableCell {
                                blocks: vec![ir::Block::Paragraph {
                                    inlines: vec![ir::Inline::plain("R1C1".to_string())],
                                }],
                                colspan: 1,
                                rowspan: 1,
                            },
                        ],
                        is_header: false,
                    },
                ],
                col_count: 2,
                inner_margin: None,
            }],
            ..Default::default()
        }],
        assets: Vec::new(),
    };

    let tmp = tempfile::NamedTempFile::new().expect("temp file");
    hwpx::write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");
    let read_back = hwpx::read_hwpx(tmp.path()).expect("read_hwpx");

    // Must contain a table block.
    let table_block = read_back
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find(|b| matches!(b, ir::Block::Table { .. }));

    assert!(
        table_block.is_some(),
        "table must survive HWPX roundtrip; blocks: {:?}",
        read_back.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    let ir::Block::Table { rows, col_count, .. } = table_block.unwrap() else {
        unreachable!()
    };
    assert_eq!(*col_count, 2, "col_count must be 2 after roundtrip");
    assert_eq!(rows.len(), 2, "row count must be 2 after roundtrip");

    // Per-cell position assertions: verify the text appears in the correct
    // structural position, not just anywhere in the document.
    fn cell_text(cell: &ir::TableCell) -> String {
        cell.blocks.iter().filter_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
            } else { None }
        }).collect()
    }
    assert_eq!(cell_text(&rows[0].cells[0]), "R0C0", "row0 col0 text mismatch");
    assert_eq!(cell_text(&rows[0].cells[1]), "R0C1", "row0 col1 text mismatch");
    assert_eq!(cell_text(&rows[1].cells[0]), "R1C0", "row1 col0 text mismatch");
    assert_eq!(cell_text(&rows[1].cells[1]), "R1C1", "row1 col1 text mismatch");

    // Markdown layer: all four cell values appear in output.
    let markdown = md::write_markdown(&read_back, false);
    assert!(
        markdown.contains("R0C0") && markdown.contains("R0C1")
            && markdown.contains("R1C0") && markdown.contains("R1C1"),
        "all cell texts must appear in markdown; got: {markdown:?}"
    );
}

// ---------------------------------------------------------------------------
// Sprint 87 P3: HWPX equation verbatim storage design documentation test
// ---------------------------------------------------------------------------

/// Documents that <hp:equation> content is stored verbatim (not passed through
/// eqedit_to_latex). This is a design contract test: if the behavior changes,
/// this test must be reviewed along with any EQEDIT-specific syntax in existing
/// HWPX files. See comment in handlers.rs for rationale.
#[test]
fn hwpx_equation_eqedit_syntax_stored_verbatim_not_converted() {
    // "a over b" is EQEDIT syntax for a fraction. eqedit_to_latex would convert
    // this to "\frac{a}{b}". In HWPX, it is stored verbatim.
    let eq_xml = r#"<hp:equation>a over b</hp:equation>"#;
    let (_dir, doc) = read_fixture(HwpxFixture::new().section(eq_xml));

    let math_block = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find(|b| matches!(b, ir::Block::Math { .. }));

    assert!(
        math_block.is_some(),
        "expected Math block; blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    let ir::Block::Math { tex, .. } = math_block.unwrap() else {
        unreachable!()
    };
    // HWPX verbatim: stored as "a over b", NOT converted to "\frac{a}{b}".
    assert_eq!(
        tex.as_str(),
        "a over b",
        "HWPX equation text must be stored verbatim; if this fails, \
         the equation handler now calls eqedit_to_latex — update this test \
         and the design comment in handlers.rs"
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

// ---------------------------------------------------------------------------
// Sprint 88 P2: HWPX inline formatting (bold/italic/underline) integration
// ---------------------------------------------------------------------------

/// Bold and italic charPr attributes produce the correct IR inline flags
/// and render as `***text***` in GFM Markdown.
#[test]
fn hwpx_charpr_bold_italic_produces_formatted_inline() {
    let body = styled_run_xml("Hello");
    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&body));

    // IR layer: inline must have bold=true, italic=true.
    let bold_italic_inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.bold && i.italic)
            } else {
                None
            }
        });

    assert!(
        bold_italic_inline.is_some(),
        "expected an inline with bold=true, italic=true; blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    assert_eq!(
        bold_italic_inline.unwrap().text,
        "Hello",
        "bold+italic inline text mismatch"
    );

    // Markdown layer: bold+italic → ***Hello***.
    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("***Hello***"),
        "bold+italic must render as ***Hello***; got: {markdown:?}"
    );
}

/// Underline charPr produces `<u>text</u>` in Markdown.
#[test]
fn hwpx_charpr_underline_produces_html_u_tag() {
    let underline_xml = r#"<hp:p paraPrIDRef="0"><hp:run charPrIDRef="0">
        <hp:charPr underline="single"/>
        <hp:t>Underlined</hp:t>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(underline_xml));

    let underline_inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.underline)
            } else {
                None
            }
        });

    assert!(
        underline_inline.is_some(),
        "expected an inline with underline=true; blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );

    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("<u>Underlined</u>"),
        "underline must render as <u>Underlined</u>; got: {markdown:?}"
    );
}

// ---------------------------------------------------------------------------
// Sprint 88 P3: HWPX superscript/subscript charPr integration test
// ---------------------------------------------------------------------------
//
// OWPML uses attribute name "supscript" (not "superscript") with values:
//   supscript="superscript" → superscript mode
//   supscript="subscript"   → subscript mode
// The handler is in context/flush.rs:124-126.

/// `supscript="superscript"` charPr → `ir::Inline.superscript=true`
/// → `<sup>text</sup>` in Markdown.
#[test]
fn hwpx_charpr_superscript_produces_sup_html() {
    let sup_xml = r#"<hp:p paraPrIDRef="0"><hp:run charPrIDRef="0">
        <hp:charPr supscript="superscript"/>
        <hp:t>2</hp:t>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(sup_xml));

    let sup_inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.superscript && !i.subscript)
            } else {
                None
            }
        });

    assert!(
        sup_inline.is_some(),
        "expected an inline with superscript=true; got: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("<sup>2</sup>"),
        "superscript must render as <sup>2</sup>; got: {markdown:?}"
    );
}

/// `supscript="subscript"` charPr → `ir::Inline.subscript=true`
/// → `<sub>text</sub>` in Markdown.
#[test]
fn hwpx_charpr_subscript_produces_sub_html() {
    let sub_xml = r#"<hp:p paraPrIDRef="0"><hp:run charPrIDRef="0">
        <hp:charPr supscript="subscript"/>
        <hp:t>i</hp:t>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(sub_xml));

    let sub_inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.subscript && !i.superscript)
            } else {
                None
            }
        });

    assert!(
        sub_inline.is_some(),
        "expected an inline with subscript=true; got: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("<sub>i</sub>"),
        "subscript must render as <sub>i</sub>; got: {markdown:?}"
    );
}

// ---------------------------------------------------------------------------
// Sprint 88 P4: multi-section HWPX document — boundary preservation
// ---------------------------------------------------------------------------

/// A two-section HWPX document must produce two sections in the IR.
/// Section boundaries must be preserved: text from section 0 must not
/// appear in section 1 and vice versa.
#[test]
fn hwpx_two_section_document_produces_two_ir_sections() {
    let (_dir, doc) = read_fixture(
        HwpxFixture::new()
            .section(&para_xml("Section one content."))
            .add_section(&para_xml("Section two content.")),
    );

    assert_eq!(
        doc.sections.len(),
        2,
        "two-section HWPX must produce 2 IR sections; got {}",
        doc.sections.len()
    );

    // Section 0 must contain "Section one content." and nothing from section 1.
    let sec0_text: String = doc.sections[0]
        .blocks
        .iter()
        .filter_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
            } else {
                None
            }
        })
        .collect();
    assert!(
        sec0_text.contains("Section one"),
        "section 0 must contain 'Section one'; got: {sec0_text:?}"
    );
    assert!(
        !sec0_text.contains("Section two"),
        "section 0 must not bleed into section 1 content; got: {sec0_text:?}"
    );

    // Section 1 must contain "Section two content.".
    let sec1_text: String = doc.sections[1]
        .blocks
        .iter()
        .filter_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
            } else {
                None
            }
        })
        .collect();
    assert!(
        sec1_text.contains("Section two"),
        "section 1 must contain 'Section two'; got: {sec1_text:?}"
    );
    // Symmetric: section 1 must not bleed section 0 content (forward direction).
    assert!(
        !sec1_text.contains("Section one"),
        "section 1 must not bleed section 0 content; got: {sec1_text:?}"
    );
}

// ---------------------------------------------------------------------------
// Sprint 89 P2: strikethrough + color charPr integration tests
// ---------------------------------------------------------------------------

/// `strikeout="single"` charPr → `ir::Inline.strikethrough=true` → `~~text~~`.
#[test]
fn hwpx_charpr_strikeout_produces_gfm_strikethrough() {
    let strike_xml = r#"<hp:p paraPrIDRef="0"><hp:run charPrIDRef="0">
        <hp:charPr strikeout="single"/>
        <hp:t>Deleted text</hp:t>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(strike_xml));

    let strike_inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.strikethrough)
            } else {
                None
            }
        });

    assert!(
        strike_inline.is_some(),
        "expected inline with strikethrough=true; blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    assert_eq!(strike_inline.unwrap().text, "Deleted text");

    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("~~Deleted text~~"),
        "strikeout must render as ~~Deleted text~~; got: {markdown:?}"
    );
}

/// Non-black `color` charPr → `<span style="color:#RRGGBB">text</span>` in Markdown.
/// Black (#000000) is treated as "no color" and does not emit a span.
#[test]
fn hwpx_charpr_non_black_color_produces_span_html() {
    let color_xml = r#"<hp:p paraPrIDRef="0"><hp:run charPrIDRef="0">
        <hp:charPr color="FF0000"/>
        <hp:t>Red text</hp:t>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(color_xml));

    let colored_inline = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                inlines.iter().find(|i| i.color.is_some())
            } else {
                None
            }
        });

    assert!(
        colored_inline.is_some(),
        "expected inline with color set; blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    // Color stored as #RRGGBB uppercase.
    assert_eq!(
        colored_inline.unwrap().color.as_deref(),
        Some("#FF0000"),
        "color must be stored as #FF0000"
    );

    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("<span style=\"color:#FF0000\">Red text</span>"),
        "non-black color must render as <span>; got: {markdown:?}"
    );
}

/// Black color (#000000) must NOT produce a span in Markdown output.
/// The reader treats black as "no color" (apply_charpr_attrs sets color to None).
#[test]
fn hwpx_charpr_black_color_produces_no_span() {
    let black_xml = r#"<hp:p paraPrIDRef="0"><hp:run charPrIDRef="0">
        <hp:charPr color="000000"/>
        <hp:t>Black text</hp:t>
    </hp:run></hp:p>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(black_xml));

    let markdown = md::write_markdown(&doc, false);
    assert!(
        !markdown.contains("<span"),
        "black color must not produce a <span>; got: {markdown:?}"
    );
    assert!(
        markdown.contains("Black text"),
        "text must still appear; got: {markdown:?}"
    );
}

// ---------------------------------------------------------------------------
// Sprint 89 P3: heading+paragraph ordering precision test
// ---------------------------------------------------------------------------

/// H1 → paragraph → H2 → paragraph must appear in that exact order in Markdown,
/// with correct ATX prefix levels (`#`, `##`) and correct position relative to text.
#[test]
fn hwpx_heading_paragraph_interleaved_preserves_order_and_levels() {
    let body = format!(
        "{}{}{}{}",
        heading_xml(1, "Main Title"),
        para_xml("Intro text."),
        heading_xml(2, "Sub Section"),
        para_xml("Body content."),
    );
    let (_dir, doc) = read_fixture(HwpxFixture::new().section(&body));
    let markdown = md::write_markdown(&doc, false);

    // All four pieces must be present.
    for expected in &["# Main Title", "## Sub Section", "Intro text.", "Body content."] {
        assert!(
            markdown.contains(expected),
            "'{expected}' must appear in markdown; got: {markdown:?}"
        );
    }

    // Ordering: H1 < intro < H2 < body.
    let pos_h1 = markdown.find("# Main Title").unwrap();
    let pos_intro = markdown.find("Intro text.").unwrap();
    let pos_h2 = markdown.find("## Sub Section").unwrap();
    let pos_body = markdown.find("Body content.").unwrap();

    assert!(
        pos_h1 < pos_intro && pos_intro < pos_h2 && pos_h2 < pos_body,
        "order must be: H1 < intro < H2 < body; positions: {pos_h1},{pos_intro},{pos_h2},{pos_body}"
    );

    // H2 prefix must not be promoted to H1 (must use "## " not "# ").
    let h2_line = markdown
        .lines()
        .find(|l| l.contains("Sub Section"))
        .unwrap_or("");
    assert!(
        h2_line.starts_with("## "),
        "H2 must start with '## ', not promoted to H1; got: {h2_line:?}"
    );

    // H1 prefix must not be demoted to H2 (must use "# " not "## ").
    let h1_line = markdown
        .lines()
        .find(|l| l.contains("Main Title"))
        .unwrap_or("");
    assert!(
        h1_line.starts_with("# ") && !h1_line.starts_with("## "),
        "H1 must start with '# ' (not '## '); got: {h1_line:?}"
    );
}

// ---------------------------------------------------------------------------
// Sprint 89 P4: MD → IR → HWPX → IR → MD roundtrip stability
// ---------------------------------------------------------------------------

/// Markdown → parse → write_hwpx → read_hwpx → write_markdown:
/// Key structural elements (heading level, bold text, paragraph) must
/// survive the HWPX hop intact.
#[test]
fn md_to_hwpx_to_md_roundtrip_preserves_structure() {
    // Source Markdown with heading, bold paragraph, and plain paragraph.
    let source_md = "# Round Trip\n\n**Bold content** here.\n\nPlain paragraph.\n";

    // MD → IR.
    let ir_doc = hwp2md::md::parse_markdown(source_md);

    // IR → HWPX file.
    let tmp = tempfile::NamedTempFile::new().expect("temp file");
    hwpx::write_hwpx(&ir_doc, tmp.path(), None).expect("write_hwpx");

    // HWPX → IR (re-read).
    let read_back = hwpx::read_hwpx(tmp.path()).expect("read_hwpx");

    // IR → MD (second pass).
    let final_md = md::write_markdown(&read_back, false);

    // H1 heading must survive.
    assert!(
        final_md.contains("# Round Trip"),
        "H1 heading must survive MD→HWPX→MD; got: {final_md:?}"
    );

    // "Round Trip" before "Bold content" before "Plain paragraph" (ordering).
    let pos_heading = final_md.find("Round Trip").expect("heading text");
    let pos_bold = final_md.find("Bold content").expect("bold text");
    let pos_plain = final_md.find("Plain paragraph").expect("plain text");
    assert!(
        pos_heading < pos_bold && pos_bold < pos_plain,
        "order must be heading < bold < plain; got: {final_md:?}"
    );

    // "Bold content" must retain bold markers (** ... **).
    assert!(
        final_md.contains("**Bold content**"),
        "bold must survive MD→HWPX→MD; got: {final_md:?}"
    );
}
