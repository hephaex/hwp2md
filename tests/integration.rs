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
