/// Phase C-3: Code block language preservation tests.
///
/// Verifies that the language hint stored on `Block::CodeBlock.language` is:
/// 1. Emitted as an XML comment (`<!-- hwp2md:lang:LANG -->`) in section XML.
/// 2. Parsed back by the reader into the `language` field on roundtrip.
/// 3. Rendered with the correct fence in Markdown output.
use super::*;

// ── writer: XML comment emission ──────────────────────────────────────────

/// The writer must emit `<!-- hwp2md:lang:python -->` before the code
/// paragraph when language is `Some("python")`.
#[test]
fn section_xml_code_block_with_language_emits_lang_comment() {
    let xml = section_xml(vec![Block::CodeBlock {
        language: Some("python".into()),
        // Use code without characters that XML-escapes change, to avoid
        // testing XML escaping behaviour here.
        code: "x = 42".into(),
    }]);
    assert!(
        xml.contains("<!--"),
        "section XML must contain an XML comment for language hint: {xml}"
    );
    assert!(
        xml.contains("hwp2md:lang:python"),
        "section XML must contain the language name in the comment: {xml}"
    );
    assert!(
        xml.contains("x = 42"),
        "code content must still appear in the output: {xml}"
    );
}

/// With no language hint, the writer must still emit the sentinel comment
/// `<!-- hwp2md:lang: -->` (empty language) so the reader knows this is a
/// code block rather than a regular paragraph.
#[test]
fn section_xml_code_block_without_language_emits_empty_lang_comment() {
    let xml = section_xml(vec![Block::CodeBlock {
        language: None,
        code: "no lang code".into(),
    }]);
    assert!(
        xml.contains("hwp2md:lang:"),
        "section XML must contain hwp2md:lang: sentinel for code block: {xml}"
    );
    // The language part after the colon must be empty (just whitespace).
    assert!(
        xml.contains("hwp2md:lang: "),
        "empty-language sentinel must end with a space before -->: {xml}"
    );
    assert!(
        xml.contains("no lang code"),
        "code content must still appear: {xml}"
    );
}

/// Language hint comment must appear **before** the `<hp:p>` element, not
/// inside it, so the XML remains well-formed.
#[test]
fn section_xml_lang_comment_precedes_paragraph() {
    let xml = section_xml(vec![Block::CodeBlock {
        language: Some("rust".into()),
        code: "fn main() {}".into(),
    }]);
    let comment_pos = xml
        .find("hwp2md:lang:rust")
        .expect("lang comment must exist");
    let p_pos = xml[comment_pos..]
        .find("<hp:p ")
        .map(|off| comment_pos + off)
        .expect("<hp:p must follow the comment");
    assert!(
        comment_pos < p_pos,
        "language comment must appear before <hp:p: {xml}"
    );
}

/// The language comment must not appear for non-code blocks.
#[test]
fn section_xml_lang_comment_absent_for_paragraph() {
    let xml = section_xml(vec![Block::Paragraph {
        inlines: vec![inline("just text")],
    }]);
    assert!(
        !xml.contains("hwp2md:lang:"),
        "hwp2md:lang: comment must NOT appear for a plain paragraph: {xml}"
    );
}

// ── reader: language hint comment parsing ─────────────────────────────────

/// Given section XML that contains the language-hint comment before a code
/// paragraph, the reader must reconstruct a `Block::CodeBlock` with the
/// correct `language`.
#[test]
fn reader_parses_lang_comment_into_code_block_language() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
        xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
  <!-- hwp2md:lang:rust -->
  <hp:p id="0" paraPrIDRef="0">
    <hp:run charPrIDRef="2">
      <hp:t>fn hello() {}</hp:t>
    </hp:run>
  </hp:p>
</hs:sec>"#;

    let section = read_hwpx_section_xml(xml);
    assert_eq!(
        section.blocks.len(),
        1,
        "expected 1 block: {:?}",
        section.blocks
    );
    match &section.blocks[0] {
        Block::CodeBlock { language, code } => {
            assert_eq!(
                language.as_deref(),
                Some("rust"),
                "language must be 'rust': {:?}",
                section.blocks
            );
            assert_eq!(code, "fn hello() {}", "code content must be preserved");
        }
        other => panic!("expected CodeBlock, got {other:?}"),
    }
}

/// An empty `<!-- hwp2md:lang: -->` comment must produce a `CodeBlock` with
/// `language: None`.
#[test]
fn reader_parses_empty_lang_comment_into_code_block_no_language() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
        xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
  <!-- hwp2md:lang: -->
  <hp:p id="0" paraPrIDRef="0">
    <hp:run charPrIDRef="2">
      <hp:t>no language here</hp:t>
    </hp:run>
  </hp:p>
</hs:sec>"#;

    let section = read_hwpx_section_xml(xml);
    assert_eq!(
        section.blocks.len(),
        1,
        "expected 1 block: {:?}",
        section.blocks
    );
    match &section.blocks[0] {
        Block::CodeBlock { language, code } => {
            assert!(
                language.is_none(),
                "language must be None for empty hint: {:?}",
                section.blocks
            );
            assert_eq!(code, "no language here", "code content must be preserved");
        }
        other => panic!("expected CodeBlock, got {other:?}"),
    }
}

/// A comment that does NOT start with `hwp2md:lang:` must not affect parsing.
#[test]
fn reader_ignores_unrelated_xml_comments() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section"
        xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph">
  <!-- some unrelated comment -->
  <hp:p id="0" paraPrIDRef="0">
    <hp:run charPrIDRef="0">
      <hp:t>normal text</hp:t>
    </hp:run>
  </hp:p>
</hs:sec>"#;

    let section = read_hwpx_section_xml(xml);
    assert_eq!(
        section.blocks.len(),
        1,
        "expected 1 block: {:?}",
        section.blocks
    );
    assert!(
        matches!(&section.blocks[0], Block::Paragraph { .. }),
        "unrelated comment must not turn paragraph into code block: {:?}",
        section.blocks
    );
}

// ── roundtrip: MD language → HWPX → MD ───────────────────────────────────

/// Full HWPX roundtrip: a `CodeBlock` with `language = Some("python")` must
/// survive write-then-read with the language preserved.
#[test]
fn roundtrip_code_block_with_language_preserved() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::CodeBlock {
                language: Some("python".into()),
                code: "x = 1\nprint(x)\n".into(),
            }],
        }],
        assets: Vec::new(),
    };
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");
    let read_back = read_hwpx(tmp.path()).expect("read_hwpx");

    let code_block = read_back
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find(|b| matches!(b, Block::CodeBlock { .. }))
        .expect("CodeBlock must survive HWPX roundtrip");

    match code_block {
        Block::CodeBlock { language, code } => {
            assert_eq!(
                language.as_deref(),
                Some("python"),
                "language must be 'python' after roundtrip"
            );
            assert_eq!(code, "x = 1\nprint(x)\n", "code content must be preserved");
        }
        _ => unreachable!(),
    }
}

/// Full HWPX roundtrip: a `CodeBlock` with no language must survive with
/// `language` remaining `None`.
#[test]
fn roundtrip_code_block_no_language_preserved() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::CodeBlock {
                language: None,
                code: "plain code".into(),
            }],
        }],
        assets: Vec::new(),
    };
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");
    let read_back = read_hwpx(tmp.path()).expect("read_hwpx");

    let code_block = read_back
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find(|b| matches!(b, Block::CodeBlock { .. }))
        .expect("CodeBlock must survive HWPX roundtrip");

    match code_block {
        Block::CodeBlock { language, code } => {
            assert!(
                language.is_none(),
                "language must remain None after roundtrip: {language:?}"
            );
            assert_eq!(code, "plain code", "code content must be preserved");
        }
        _ => unreachable!(),
    }
}

/// Edge case: unusual language names with special characters must roundtrip
/// without corruption.  Tested names: "c++", "objective-c", "shell".
#[test]
fn roundtrip_code_block_unusual_language_names() {
    for lang in &["c++", "objective-c", "shell"] {
        let tmp = tempfile::NamedTempFile::new().expect("tmp file");
        let doc = Document {
            metadata: Metadata::default(),
            sections: vec![Section {
                blocks: vec![Block::CodeBlock {
                    language: Some((*lang).to_string()),
                    code: format!("// {lang} code"),
                }],
            }],
            assets: Vec::new(),
        };
        write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");
        let read_back = read_hwpx(tmp.path()).expect("read_hwpx");

        let code_block = read_back
            .sections
            .iter()
            .flat_map(|s| &s.blocks)
            .find(|b| matches!(b, Block::CodeBlock { .. }))
            .unwrap_or_else(|| panic!("CodeBlock must survive roundtrip for language '{lang}'"));

        match code_block {
            Block::CodeBlock {
                language: found_lang,
                ..
            } => {
                assert_eq!(
                    found_lang.as_deref(),
                    Some(*lang),
                    "language '{lang}' must survive roundtrip"
                );
            }
            _ => unreachable!(),
        }
    }
}

/// A document with a code block followed by a normal paragraph must have both
/// blocks correctly reconstructed after roundtrip.
#[test]
fn roundtrip_code_block_followed_by_paragraph() {
    let tmp = tempfile::NamedTempFile::new().expect("tmp file");
    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![
                Block::CodeBlock {
                    language: Some("bash".into()),
                    code: "echo hello".into(),
                },
                Block::Paragraph {
                    inlines: vec![inline("after code")],
                },
            ],
        }],
        assets: Vec::new(),
    };
    write_hwpx(&doc, tmp.path(), None).expect("write_hwpx");
    let read_back = read_hwpx(tmp.path()).expect("read_hwpx");

    let blocks: Vec<_> = read_back.sections.iter().flat_map(|s| &s.blocks).collect();

    let has_code = blocks.iter().any(
        |b| matches!(b, Block::CodeBlock { language, .. } if language.as_deref() == Some("bash")),
    );
    let has_para = blocks
        .iter()
        .any(|b| matches!(b, Block::Paragraph { inlines } if inlines.iter().any(|i| i.text == "after code")));

    assert!(has_code, "code block must survive roundtrip: {blocks:?}");
    assert!(has_para, "paragraph must survive roundtrip: {blocks:?}");
}

// ── MD writer: language in fence ──────────────────────────────────────────

/// The Markdown writer must emit ` ```python ` (with language) when
/// `language` is `Some("python")`.
#[test]
fn md_writer_code_block_with_language_emits_fence_with_lang() {
    use crate::md::write_markdown;

    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::CodeBlock {
                language: Some("python".into()),
                code: "x = 42\n".into(),
            }],
        }],
        assets: Vec::new(),
    };

    let md = write_markdown(&doc, false);
    assert!(
        md.contains("```python"),
        "Markdown must contain ```python fence: {md}"
    );
    assert!(md.contains("x = 42"), "code content must appear: {md}");
}

/// The Markdown writer must emit a plain ` ``` ` fence (no language label)
/// when `language` is `None`.
#[test]
fn md_writer_code_block_no_language_emits_plain_fence() {
    use crate::md::write_markdown;

    let doc = Document {
        metadata: Metadata::default(),
        sections: vec![Section {
            blocks: vec![Block::CodeBlock {
                language: None,
                code: "no lang\n".into(),
            }],
        }],
        assets: Vec::new(),
    };

    let md = write_markdown(&doc, false);
    // The opening fence must be exactly ``` with no language tag.
    assert!(
        md.contains("```\n"),
        "Markdown must have plain ``` fence when language is None: {md}"
    );
    assert!(md.contains("no lang"), "code content must appear: {md}");
}
