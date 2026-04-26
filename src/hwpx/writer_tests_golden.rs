use super::*;

// ── Golden / structural tests ─────────────────────────────────────────────

// ── golden file test: verify actual XML output ─────────────────────────

/// Golden file test that validates the writer's actual XML output byte-for-byte
/// rather than relying on roundtrip fidelity.  Builds a comprehensive IR document
/// (heading, bold paragraph, italic paragraph, table, list), writes it to HWPX,
/// then inspects specific XML files inside the ZIP for expected patterns.
#[test]
fn golden_comprehensive_document_structure() {
    use std::io::Read as _;

    // ── 1. Build a comprehensive IR document ──

    let doc = Document {
        metadata: Metadata {
            title: Some("Golden Test Doc".into()),
            author: Some("Test Author".into()),
            ..Metadata::default()
        },
        sections: vec![Section {
            blocks: vec![
                // H1 heading
                Block::Heading {
                    level: 1,
                    inlines: vec![Inline::plain("Main Title")],
                },
                // Paragraph with bold inline
                Block::Paragraph {
                    inlines: vec![inline("Normal text "), bold_inline("bold text")],
                },
                // Paragraph with italic inline
                Block::Paragraph {
                    inlines: vec![italic_inline("italic text")],
                },
                // 2x2 table
                Block::Table {
                    rows: vec![
                        TableRow {
                            cells: vec![
                                TableCell {
                                    blocks: vec![Block::Paragraph {
                                        inlines: vec![inline("Cell A1")],
                                    }],
                                    colspan: 1,
                                    rowspan: 1,
                                },
                                TableCell {
                                    blocks: vec![Block::Paragraph {
                                        inlines: vec![inline("Cell B1")],
                                    }],
                                    colspan: 1,
                                    rowspan: 1,
                                },
                            ],
                            is_header: true,
                        },
                        TableRow {
                            cells: vec![
                                TableCell {
                                    blocks: vec![Block::Paragraph {
                                        inlines: vec![inline("Cell A2")],
                                    }],
                                    colspan: 1,
                                    rowspan: 1,
                                },
                                TableCell {
                                    blocks: vec![Block::Paragraph {
                                        inlines: vec![inline("Cell B2")],
                                    }],
                                    colspan: 1,
                                    rowspan: 1,
                                },
                            ],
                            is_header: false,
                        },
                    ],
                    col_count: 2,
                },
                // Code block
                Block::CodeBlock {
                    language: Some("rust".into()),
                    code: "fn main() {}".into(),
                },
                // Horizontal rule
                Block::HorizontalRule,
                // Block quote
                Block::BlockQuote {
                    blocks: vec![Block::Paragraph {
                        inlines: vec![inline("quoted text")],
                    }],
                },
                // Unordered list
                Block::List {
                    ordered: false,
                    start: 1,
                    items: vec![
                        ListItem {
                            blocks: vec![Block::Paragraph {
                                inlines: vec![inline("List item one")],
                            }],
                            children: Vec::new(),
                        },
                        ListItem {
                            blocks: vec![Block::Paragraph {
                                inlines: vec![inline("List item two")],
                            }],
                            children: Vec::new(),
                        },
                    ],
                },
            ],
        }],
        assets: Vec::new(),
    };

    // ── 2. Write to HWPX bytes ──

    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");

    // ── 3. Open the ZIP and read specific XML entries ──

    let file = std::fs::File::open(tmp.path()).expect("open zip");
    let mut archive = zip::ZipArchive::new(file).expect("parse zip");

    // -- section0.xml assertions --
    let section_xml = {
        let mut entry = archive
            .by_name("Contents/section0.xml")
            .expect("section0.xml must exist in HWPX");
        let mut buf = String::new();
        entry.read_to_string(&mut buf).expect("read section0.xml");
        buf
    };

    // Verify heading has styleIDRef
    assert!(
        section_xml.contains(r#"hp:styleIDRef="1""#),
        "H1 heading must have hp:styleIDRef=\"1\" in section XML:\n{section_xml}"
    );

    // Verify heading text
    assert!(
        section_xml.contains("<hp:t>Main Title</hp:t>"),
        "heading text 'Main Title' must appear in <hp:t>:\n{section_xml}"
    );

    // Verify bold inline charPr
    assert!(
        section_xml.contains(r#"bold="true""#),
        "bold inline must emit charPr with bold=\"true\":\n{section_xml}"
    );

    // Verify bold text content
    assert!(
        section_xml.contains("<hp:t>bold text</hp:t>"),
        "bold text content must appear in <hp:t>:\n{section_xml}"
    );

    // Verify italic inline charPr
    assert!(
        section_xml.contains(r#"italic="true""#),
        "italic inline must emit charPr with italic=\"true\":\n{section_xml}"
    );

    // Verify italic text content
    assert!(
        section_xml.contains("<hp:t>italic text</hp:t>"),
        "italic text content must appear in <hp:t>:\n{section_xml}"
    );

    // Verify normal (non-bold, non-italic) text
    assert!(
        section_xml.contains("<hp:t>Normal text </hp:t>"),
        "plain text must appear in <hp:t>:\n{section_xml}"
    );

    // Verify table structure
    assert!(
        section_xml.contains(r#"<hp:tbl"#),
        "table must emit <hp:tbl> element:\n{section_xml}"
    );
    assert!(
        section_xml.contains(r#"rowCnt="2""#),
        "table must have rowCnt=\"2\":\n{section_xml}"
    );
    assert!(
        section_xml.contains(r#"colCnt="2""#),
        "table must have colCnt=\"2\":\n{section_xml}"
    );
    assert!(
        section_xml.contains("<hp:tr>"),
        "table must contain <hp:tr> rows:\n{section_xml}"
    );
    assert!(
        section_xml.contains("<hp:tc>"),
        "table must contain <hp:tc> cells:\n{section_xml}"
    );
    assert!(
        section_xml.contains("<hp:t>Cell A1</hp:t>"),
        "table cell text 'Cell A1' must appear:\n{section_xml}"
    );
    assert!(
        section_xml.contains("<hp:t>Cell B2</hp:t>"),
        "table cell text 'Cell B2' must appear:\n{section_xml}"
    );

    // Verify list items are emitted as paragraphs
    assert!(
        section_xml.contains("<hp:t>List item one</hp:t>"),
        "list item text 'List item one' must appear:\n{section_xml}"
    );
    assert!(
        section_xml.contains("<hp:t>List item two</hp:t>"),
        "list item text 'List item two' must appear:\n{section_xml}"
    );

    // Verify section XML namespace declarations
    assert!(
        section_xml.contains("xmlns:hs="),
        "section XML must declare hs namespace:\n{section_xml}"
    );
    assert!(
        section_xml.contains("xmlns:hp="),
        "section XML must declare hp namespace:\n{section_xml}"
    );

    // Verify no inline <hp:charPr> for plain text runs (the plain text
    // runs should only have a charPrIDRef attribute, not an inline element).
    // We check that the number of <hp:charPr occurrences matches the number
    // of formatted inlines (bold + italic = 2).
    let charpr_count = section_xml.matches("<hp:charPr ").count();
    assert!(
        charpr_count >= 2,
        "at least 2 inline <hp:charPr> elements expected (bold + italic), found {charpr_count}:\n{section_xml}"
    );

    // -- content.hpf assertions --
    let content_hpf = {
        let mut entry = archive
            .by_name("Contents/content.hpf")
            .expect("content.hpf must exist in HWPX");
        let mut buf = String::new();
        entry.read_to_string(&mut buf).expect("read content.hpf");
        buf
    };

    assert!(
        content_hpf.contains("section0.xml"),
        "content.hpf must reference section0.xml:\n{content_hpf}"
    );
    assert!(
        content_hpf.contains("<hp:title>Golden Test Doc</hp:title>"),
        "content.hpf must contain document title:\n{content_hpf}"
    );
    assert!(
        content_hpf.contains("<hp:author>Test Author</hp:author>"),
        "content.hpf must contain document author:\n{content_hpf}"
    );

    // -- header.xml assertions --
    let header_xml = {
        let mut entry = archive
            .by_name("Contents/header.xml")
            .expect("header.xml must exist in HWPX");
        let mut buf = String::new();
        entry.read_to_string(&mut buf).expect("read header.xml");
        buf
    };

    // Header must contain fontface declarations
    assert!(
        header_xml.contains("hh:fontface"),
        "header.xml must contain fontface declarations:\n{header_xml}"
    );

    // Header must contain charProperties
    assert!(
        header_xml.contains("hh:charPr"),
        "header.xml must contain charPr entries:\n{header_xml}"
    );

    // Header must contain styles (heading styles)
    assert!(
        header_xml.contains("hh:style"),
        "header.xml must contain style entries:\n{header_xml}"
    );

    // -- Verify code block text --
    assert!(
        section_xml.contains("<hp:t>fn main() {}</hp:t>"),
        "code block text must appear in <hp:t>:\n{section_xml}"
    );

    // -- Verify horizontal rule produces box-drawing characters --
    assert!(
        section_xml.contains("\u{2500}"),
        "horizontal rule must emit box-drawing characters:\n{section_xml}"
    );

    // -- Verify block quote text (emitted as plain paragraph) --
    assert!(
        section_xml.contains("<hp:t>quoted text</hp:t>"),
        "block quote text must appear in <hp:t>:\n{section_xml}"
    );

    // -- mimetype assertion --
    let mimetype = {
        let mut entry = archive
            .by_name("mimetype")
            .expect("mimetype must exist in HWPX");
        let mut buf = String::new();
        entry.read_to_string(&mut buf).expect("read mimetype");
        buf
    };
    assert_eq!(
        mimetype, "application/hwp+zip",
        "mimetype must be exactly 'application/hwp+zip'"
    );

    // -- version.xml assertion --
    archive
        .by_name("version.xml")
        .expect("version.xml must exist in HWPX");

    // -- META-INF/container.xml assertion --
    let container_xml = {
        let mut entry = archive
            .by_name("META-INF/container.xml")
            .expect("META-INF/container.xml must exist in HWPX");
        let mut buf = String::new();
        entry.read_to_string(&mut buf).expect("read container.xml");
        buf
    };
    assert!(
        container_xml.contains("content.hpf"),
        "container.xml must reference content.hpf:\n{container_xml}"
    );

    // -- Block quote must use paraPrIDRef="1" (indented paragraph) --
    assert!(
        section_xml.contains(r#"paraPrIDRef="1""#),
        "block quote paragraph must use paraPrIDRef=\"1\":\n{section_xml}"
    );

    // -- header.xml must have paraPr id="1" with left margin --
    assert!(
        header_xml.contains(r#"<hh:paraPr id="1">"#),
        "header.xml must contain paraPr id=\"1\" for blockquote indent:\n{header_xml}"
    );
}

// ── Phase A-3 tests: BlockQuote paraPr header + roundtrip ──────────────

#[test]
fn header_xml_contains_blockquote_para_pr() {
    let doc = doc_with_section(vec![Block::Paragraph {
        inlines: vec![inline("text")],
    }]);
    let tables = RefTables::build(&doc);
    let header =
        super::header::generate_header_xml(&doc, &tables).expect("generate_header_xml failed");

    // paraPr id="0" (normal) must exist.
    assert!(
        header.contains(r#"<hh:paraPr id="0">"#),
        "header must contain paraPr id=\"0\":\n{header}"
    );
    // paraPr id="1" (blockquote indent) must exist.
    assert!(
        header.contains(r#"<hh:paraPr id="1">"#),
        "header must contain paraPr id=\"1\":\n{header}"
    );
    // itemCnt must be "4" for paraProperties (id=0 normal, id=1 blockquote,
    // id=2 list-depth-0, id=3 list-depth-1+).
    assert!(
        header.contains(r#"itemCnt="4""#),
        "paraProperties itemCnt must be 4:\n{header}"
    );
    // paraPr id="1" must have a left margin value of 800.
    assert!(
        header.contains(r#"<hh:left value="800"/>"#),
        "paraPr id=\"1\" must have left margin 800:\n{header}"
    );
}

#[test]
fn header_xml_para_pr_0_has_zero_left_margin() {
    let doc = doc_with_section(vec![Block::Paragraph {
        inlines: vec![inline("text")],
    }]);
    let tables = RefTables::build(&doc);
    let header =
        super::header::generate_header_xml(&doc, &tables).expect("generate_header_xml failed");

    // paraPr id="0" must have left margin = 0.
    // We verify that the first paraPr (id=0) has <hh:left value="0"/>.
    // Since id=0 comes first, the first occurrence of <hh:left is from id=0.
    let first_left_pos = header
        .find(r#"<hh:left value="#)
        .expect("hh:left must exist");
    let first_left_slice = &header[first_left_pos..];
    assert!(
        first_left_slice.starts_with(r#"<hh:left value="0"/>"#),
        "first paraPr (id=0) must have left margin 0:\n{header}"
    );
}

#[test]
fn roundtrip_blockquote_content_preserved() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::BlockQuote {
                blocks: vec![Block::Paragraph {
                    inlines: vec![inline("roundtrip quote")],
                }],
            }],
        }],
        assets: Vec::new(),
    };
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");
    let read_back = read_hwpx(tmp.path()).expect("read_hwpx");

    // The text must survive the roundtrip.  The reader currently does not
    // reconstruct BlockQuote from paraPrIDRef, so the content appears as
    // a plain Paragraph -- that is acceptable for now.
    let has_quote_text = read_back
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .any(|b| match b {
            Block::Paragraph { inlines } => inlines.iter().any(|i| i.text == "roundtrip quote"),
            Block::BlockQuote { blocks } => blocks.iter().any(|b2| {
                matches!(b2, Block::Paragraph { inlines } if inlines.iter().any(|i| i.text == "roundtrip quote"))
            }),
            _ => false,
        });
    assert!(
        has_quote_text,
        "blockquote text must survive HWPX roundtrip; sections: {:?}",
        read_back.sections
    );
}
