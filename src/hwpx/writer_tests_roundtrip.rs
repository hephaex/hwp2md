use super::*;

// ── Roundtrip / integration tests ────────────────────────────────────────

#[test]
fn write_hwpx_image_roundtrip_preserves_asset() {
    let png_bytes = vec![0x89u8, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");

    let original = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::Image {
                src: "photo.png".into(),
                alt: "a photo".into(),
            }],
        }],
        assets: vec![Asset {
            name: "photo.png".into(),
            data: png_bytes.clone(),
            mime_type: "image/png".into(),
        }],
    };
    write_hwpx(&original, tmp.path(), None).expect("write");

    let read_back = read_hwpx(tmp.path()).expect("read_hwpx");

    let has_image = read_back
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .any(|b| matches!(b, Block::Image { src, .. } if src.contains("photo")));
    assert!(
        has_image,
        "image block must survive HWPX roundtrip; sections: {:?}",
        read_back.sections
    );

    assert_eq!(
        read_back.assets.len(),
        1,
        "one asset expected after roundtrip"
    );
    assert_eq!(
        read_back.assets[0].data, png_bytes,
        "asset binary content must be preserved through roundtrip"
    );
    assert_eq!(
        read_back.assets[0].mime_type, "image/png",
        "asset MIME type must be preserved"
    );
}
