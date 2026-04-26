use super::*;

// -----------------------------------------------------------------------
// BinData reference resolution -- resolve_bin_refs + build_bin_map
// -----------------------------------------------------------------------

/// Helper: build a section containing a single top-level Image block.
fn make_image_section(src: &str) -> ir::Section {
    ir::Section {
        blocks: vec![ir::Block::Image {
            src: src.to_string(),
            alt: String::new(),
        }],
        page_layout: None,
    }
}

#[test]
fn resolve_bin_refs_replaces_image_src() {
    // An Image whose src matches a BinData stem must be updated to the
    // full ZIP path, including the extension.
    let bin_files = vec!["BinData/BIN0001.png".to_string()];
    let bin_map = build_bin_map(&bin_files);

    let mut section = make_image_section("BIN0001");
    resolve_bin_refs(&mut section, &bin_map);

    match &section.blocks[0] {
        ir::Block::Image { src, .. } => {
            assert_eq!(
                src, "BinData/BIN0001.png",
                "src must be resolved to full path"
            );
        }
        other => panic!("expected Image, got {other:?}"),
    }
}

#[test]
fn resolve_bin_refs_no_match_leaves_src_unchanged() {
    // An Image with a src that has no entry in the bin_map must not be
    // modified -- e.g. when src is already a full filename or an URL.
    let bin_files = vec!["BinData/BIN0001.png".to_string()];
    let bin_map = build_bin_map(&bin_files);

    let mut section = make_image_section("img.png");
    resolve_bin_refs(&mut section, &bin_map);

    match &section.blocks[0] {
        ir::Block::Image { src, .. } => {
            assert_eq!(src, "img.png", "unmatched src must remain unchanged");
        }
        other => panic!("expected Image, got {other:?}"),
    }
}

#[test]
fn resolve_bin_refs_inside_table_cell() {
    // resolve_block_bin_refs must recurse into Table -> rows -> cells -> blocks.
    let bin_files = vec!["BinData/BIN0002.jpg".to_string()];
    let bin_map = build_bin_map(&bin_files);

    let cell_image = ir::Block::Image {
        src: "BIN0002".to_string(),
        alt: String::new(),
    };
    let cell = ir::TableCell {
        blocks: vec![cell_image],
        colspan: 1,
        rowspan: 1,
    };
    let row = ir::TableRow {
        cells: vec![cell],
        is_header: false,
    };
    let mut section = ir::Section {
        blocks: vec![ir::Block::Table {
            rows: vec![row],
            col_count: 1,
        }],

        page_layout: None,
    };

    resolve_bin_refs(&mut section, &bin_map);

    match &section.blocks[0] {
        ir::Block::Table { rows, .. } => match &rows[0].cells[0].blocks[0] {
            ir::Block::Image { src, .. } => {
                assert_eq!(
                    src, "BinData/BIN0002.jpg",
                    "image inside table cell must be resolved"
                );
            }
            other => panic!("expected Image inside cell, got {other:?}"),
        },
        other => panic!("expected Table, got {other:?}"),
    }
}

#[test]
fn bin_map_from_bin_files() {
    // build_bin_map must produce a map with stem keys and full-path values.
    // It must handle both prefixes (BinData/ and Contents/BinData/).
    let bin_files = vec![
        "BinData/BIN0001.png".to_string(),
        "BinData/BIN0002.jpg".to_string(),
        "Contents/BinData/BIN0003.emf".to_string(),
    ];
    let map = build_bin_map(&bin_files);

    assert_eq!(
        map.get("BIN0001").map(String::as_str),
        Some("BinData/BIN0001.png")
    );
    assert_eq!(
        map.get("BIN0002").map(String::as_str),
        Some("BinData/BIN0002.jpg")
    );
    assert_eq!(
        map.get("BIN0003").map(String::as_str),
        Some("Contents/BinData/BIN0003.emf")
    );
    assert_eq!(map.len(), 3, "map must contain exactly 3 entries");
}
