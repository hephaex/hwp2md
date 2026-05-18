use super::*;

// ── constants (duplicated from writer_tests_image for module isolation) ───

const PNG_MAGIC: &[u8] = &[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
const JPEG_MAGIC: &[u8] = &[0xFF, 0xD8, 0xFF];

// ── test-only base64 encoder ─────────────────────────────────────────────

/// Simple base64 encoder used only inside tests so that `base64_decode`
/// can be tested with round-trip data without pulling in an external crate.
fn base64_encode_test(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i + 2 < data.len() {
        let n = (u32::from(data[i]) << 16) | (u32::from(data[i + 1]) << 8) | u32::from(data[i + 2]);
        out.push(CHARS[((n >> 18) & 0x3f) as usize] as char);
        out.push(CHARS[((n >> 12) & 0x3f) as usize] as char);
        out.push(CHARS[((n >> 6) & 0x3f) as usize] as char);
        out.push(CHARS[(n & 0x3f) as usize] as char);
        i += 3;
    }
    let rem = data.len() - i;
    if rem == 1 {
        let n = u32::from(data[i]) << 16;
        out.push(CHARS[((n >> 18) & 0x3f) as usize] as char);
        out.push(CHARS[((n >> 12) & 0x3f) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let n = (u32::from(data[i]) << 16) | (u32::from(data[i + 1]) << 8);
        out.push(CHARS[((n >> 18) & 0x3f) as usize] as char);
        out.push(CHARS[((n >> 12) & 0x3f) as usize] as char);
        out.push(CHARS[((n >> 6) & 0x3f) as usize] as char);
        out.push('=');
    }
    out
}

// ── Unit tests: base64_decode ─────────────────────────────────────────────

#[test]
fn base64_decode_hello_world() {
    // "hello" in base64 is "aGVsbG8="
    let result = base64_decode("aGVsbG8=").expect("decode");
    assert_eq!(result, b"hello");
}

#[test]
fn base64_decode_empty_string() {
    let result = base64_decode("").expect("decode empty");
    assert!(result.is_empty());
}

#[test]
fn base64_decode_png_magic_roundtrip() {
    let encoded = base64_encode_test(PNG_MAGIC);
    let decoded = base64_decode(&encoded).expect("decode");
    assert_eq!(decoded, PNG_MAGIC);
}

#[test]
fn base64_decode_invalid_char_returns_error() {
    // '!' is not in the base64 alphabet.
    let result = base64_decode("aGVs!G8=");
    assert!(result.is_err(), "invalid base64 must return error");
}

// ── mime_from_extension ───────────────────────────────────────────────────

#[test]
fn mime_from_extension_png() {
    assert_eq!(mime_from_extension("image.png"), "image/png");
}

#[test]
fn mime_from_extension_jpg() {
    assert_eq!(mime_from_extension("photo.jpg"), "image/jpeg");
}

#[test]
fn mime_from_extension_jpeg() {
    assert_eq!(mime_from_extension("photo.jpeg"), "image/jpeg");
}

#[test]
fn mime_from_extension_gif() {
    assert_eq!(mime_from_extension("anim.gif"), "image/gif");
}

#[test]
fn mime_from_extension_bmp() {
    assert_eq!(mime_from_extension("icon.bmp"), "image/bmp");
}

#[test]
fn mime_from_extension_svg() {
    assert_eq!(mime_from_extension("vector.svg"), "image/svg+xml");
}

#[test]
fn mime_from_extension_unknown_falls_back() {
    assert_eq!(mime_from_extension("file.tiff"), "application/octet-stream");
}

// ── unique_entry_name (filename collision dedup) ──────────────────────────

/// When two images on disk share the same bare filename the second one must
/// be assigned a deduplicated entry name with a `_2` suffix (not silently
/// share the first image's `BinData` entry).
#[test]
fn collect_image_assets_filename_collision_dedup_counter_suffix() {
    let dir = tempfile::tempdir().expect("tmp dir");

    // Create two separate files with the same bare filename but different content.
    let subdir1 = dir.path().join("a");
    let subdir2 = dir.path().join("b");
    std::fs::create_dir_all(&subdir1).expect("create subdir a");
    std::fs::create_dir_all(&subdir2).expect("create subdir b");

    let path1 = subdir1.join("photo.png");
    let path2 = subdir2.join("photo.png");
    std::fs::write(&path1, PNG_MAGIC).expect("write png1");
    // Different bytes so we can distinguish the two assets.
    std::fs::write(&path2, JPEG_MAGIC).expect("write png2 (different bytes)");

    let src1 = path1.to_str().expect("path utf-8").to_owned();
    let src2 = path2.to_str().expect("path utf-8").to_owned();

    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![
                Block::Image {
                    src: src1.clone(),
                    alt: "first".into(),
                },
                Block::Image {
                    src: src2.clone(),
                    alt: "second".into(),
                },
            ],

            page_layout: None,
            ..Default::default()
        }],
        assets: Vec::new(),
    };

    let (map, resolved) = collect_image_assets(&doc);

    // Both srcs must be independently mapped.
    assert!(
        map.contains_key(&src1),
        "first src must be in asset map: {map:?}"
    );
    assert!(
        map.contains_key(&src2),
        "second src must be in asset map: {map:?}"
    );

    let entry1 = map.get(&src1).expect("entry for src1");
    let entry2 = map.get(&src2).expect("entry for src2");

    // The two entry names must be distinct.
    assert_ne!(
        entry1, entry2,
        "colliding filenames must be assigned distinct entry names; got entry1={entry1:?} entry2={entry2:?}"
    );

    // The first entry keeps the original name; the second gets a suffix.
    assert_eq!(
        entry1, "photo.png",
        "first image keeps bare filename: {entry1:?}"
    );
    assert_eq!(
        entry2, "photo_2.png",
        "second colliding image gets _2 suffix: {entry2:?}"
    );

    // Two distinct resolved assets must be present with their correct bytes.
    assert_eq!(
        resolved.len(),
        2,
        "two resolved assets expected (no merging): {resolved:?}"
    );

    let bytes1 = resolved
        .iter()
        .find(|r| r.entry_name == "photo.png")
        .map(|r| r.data.as_slice())
        .expect("photo.png entry");
    let bytes2 = resolved
        .iter()
        .find(|r| r.entry_name == "photo_2.png")
        .map(|r| r.data.as_slice())
        .expect("photo_2.png entry");

    assert_eq!(bytes1, PNG_MAGIC, "first asset bytes must match path1");
    assert_eq!(bytes2, JPEG_MAGIC, "second asset bytes must match path2");
}

/// Three-way collision: photo.png / `photo_2.png` already taken → third gets `photo_3.png`.
#[test]
fn collect_image_assets_three_way_collision_increments_counter() {
    let dir = tempfile::tempdir().expect("tmp dir");

    let dirs: Vec<_> = (0..3)
        .map(|i| {
            let d = dir.path().join(format!("d{i}"));
            std::fs::create_dir_all(&d).expect("subdir");
            d
        })
        .collect();

    let paths: Vec<_> = dirs
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let p = d.join("img.png");
            // Use different byte patterns to tell assets apart.
            std::fs::write(&p, vec![u8::try_from(i).unwrap(); 4]).expect("write");
            p
        })
        .collect();

    let srcs: Vec<String> = paths
        .iter()
        .map(|p| p.to_str().expect("utf-8").to_owned())
        .collect();

    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: srcs
                .iter()
                .map(|s| Block::Image {
                    src: s.clone(),
                    alt: String::new(),
                })
                .collect(),

            page_layout: None,
            ..Default::default()
        }],
        assets: Vec::new(),
    };

    let (map, resolved) = collect_image_assets(&doc);

    assert_eq!(resolved.len(), 3, "three distinct assets expected");

    let names: Vec<_> = srcs
        .iter()
        .map(|s| map.get(s).expect("mapped").as_str())
        .collect();

    assert_eq!(names[0], "img.png");
    assert_eq!(names[1], "img_2.png");
    assert_eq!(names[2], "img_3.png");
}
