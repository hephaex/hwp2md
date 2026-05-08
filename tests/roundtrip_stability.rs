/// Integration tests: MD → IR → MD stability and header/footer roundtrips.
use hwp2md::ir;
use hwp2md::md::{parse_markdown, write_markdown};

#[path = "common/mod.rs"]
mod common;

use common::{first_blocks, make_doc, plain};

// -----------------------------------------------------------------------
// MD → IR → MD: stability tests
// -----------------------------------------------------------------------

#[test]
fn roundtrip_md_to_ir_to_md_stable_display_math() {
    // The writer emits $$\n…\n$$ for display math; ensure the formula survives.
    let original = make_doc(vec![ir::Block::Math {
        display: true,
        tex: "E=mc^2".into(),
    }]);
    let md1 = write_markdown(&original, false);

    let doc2 = parse_markdown(&md1);
    let md2 = write_markdown(&doc2, false);

    // Both passes must contain the formula.
    assert!(
        md1.contains("E=mc^2"),
        "formula missing from pass 1; md1: {md1:?}"
    );
    assert!(
        md2.contains("E=mc^2"),
        "formula missing from pass 2; md2: {md2:?}"
    );
}

#[test]
fn roundtrip_md_to_ir_to_md_stable_footnote() {
    let source = "Text[^1]\n\n[^1]: footnote body\n";
    let doc1 = parse_markdown(source);
    let md1 = write_markdown(&doc1, false);
    let doc2 = parse_markdown(&md1);
    let md2 = write_markdown(&doc2, false);

    assert!(
        md1.contains("[^1]"),
        "footnote ref missing from pass 1; md1: {md1:?}"
    );
    assert!(
        md2.contains("[^1]"),
        "footnote ref missing from pass 2; md2: {md2:?}"
    );
    assert!(
        md1.contains("footnote body"),
        "footnote body missing from pass 1; md1: {md1:?}"
    );
    assert!(
        md2.contains("footnote body"),
        "footnote body missing from pass 2; md2: {md2:?}"
    );
}

#[test]
fn roundtrip_md_to_ir_to_md_escaped_text_preserved() {
    // Backslash-escaped metacharacters: the parser strips the backslash and
    // produces plain text; the writer re-escapes on output.  The underlying
    // text content (without backslashes) must survive both passes.
    let source = "a\\*b\\_c\\~d\n";
    let doc1 = parse_markdown(source);
    let md1 = write_markdown(&doc1, false);
    let doc2 = parse_markdown(&md1);
    let md2 = write_markdown(&doc2, false);

    // Plain text "a*b_c~d" must be present in both IR representations.
    let text1: String = first_blocks(&doc1)
        .iter()
        .filter_map(|b| {
            if let ir::Block::Paragraph { inlines } = b {
                Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
            } else {
                None
            }
        })
        .collect();
    let text2: String = first_blocks(&doc2)
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
        text1.contains("a*b_c~d"),
        "plain text lost in pass 1; text1: {text1:?}"
    );
    assert!(
        text2.contains("a*b_c~d"),
        "plain text lost in pass 2; text2: {text2:?}"
    );
    // The writer must re-escape the metacharacters.
    assert!(md1.contains("\\*"), "asterisk not re-escaped; md1: {md1:?}");
    assert!(md2.contains("\\*"), "asterisk not re-escaped; md2: {md2:?}");
}

#[test]
fn roundtrip_md_to_ir_to_md_stable_image() {
    // comrak wraps a standalone image in a Paragraph and records it as an
    // inline.  The roundtrip should preserve the src and alt attributes.
    let source = "![alt text](image.png)\n";
    let doc1 = parse_markdown(source);
    let md1 = write_markdown(&doc1, false);
    let doc2 = parse_markdown(&md1);
    let md2 = write_markdown(&doc2, false);

    // Both passes must carry the URL and alt text.
    assert!(
        md1.contains("image.png") && md1.contains("alt text"),
        "image attrs missing from pass 1; md1: {md1:?}"
    );
    assert!(
        md2.contains("image.png") && md2.contains("alt text"),
        "image attrs missing from pass 2; md2: {md2:?}"
    );
}

#[test]
fn roundtrip_md_to_ir_to_md_html_table_colspan() {
    // A table with a colspan attribute triggers the HTML table path.
    // The original IR representation is built directly so we control the output.
    let rows = vec![ir::TableRow {
        cells: vec![ir::TableCell {
            blocks: vec![ir::Block::Paragraph {
                inlines: vec![plain("wide")],
            }],
            colspan: 2,
            rowspan: 1,
        }],
        is_header: true,
    }];
    let original = make_doc(vec![ir::Block::Table { rows, col_count: 2 }]);

    let md = write_markdown(&original, false);
    assert!(md.contains("<table>"), "HTML table tag missing; md: {md:?}");
    assert!(
        md.contains("colspan=\"2\""),
        "colspan attr missing; md: {md:?}"
    );
    assert!(md.contains("wide"), "cell text missing; md: {md:?}");
}

#[test]
fn roundtrip_empty_document() {
    // A document with no blocks must roundtrip without panicking and produce
    // output that parses back to a document with no meaningful blocks.
    let original = make_doc(vec![]);
    let md = write_markdown(&original, false);
    let parsed = parse_markdown(&md);

    // Either the section is empty or contains only empty paragraphs.
    let non_empty = first_blocks(&parsed).iter().any(|b| match b {
        ir::Block::Paragraph { inlines } => !inlines.is_empty(),
        _ => true,
    });
    // For an empty document, we expect no meaningful content.
    assert!(
        !non_empty,
        "empty document roundtrip produced unexpected blocks; md: {md:?}"
    );
}

#[test]
fn roundtrip_unicode_korean_text() {
    let korean = "안녕하세요 세계";
    let original = make_doc(vec![
        ir::Block::Heading {
            level: 1,
            inlines: vec![plain(korean)],
        },
        ir::Block::Paragraph {
            inlines: vec![plain("한국어 단락입니다.")],
        },
    ]);

    let md = write_markdown(&original, false);
    assert!(
        md.contains(korean),
        "Korean heading text missing; md: {md:?}"
    );
    assert!(
        md.contains("한국어 단락입니다."),
        "Korean paragraph text missing; md: {md:?}"
    );

    let parsed = parse_markdown(&md);

    let heading_text: String = first_blocks(&parsed)
        .iter()
        .filter_map(|b| {
            if let ir::Block::Heading { inlines, .. } = b {
                Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
            } else {
                None
            }
        })
        .collect();
    assert!(
        heading_text.contains(korean),
        "Korean heading lost after roundtrip; heading_text: {heading_text:?}"
    );

    let para_text: String = first_blocks(&parsed)
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
        para_text.contains("한국어"),
        "Korean paragraph text lost; para_text: {para_text:?}"
    );
}

#[test]
fn roundtrip_code_block_two_pass_content_identical() {
    // A multi-line code block with special characters must survive two full
    // write→parse→write cycles with identical code content both times.
    let code = "fn greet(name: &str) {\n    println!(\"Hello, {name}!\");\n    // a < b && c > d\n    let x = 1 * 2 + 3 - 4;\n}";
    let original = make_doc(vec![ir::Block::CodeBlock {
        language: Some("rust".into()),
        code: code.to_string(),
    }]);

    let md1 = write_markdown(&original, false);
    let doc2 = parse_markdown(&md1);
    let md2 = write_markdown(&doc2, false);
    let doc3 = parse_markdown(&md2);

    let extract_code = |doc: &ir::Document| -> Option<String> {
        first_blocks(doc).iter().find_map(|b| {
            if let ir::Block::CodeBlock { code, .. } = b {
                Some(code.clone())
            } else {
                None
            }
        })
    };

    let code2 = extract_code(&doc2).expect("code block missing after pass 1");
    let code3 = extract_code(&doc3).expect("code block missing after pass 2");

    assert_eq!(
        code2.trim(),
        code.trim(),
        "code content changed after pass 1\npass1 md:\n{md1}"
    );
    assert_eq!(
        code3.trim(),
        code.trim(),
        "code content changed after pass 2\npass2 md:\n{md2}"
    );
    assert_eq!(md1, md2, "markdown output not stable across two passes");
}

// -----------------------------------------------------------------------
// B-4: header/footer IR → HWPX → IR roundtrip
// -----------------------------------------------------------------------

#[test]
fn header_footer_ir_to_hwpx_to_ir() {
    use hwp2md::hwpx::{read_hwpx, write_hwpx};

    let header_text = "My Page Header";
    let footer_text = "My Page Footer";
    let body_text = "Body content";

    let mut original = ir::Document::new();
    original.sections.push(ir::Section {
        blocks: vec![ir::Block::Paragraph {
            inlines: vec![plain(body_text)],
        }],
        page_layout: None,
        header: Some(vec![ir::Block::Paragraph {
            inlines: vec![plain(header_text)],
        }]),
        footer: Some(vec![ir::Block::Paragraph {
            inlines: vec![plain(footer_text)],
        }]),
        header_footer_type: None,
    });

    // Write to a temp file and read back.
    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
    write_hwpx(&original, tmp.path(), None).expect("write_hwpx must succeed");
    let parsed = read_hwpx(tmp.path()).expect("read_hwpx must succeed");

    assert!(
        !parsed.sections.is_empty(),
        "parsed document must have sections"
    );

    let section = &parsed.sections[0];

    // Body text preserved.
    let body: String = section
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
        body.contains(body_text),
        "body text not found after HWPX roundtrip; got: {body:?}"
    );

    // Header preserved.
    let header_blocks = section
        .header
        .as_ref()
        .expect("section.header must be Some after HWPX roundtrip");
    let header_text_found: String = header_blocks
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
        header_text_found.contains(header_text),
        "header text not preserved after HWPX roundtrip; got: {header_text_found:?}"
    );

    // Footer preserved.
    let footer_blocks = section
        .footer
        .as_ref()
        .expect("section.footer must be Some after HWPX roundtrip");
    let footer_text_found: String = footer_blocks
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
        footer_text_found.contains(footer_text),
        "footer text not preserved after HWPX roundtrip; got: {footer_text_found:?}"
    );
}

// -----------------------------------------------------------------------
// MD header/footer marker round-trip
// -----------------------------------------------------------------------

#[test]
fn header_footer_md_roundtrip() {
    let header_text = "Page header content";
    let footer_text = "Page footer content";
    let body_text = "Body of the document";

    let mut doc = ir::Document::new();
    doc.sections.push(ir::Section {
        blocks: vec![ir::Block::Paragraph {
            inlines: vec![plain(body_text)],
        }],
        page_layout: None,
        header: Some(vec![ir::Block::Paragraph {
            inlines: vec![plain(header_text)],
        }]),
        footer: Some(vec![ir::Block::Paragraph {
            inlines: vec![plain(footer_text)],
        }]),
        header_footer_type: None,
    });

    // IR → MD
    let md = write_markdown(&doc, false);
    assert!(
        md.contains("<!-- header -->"),
        "header open marker missing; md: {md:?}"
    );
    assert!(
        md.contains("<!-- /header -->"),
        "header close marker missing; md: {md:?}"
    );
    assert!(
        md.contains("<!-- footer -->"),
        "footer open marker missing; md: {md:?}"
    );
    assert!(
        md.contains("<!-- /footer -->"),
        "footer close marker missing; md: {md:?}"
    );

    // MD → IR
    let parsed = parse_markdown(&md);
    let section = parsed
        .sections
        .first()
        .expect("parsed document has no sections");

    // Body content preserved and not contaminated with header/footer text.
    let body: String = section
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
        body.contains(body_text),
        "body text lost after MD roundtrip; body: {body:?}"
    );
    assert!(
        !body.contains(header_text),
        "header text leaked into body; body: {body:?}"
    );
    assert!(
        !body.contains(footer_text),
        "footer text leaked into body; body: {body:?}"
    );

    // Header recovered.
    let header_blocks = section
        .header
        .as_ref()
        .expect("section.header must be Some after MD roundtrip");
    let htext: String = header_blocks
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
        htext.contains(header_text),
        "header text not recovered after MD roundtrip; got: {htext:?}"
    );

    // Footer recovered.
    let footer_blocks = section
        .footer
        .as_ref()
        .expect("section.footer must be Some after MD roundtrip");
    let ftext: String = footer_blocks
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
        ftext.contains(footer_text),
        "footer text not recovered after MD roundtrip; got: {ftext:?}"
    );
}
