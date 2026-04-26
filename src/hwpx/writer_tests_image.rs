use super::*;

// ── helpers ──────────────────────────────────────────────────────────────

/// Minimal 8-byte PNG magic header used as fake image data in tests.
const PNG_MAGIC: &[u8] = &[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

/// Minimal 3-byte JPEG magic header.
const JPEG_MAGIC: &[u8] = &[0xFF, 0xD8, 0xFF];

/// Write a document to a temp HWPX and return the list of ZIP entry names.
fn write_to_zip_entries(doc: &Document) -> Vec<String> {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    write_hwpx(doc, tmp.path(), None).expect("write_hwpx");
    let file = std::fs::File::open(tmp.path()).expect("open zip");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    (0..archive.len())
        .map(|i| archive.by_index(i).expect("entry").name().to_owned())
        .collect()
}

/// Write a document to a temp HWPX, open the ZIP, and read a named entry's bytes.
fn read_zip_entry_bytes(doc: &Document, entry_name: &str) -> Vec<u8> {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    write_hwpx(doc, tmp.path(), None).expect("write_hwpx");
    let file = std::fs::File::open(tmp.path()).expect("open zip");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive.by_name(entry_name).expect("entry not found");
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).expect("read entry");
    buf
}

/// Write a document to a temp HWPX, open the ZIP, and read a named text entry.
fn read_zip_entry_text(doc: &Document, entry_name: &str) -> String {
    let bytes = read_zip_entry_bytes(doc, entry_name);
    String::from_utf8(bytes).expect("UTF-8 entry")
}

// ── Unit tests: collect_image_assets ─────────────────────────────────────

#[test]
fn collect_image_assets_local_file_reads_bytes_and_maps_src() {
    // Write a fake PNG to disk and make the IR point to it.
    let dir = tempfile::tempdir().expect("tmp dir");
    let img_path = dir.path().join("test.png");
    std::fs::write(&img_path, PNG_MAGIC).expect("write png");

    let src = img_path.to_str().expect("valid utf-8 path").to_owned();
    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::Image {
                src: src.clone(),
                alt: "test image".into(),
            }],
        }],
        assets: Vec::new(),
    };

    let (map, resolved) = collect_image_assets(&doc);

    // The src must be in the map.
    assert!(map.contains_key(&src), "src must be mapped: {map:?}");

    // The entry name must be the bare filename.
    let entry_name = map.get(&src).expect("entry name");
    assert_eq!(
        entry_name, "test.png",
        "entry name must be the bare filename"
    );

    // One resolved asset must be present with correct data and MIME.
    assert_eq!(resolved.len(), 1, "exactly one asset expected");
    assert_eq!(resolved[0].data, PNG_MAGIC, "asset bytes must match file");
    assert_eq!(
        resolved[0].mime_type, "image/png",
        "MIME type must be image/png"
    );
}

#[test]
fn collect_image_assets_data_uri_decodes_base64() {
    // Construct a data URI carrying the base64-encoded PNG magic.
    // PNG_MAGIC_B64 is the base64 of PNG_MAGIC[:6] ≈ correct prefix, but
    // for the test we use the full-round-trip value produced by our encoder.
    // We encode PNG_MAGIC into base64 manually (it is 8 bytes → 12 chars).
    let b64: String = base64_encode_test(PNG_MAGIC);
    let src = format!("data:image/png;base64,{b64}");

    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::Image {
                src: src.clone(),
                alt: "data uri image".into(),
            }],
        }],
        assets: Vec::new(),
    };

    let (map, resolved) = collect_image_assets(&doc);

    assert!(
        map.contains_key(&src),
        "data URI src must be mapped: {map:?}"
    );

    let entry_name = map.get(&src).expect("entry name");
    assert!(
        entry_name.ends_with(".png"),
        "data URI entry must have .png extension: {entry_name}"
    );

    assert_eq!(resolved.len(), 1, "one asset expected");
    assert_eq!(
        resolved[0].data, PNG_MAGIC,
        "decoded data must match original bytes"
    );
    assert_eq!(resolved[0].mime_type, "image/png");
}

#[test]
fn collect_image_assets_http_url_not_embedded() {
    let src = "https://example.com/photo.png".to_owned();
    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::Image {
                src: src.clone(),
                alt: "remote".into(),
            }],
        }],
        assets: Vec::new(),
    };

    let (map, resolved) = collect_image_assets(&doc);

    assert!(
        !map.contains_key(&src),
        "remote URL must NOT be in asset map: {map:?}"
    );
    assert!(
        resolved.is_empty(),
        "no assets must be resolved for remote URL"
    );
}

#[test]
fn collect_image_assets_missing_file_graceful() {
    let src = "/nonexistent/path/image.png".to_owned();
    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::Image {
                src: src.clone(),
                alt: "broken".into(),
            }],
        }],
        assets: Vec::new(),
    };

    // Must not panic; missing file is silently skipped.
    let (map, resolved) = collect_image_assets(&doc);

    assert!(
        !map.contains_key(&src),
        "missing file must not be in asset map"
    );
    assert!(
        resolved.is_empty(),
        "no assets expected for unreadable file"
    );
}

#[test]
fn collect_image_assets_pre_existing_assets_included() {
    // When doc.assets already contains entries (e.g. from an HWPX reader
    // roundtrip), collect_image_assets must include them in the output.
    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::Image {
                src: "photo.png".into(),
                alt: "pre-existing".into(),
            }],
        }],
        assets: vec![Asset {
            name: "photo.png".into(),
            data: PNG_MAGIC.to_vec(),
            mime_type: "image/png".into(),
        }],
    };

    let (map, resolved) = collect_image_assets(&doc);

    // The pre-existing asset name must be in the map.
    assert!(
        map.contains_key("photo.png"),
        "pre-existing asset src must be in map: {map:?}"
    );
    assert_eq!(
        resolved.len(),
        1,
        "exactly one resolved asset expected (no duplication)"
    );
    assert_eq!(resolved[0].data, PNG_MAGIC);
}

// ── Unit tests: write_hwpx BinData ZIP entries ───────────────────────────

#[test]
fn write_hwpx_local_image_creates_bindata_entry() {
    let dir = tempfile::tempdir().expect("tmp dir");
    let img_path = dir.path().join("photo.png");
    std::fs::write(&img_path, PNG_MAGIC).expect("write png");

    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::Image {
                src: img_path.to_str().expect("path utf-8").to_owned(),
                alt: "a photo".into(),
            }],
        }],
        assets: Vec::new(),
    };

    let entries = write_to_zip_entries(&doc);

    let has_bindata = entries
        .iter()
        .any(|e| e.starts_with("BinData/") && e.ends_with("photo.png"));
    assert!(
        has_bindata,
        "BinData/photo.png must be present in HWPX ZIP; entries: {entries:?}"
    );
}

#[test]
fn write_hwpx_local_image_bytes_preserved_in_bindata() {
    let dir = tempfile::tempdir().expect("tmp dir");
    let img_path = dir.path().join("pic.png");
    std::fs::write(&img_path, PNG_MAGIC).expect("write png");

    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::Image {
                src: img_path.to_str().expect("path utf-8").to_owned(),
                alt: "pic".into(),
            }],
        }],
        assets: Vec::new(),
    };

    let bytes = read_zip_entry_bytes(&doc, "BinData/pic.png");
    assert_eq!(bytes, PNG_MAGIC, "BinData bytes must match original file");
}

#[test]
fn write_hwpx_data_uri_image_creates_bindata_entry() {
    let b64 = base64_encode_test(PNG_MAGIC);
    let src = format!("data:image/png;base64,{b64}");

    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::Image {
                src,
                alt: "data uri".into(),
            }],
        }],
        assets: Vec::new(),
    };

    let entries = write_to_zip_entries(&doc);

    let has_bindata = entries
        .iter()
        .any(|e| e.starts_with("BinData/image_") && e.ends_with(".png"));
    assert!(
        has_bindata,
        "BinData/image_N.png must be present for data URI image; entries: {entries:?}"
    );
}

#[test]
fn write_hwpx_http_url_no_bindata_entry() {
    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::Image {
                src: "https://example.com/photo.png".into(),
                alt: "remote".into(),
            }],
        }],
        assets: Vec::new(),
    };

    let entries = write_to_zip_entries(&doc);

    let has_bindata = entries.iter().any(|e| e.starts_with("BinData/"));
    assert!(
        !has_bindata,
        "HTTP URL images must NOT produce BinData entries; entries: {entries:?}"
    );
}

#[test]
fn write_hwpx_content_hpf_has_bindata_manifest_entry() {
    let dir = tempfile::tempdir().expect("tmp dir");
    let img_path = dir.path().join("logo.png");
    std::fs::write(&img_path, PNG_MAGIC).expect("write png");

    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::Image {
                src: img_path.to_str().expect("path utf-8").to_owned(),
                alt: "logo".into(),
            }],
        }],
        assets: Vec::new(),
    };

    let hpf = read_zip_entry_text(&doc, "Contents/content.hpf");

    assert!(
        hpf.contains("hp:binData"),
        "content.hpf must contain <hp:binData> element; hpf:\n{hpf}"
    );
    assert!(
        hpf.contains("logo.png"),
        "content.hpf binData must reference logo.png; hpf:\n{hpf}"
    );
    assert!(
        hpf.contains(r#"type="EMBED""#),
        "content.hpf binData must have type=\"EMBED\"; hpf:\n{hpf}"
    );
}

#[test]
fn write_hwpx_section_xml_uses_entry_name_as_binary_item_id_ref() {
    let dir = tempfile::tempdir().expect("tmp dir");
    let img_path = dir.path().join("myimage.png");
    std::fs::write(&img_path, PNG_MAGIC).expect("write png");

    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::Image {
                src: img_path.to_str().expect("path utf-8").to_owned(),
                alt: "my image".into(),
            }],
        }],
        assets: Vec::new(),
    };

    let section_xml = read_zip_entry_text(&doc, "Contents/section0.xml");

    // The binaryItemIDRef must be the bare entry name, not the full filesystem path.
    assert!(
        section_xml.contains(r#"hp:binaryItemIDRef="myimage.png""#),
        "section XML must use bare filename as binaryItemIDRef; section XML:\n{section_xml}"
    );
}

#[test]
fn write_hwpx_http_url_src_used_verbatim_as_binary_item_id_ref() {
    let url = "https://example.com/photo.png";
    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::Image {
                src: url.into(),
                alt: "remote".into(),
            }],
        }],
        assets: Vec::new(),
    };

    let section_xml = read_zip_entry_text(&doc, "Contents/section0.xml");

    // Remote URLs that are not embedded must still appear as binaryItemIDRef.
    assert!(
        section_xml.contains(url),
        "section XML must contain the original HTTP URL; section XML:\n{section_xml}"
    );
}

#[test]
fn write_hwpx_missing_file_does_not_panic() {
    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::Image {
                src: "/no/such/file.png".into(),
                alt: "broken".into(),
            }],
        }],
        assets: Vec::new(),
    };

    // Must complete without panic even when the image file does not exist.
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let result = write_hwpx(&doc, tmp.path(), None);
    assert!(
        result.is_ok(),
        "write_hwpx must succeed gracefully for missing image file: {result:?}"
    );

    let entries = write_to_zip_entries(&doc);
    let has_bindata = entries.iter().any(|e| e.starts_with("BinData/"));
    assert!(
        !has_bindata,
        "no BinData entry expected for unreadable file; entries: {entries:?}"
    );
}

// ── Roundtrip test ────────────────────────────────────────────────────────

#[test]
fn image_file_path_roundtrip_md_to_hwpx_embeds_bytes() {
    // Write a PNG to disk, build an IR document with a Block::Image pointing
    // to it, write to HWPX, read back, and verify the asset bytes are intact.
    let dir = tempfile::tempdir().expect("tmp dir");
    let img_path = dir.path().join("roundtrip.png");
    std::fs::write(&img_path, PNG_MAGIC).expect("write png");

    let src = img_path.to_str().expect("path utf-8").to_owned();
    let original = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::Image {
                src: src.clone(),
                alt: "roundtrip image".into(),
            }],
        }],
        assets: Vec::new(),
    };

    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    write_hwpx(&original, tmp.path(), None).expect("write_hwpx");

    let read_back = crate::hwpx::read_hwpx(tmp.path()).expect("read_hwpx");

    // The read-back document must have the asset with correct bytes.
    assert_eq!(
        read_back.assets.len(),
        1,
        "exactly one asset expected after roundtrip; assets: {:?}",
        read_back.assets
    );
    assert_eq!(
        read_back.assets[0].data, PNG_MAGIC,
        "asset bytes must match original after roundtrip"
    );
    assert_eq!(
        read_back.assets[0].mime_type, "image/png",
        "MIME type must survive roundtrip"
    );
}

#[test]
fn image_jpeg_extension_gets_correct_mime_type() {
    let dir = tempfile::tempdir().expect("tmp dir");
    let img_path = dir.path().join("photo.jpg");
    std::fs::write(&img_path, JPEG_MAGIC).expect("write jpeg");

    let src = img_path.to_str().expect("path utf-8").to_owned();
    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::Image {
                src,
                alt: "jpeg".into(),
            }],
        }],
        assets: Vec::new(),
    };

    let (_, resolved) = collect_image_assets(&doc);

    assert_eq!(resolved.len(), 1);
    assert_eq!(
        resolved[0].mime_type, "image/jpeg",
        "JPEG extension must map to image/jpeg"
    );
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
/// share the first image's BinData entry).
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

/// Three-way collision: photo.png / photo_2.png already taken → third gets photo_3.png.
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
            std::fs::write(&p, vec![i as u8; 4]).expect("write");
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

// ── test-only base64 encoder ─────────────────────────────────────────────

/// Simple base64 encoder used only inside tests so that `base64_decode`
/// can be tested with round-trip data without pulling in an external crate.
fn base64_encode_test(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i + 2 < data.len() {
        let n = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8) | (data[i + 2] as u32);
        out.push(CHARS[((n >> 18) & 0x3f) as usize] as char);
        out.push(CHARS[((n >> 12) & 0x3f) as usize] as char);
        out.push(CHARS[((n >> 6) & 0x3f) as usize] as char);
        out.push(CHARS[(n & 0x3f) as usize] as char);
        i += 3;
    }
    let rem = data.len() - i;
    if rem == 1 {
        let n = (data[i] as u32) << 16;
        out.push(CHARS[((n >> 18) & 0x3f) as usize] as char);
        out.push(CHARS[((n >> 12) & 0x3f) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let n = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8);
        out.push(CHARS[((n >> 18) & 0x3f) as usize] as char);
        out.push(CHARS[((n >> 12) & 0x3f) as usize] as char);
        out.push(CHARS[((n >> 6) & 0x3f) as usize] as char);
        out.push('=');
    }
    out
}
