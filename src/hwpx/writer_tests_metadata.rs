use super::*;

// ── write_hwpx integration: ZIP entry presence ─────────────────────────

#[test]
fn write_hwpx_empty_doc_produces_required_entries() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = Document::new();
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");

    let entries = zip_entry_names(tmp.path());
    assert!(entries.contains(&"mimetype".to_owned()), "{entries:?}");
    assert!(
        entries.contains(&"META-INF/container.xml".to_owned()),
        "{entries:?}"
    );
    assert!(
        entries.contains(&"Contents/header.xml".to_owned()),
        "{entries:?}"
    );
    assert!(
        entries.contains(&"Contents/content.hpf".to_owned()),
        "{entries:?}"
    );
    assert!(
        entries.contains(&"Contents/section0.xml".to_owned()),
        "{entries:?}"
    );
}

#[test]
fn write_hwpx_mimetype_is_stored_uncompressed() {
    // HWPX spec: mimetype must use Stored (no compression).
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    write_hwpx(&Document::new(), tmp.path(), None).expect("write");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let entry = archive.by_name("mimetype").expect("mimetype entry");
    assert_eq!(
        entry.compression(),
        zip::CompressionMethod::Stored,
        "mimetype must be Stored"
    );
}

#[test]
fn write_hwpx_mimetype_content() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    write_hwpx(&Document::new(), tmp.path(), None).expect("write");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive.by_name("mimetype").expect("mimetype entry");
    let mut content = String::new();
    entry.read_to_string(&mut content).expect("read");
    assert_eq!(content, "application/hwp+zip");
}

#[test]
fn write_hwpx_single_section_produces_section0_xml() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = doc_with_section(vec![Block::Paragraph {
        inlines: vec![inline("hello")],
    }]);
    write_hwpx(&doc, tmp.path(), None).expect("write");

    let entries = zip_entry_names(tmp.path());
    assert!(
        entries.contains(&"Contents/section0.xml".to_owned()),
        "{entries:?}"
    );
    // With one explicit section there should NOT be a duplicate section0.
    assert_eq!(
        entries.iter().filter(|e| e.contains("section")).count(),
        1,
        "exactly one section entry: {entries:?}"
    );
}

#[test]
fn write_hwpx_two_sections_produces_section0_and_section1() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![
            Section {
                blocks: vec![Block::Paragraph {
                    inlines: vec![inline("s0")],
                }],
            },
            Section {
                blocks: vec![Block::Paragraph {
                    inlines: vec![inline("s1")],
                }],
            },
        ],
        assets: Vec::new(),
    };
    write_hwpx(&doc, tmp.path(), None).expect("write");

    let entries = zip_entry_names(tmp.path());
    assert!(
        entries.contains(&"Contents/section0.xml".to_owned()),
        "{entries:?}"
    );
    assert!(
        entries.contains(&"Contents/section1.xml".to_owned()),
        "{entries:?}"
    );
}

#[test]
fn write_hwpx_with_bindata_asset_produces_bindata_entry() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = Document {
        metadata: Metadata::default(),
        sections: Vec::new(),
        assets: vec![Asset {
            name: "photo.png".into(),
            data: vec![0x89, 0x50, 0x4e, 0x47],
            mime_type: "image/png".into(),
        }],
    };
    write_hwpx(&doc, tmp.path(), None).expect("write");

    let entries = zip_entry_names(tmp.path());
    assert!(
        entries.contains(&"BinData/photo.png".to_owned()),
        "{entries:?}"
    );
}

#[test]
fn write_hwpx_asset_with_path_prefix_uses_basename_only() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = Document {
        metadata: Metadata::default(),
        sections: Vec::new(),
        assets: vec![Asset {
            name: "/some/nested/path/image.jpg".into(),
            data: vec![0xFF, 0xD8],
            mime_type: "image/jpeg".into(),
        }],
    };
    write_hwpx(&doc, tmp.path(), None).expect("write");

    let entries = zip_entry_names(tmp.path());
    // Only the basename should be used inside BinData/.
    assert!(
        entries.contains(&"BinData/image.jpg".to_owned()),
        "{entries:?}"
    );
    assert!(
        !entries.iter().any(|e| e.contains("/some/nested/")),
        "path prefix must be stripped: {entries:?}"
    );
}

#[test]
fn write_hwpx_header_xml_has_version_and_sec_cnt() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = doc_with_section(vec![]);
    write_hwpx(&doc, tmp.path(), None).expect("write");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive.by_name("Contents/header.xml").expect("header.xml");
    let mut content = String::new();
    entry.read_to_string(&mut content).expect("read");
    assert!(
        content.contains(r#"version="1.1""#),
        "version attr: {content}"
    );
    assert!(
        content.contains(r#"secCnt="1""#),
        "secCnt attr: {content}"
    );
}

#[test]
fn write_hwpx_content_hpf_references_sections() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = doc_with_section(vec![]);
    write_hwpx(&doc, tmp.path(), None).expect("write");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive
        .by_name("Contents/content.hpf")
        .expect("content.hpf");
    let mut content = String::new();
    entry.read_to_string(&mut content).expect("read");
    assert!(content.contains("section0.xml"), "{content}");
}

#[test]
fn write_hwpx_bindata_entry_has_correct_content() {
    let png_bytes = vec![0x89u8, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = Document {
        metadata: Metadata::default(),
        sections: Vec::new(),
        assets: vec![Asset {
            name: "banner.png".into(),
            data: png_bytes.clone(),
            mime_type: "image/png".into(),
        }],
    };
    write_hwpx(&doc, tmp.path(), None).expect("write");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive
        .by_name("BinData/banner.png")
        .expect("BinData/banner.png");
    let mut actual = Vec::new();
    entry.read_to_end(&mut actual).expect("read");
    assert_eq!(
        actual, png_bytes,
        "BinData entry content must match asset data"
    );
}

#[test]
fn write_hwpx_image_block_xml_references_asset_name() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::Image {
                src: "diagram.png".into(),
                alt: "a diagram".into(),
            }],
        }],
        assets: vec![Asset {
            name: "diagram.png".into(),
            data: vec![0x89, 0x50, 0x4e, 0x47],
            mime_type: "image/png".into(),
        }],
    };
    write_hwpx(&doc, tmp.path(), None).expect("write");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive
        .by_name("Contents/section0.xml")
        .expect("section0.xml");
    let mut content = String::new();
    entry.read_to_string(&mut content).expect("read");

    assert!(
        content.contains("diagram.png"),
        "section XML must reference image asset name; got: {content}"
    );
    assert!(
        content.contains("<hp:img"),
        "section XML must contain hp:img element; got: {content}"
    );
    assert!(
        content.contains(r#"alt="a diagram""#),
        "section XML must carry alt text; got: {content}"
    );
    let entries = zip_entry_names(tmp.path());
    assert!(
        entries.contains(&"BinData/diagram.png".to_owned()),
        "BinData entry must exist: {entries:?}"
    );
}

// ── Phase 8 tests: metadata in content.hpf ──────────────────────────────

#[test]
fn content_hpf_with_title_and_author() {
    let doc = Document {
        metadata: Metadata {
            title: Some("Test Title".into()),
            author: Some("Test Author".into()),
            ..Metadata::default()
        },
        sections: vec![Section {
            blocks: vec![Block::Paragraph {
                inlines: vec![inline("hello")],
            }],
        }],
        assets: Vec::new(),
    };
    let hpf = generate_content_hpf(&doc);

    assert!(
        hpf.contains("<hp:docInfo>"),
        "docInfo section must be present: {hpf}"
    );
    assert!(
        hpf.contains("<hp:title>Test Title</hp:title>"),
        "title must be present: {hpf}"
    );
    assert!(
        hpf.contains("<hp:author>Test Author</hp:author>"),
        "author must be present: {hpf}"
    );
    assert!(
        hpf.contains("</hp:docInfo>"),
        "docInfo closing tag: {hpf}"
    );
}

#[test]
fn content_hpf_without_metadata_has_no_docinfo() {
    let doc = Document::new();
    let hpf = generate_content_hpf(&doc);

    assert!(
        !hpf.contains("<hp:docInfo>"),
        "docInfo must NOT appear when metadata is empty: {hpf}"
    );
}

#[test]
fn content_hpf_title_only() {
    let doc = Document {
        metadata: Metadata {
            title: Some("Only Title".into()),
            ..Metadata::default()
        },
        sections: Vec::new(),
        assets: Vec::new(),
    };
    let hpf = generate_content_hpf(&doc);

    assert!(
        hpf.contains("<hp:title>Only Title</hp:title>"),
        "title: {hpf}"
    );
    assert!(
        !hpf.contains("<hp:author>"),
        "author must NOT appear when absent: {hpf}"
    );
}

#[test]
fn content_hpf_author_only() {
    let doc = Document {
        metadata: Metadata {
            author: Some("Only Author".into()),
            ..Metadata::default()
        },
        sections: Vec::new(),
        assets: Vec::new(),
    };
    let hpf = generate_content_hpf(&doc);

    assert!(
        !hpf.contains("<hp:title>"),
        "title must NOT appear: {hpf}"
    );
    assert!(
        hpf.contains("<hp:author>Only Author</hp:author>"),
        "author: {hpf}"
    );
}

#[test]
fn content_hpf_metadata_xml_escaping() {
    let doc = Document {
        metadata: Metadata {
            title: Some("A & B <C>".into()),
            author: Some("D & E".into()),
            ..Metadata::default()
        },
        sections: Vec::new(),
        assets: Vec::new(),
    };
    let hpf = generate_content_hpf(&doc);

    assert!(
        hpf.contains("A &amp; B &lt;C&gt;"),
        "title must be XML-escaped: {hpf}"
    );
    assert!(
        hpf.contains("D &amp; E"),
        "author must be XML-escaped: {hpf}"
    );
}

#[test]
fn write_hwpx_metadata_in_content_hpf() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = Document {
        metadata: Metadata {
            title: Some("HWPX Title".into()),
            author: Some("HWPX Author".into()),
            ..Metadata::default()
        },
        sections: vec![Section {
            blocks: vec![Block::Paragraph {
                inlines: vec![inline("body")],
            }],
        }],
        assets: Vec::new(),
    };
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");

    let file = std::fs::File::open(tmp.path()).expect("open");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");
    let mut entry = archive
        .by_name("Contents/content.hpf")
        .expect("content.hpf");
    let mut content = String::new();
    entry.read_to_string(&mut content).expect("read");

    assert!(
        content.contains("<hp:title>HWPX Title</hp:title>"),
        "title in content.hpf: {content}"
    );
    assert!(
        content.contains("<hp:author>HWPX Author</hp:author>"),
        "author in content.hpf: {content}"
    );
}
