/// Integration tests for HWPX image block (`<hp:img>`) handling.
///
/// Covers both the `binaryItemIDRef` attribute path (the canonical OWPML
/// embedding) and the alternate `src` attribute path.  Tests use
/// `HwpxFixture::bin_data` to embed minimal PNG bytes so the fixture is
/// a well-formed HWPX ZIP.
///
/// Extracted from integration.rs (Sprint 84 P4) to keep each test file focused.
#[path = "fixtures/mod.rs"]
#[allow(dead_code)]
mod fixtures;

use fixtures::{read_fixture, HwpxFixture};
use hwp2md::{ir, md};

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
