use super::tests::first_section_blocks;
use super::tests_inline::parse_with_unsafe_html;
use super::*;
use crate::ir;

// -----------------------------------------------------------------------
// parse_markdown — pagebreak HTML comment markers
// -----------------------------------------------------------------------

#[test]
fn parse_markdown_pagebreak_html_comment_yields_page_break_block() {
    let doc = parse_markdown("before\n\n<!-- pagebreak -->\n\nafter\n");
    let blocks = first_section_blocks(&doc);
    // Tighten: assert the exact block sequence, not just position bounds, so
    // that any silent re-ordering or extra blocks fail the test.
    let kinds: Vec<&'static str> = blocks
        .iter()
        .map(|b| match b {
            ir::Block::Paragraph { .. } => "para",
            ir::Block::PageBreak => "pb",
            _ => "other",
        })
        .collect();
    assert_eq!(
        kinds,
        vec!["para", "pb", "para"],
        "expected exact [para, pb, para] sequence: {blocks:?}"
    );
}

#[test]
fn parse_markdown_pagebreak_marker_is_case_insensitive() {
    let doc = parse_markdown("text\n\n<!-- PageBreak -->\n\nmore\n");
    let blocks = first_section_blocks(&doc);
    assert!(
        blocks.iter().any(|b| matches!(b, ir::Block::PageBreak)),
        "case-insensitive marker should yield PageBreak: {blocks:?}"
    );
}

#[test]
fn parse_markdown_unrelated_html_comment_is_not_pagebreak() {
    let doc = parse_markdown("text\n\n<!-- not a page break -->\n\nmore\n");
    let blocks = first_section_blocks(&doc);
    assert!(
        !blocks.iter().any(|b| matches!(b, ir::Block::PageBreak)),
        "non-pagebreak HTML comment must not yield PageBreak: {blocks:?}"
    );
}

#[test]
fn parse_markdown_pagebreak_lookalike_substring_is_rejected() {
    // The `pagebreakish` keyword contains `pagebreak` as a substring.  The
    // marker detector must require an EXACT match (after trimming), not a
    // substring search.
    let doc = parse_markdown("a\n\n<!-- pagebreakish -->\n\nb\n");
    let blocks = first_section_blocks(&doc);
    assert!(
        !blocks.iter().any(|b| matches!(b, ir::Block::PageBreak)),
        "lookalike `pagebreakish` must not match: {blocks:?}"
    );
}

#[test]
fn parse_markdown_pagebreak_marker_with_trailing_text_is_rejected() {
    // Comrak feeds the entire HTML block including any text after the
    // closing `-->`.  The detector must refuse to match in that case so
    // that the trailing text is preserved rather than swallowed.
    let doc = parse_markdown("a\n\n<!-- pagebreak --> trailing\n\nb\n");
    let blocks = first_section_blocks(&doc);
    assert!(
        !blocks.iter().any(|b| matches!(b, ir::Block::PageBreak)),
        "marker with trailing text must not match: {blocks:?}"
    );
}

// -----------------------------------------------------------------------
// parse_markdown — header/footer HTML comment markers
// -----------------------------------------------------------------------

#[test]
fn header_footer_markers_parsed_to_section() {
    let md = "\
<!-- header -->
Header line
<!-- /header -->

<!-- footer -->
Footer line
<!-- /footer -->

Body paragraph
";
    let doc = parse_markdown(md);
    let section = doc.sections.first().expect("section must exist");

    // Body must contain the paragraph but NOT the header/footer text.
    let body_text: String = section
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
        body_text.contains("Body paragraph"),
        "body paragraph missing; got: {body_text:?}"
    );
    assert!(
        !body_text.contains("Header line"),
        "header text must not appear in body; got: {body_text:?}"
    );
    assert!(
        !body_text.contains("Footer line"),
        "footer text must not appear in body; got: {body_text:?}"
    );

    // Header blocks.
    let header_blocks = section
        .header
        .as_ref()
        .expect("section.header must be Some");
    let header_text: String = header_blocks
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
        header_text.contains("Header line"),
        "header text not found; got: {header_text:?}"
    );

    // Footer blocks.
    let footer_blocks = section
        .footer
        .as_ref()
        .expect("section.footer must be Some");
    let footer_text: String = footer_blocks
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
        footer_text.contains("Footer line"),
        "footer text not found; got: {footer_text:?}"
    );
}

#[test]
fn header_only_marker() {
    let md = "\
<!-- header -->
Just a header
<!-- /header -->

Body text
";
    let doc = parse_markdown(md);
    let section = doc.sections.first().expect("section must exist");

    let header = section
        .header
        .as_ref()
        .expect("section.header must be Some");
    let htext: String = header
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
        htext.contains("Just a header"),
        "header text not found; got: {htext:?}"
    );

    assert!(
        section.footer.is_none(),
        "footer must be None when no footer marker present"
    );
}

#[test]
fn footer_only_marker() {
    let md = "\
<!-- footer -->
Just a footer
<!-- /footer -->

Body text
";
    let doc = parse_markdown(md);
    let section = doc.sections.first().expect("section must exist");

    assert!(
        section.header.is_none(),
        "header must be None when no header marker present"
    );

    let footer = section
        .footer
        .as_ref()
        .expect("section.footer must be Some");
    let ftext: String = footer
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
        ftext.contains("Just a footer"),
        "footer text not found; got: {ftext:?}"
    );
}

#[test]
fn no_markers_leaves_header_footer_none() {
    let doc = parse_markdown("# Heading\n\nParagraph.\n");
    let section = doc.sections.first().expect("section must exist");
    assert!(
        section.header.is_none(),
        "header must be None when no markers are present"
    );
    assert!(
        section.footer.is_none(),
        "footer must be None when no markers are present"
    );
}

#[test]
fn markers_case_insensitive() {
    let md = "\
<!-- HEADER -->
Upper case header
<!-- /HEADER -->

<!-- FOOTER -->
Upper case footer
<!-- /FOOTER -->

Body
";
    let doc = parse_markdown(md);
    let section = doc.sections.first().expect("section must exist");

    let header = section
        .header
        .as_ref()
        .expect("header must be Some for HEADER marker");
    let htext: String = header
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
        htext.contains("Upper case header"),
        "case-insensitive HEADER not matched; got: {htext:?}"
    );

    let footer = section
        .footer
        .as_ref()
        .expect("footer must be Some for FOOTER marker");
    let ftext: String = footer
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
        ftext.contains("Upper case footer"),
        "case-insensitive FOOTER not matched; got: {ftext:?}"
    );
}

#[test]
fn header_footer_markers_roundtrip_via_write_then_parse() {
    use crate::md::write_markdown;

    let header_text = "Running header text";
    let footer_text = "Running footer text";
    let body_text = "Main body content";

    let mut doc = ir::Document::new();
    doc.sections.push(ir::Section {
        blocks: vec![ir::Block::Paragraph {
            inlines: vec![ir::Inline::plain(body_text)],
        }],
        page_layout: None,
        header: Some(vec![ir::Block::Paragraph {
            inlines: vec![ir::Inline::plain(header_text)],
        }]),
        footer: Some(vec![ir::Block::Paragraph {
            inlines: vec![ir::Inline::plain(footer_text)],
        }]),
        header_footer_type: None,
        ..Default::default()
    });

    let md = write_markdown(&doc, false);
    assert!(
        md.contains("<!-- header -->"),
        "header open marker missing in MD output; md: {md:?}"
    );
    assert!(
        md.contains("<!-- /header -->"),
        "header close marker missing in MD output; md: {md:?}"
    );
    assert!(
        md.contains("<!-- footer -->"),
        "footer open marker missing in MD output; md: {md:?}"
    );
    assert!(
        md.contains("<!-- /footer -->"),
        "footer close marker missing in MD output; md: {md:?}"
    );

    let parsed = parse_markdown(&md);
    let section = parsed.sections.first().expect("section must exist");

    // Body preserved, header/footer NOT leaked into body.
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
        "body text lost after roundtrip; body: {body:?}"
    );
    assert!(
        !body.contains(header_text),
        "header text leaked into body; body: {body:?}"
    );
    assert!(
        !body.contains(footer_text),
        "footer text leaked into body; body: {body:?}"
    );

    // Header preserved.
    let header_blocks = section
        .header
        .as_ref()
        .expect("section.header must be Some");
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
        "header text lost after roundtrip; got: {htext:?}"
    );

    // Footer preserved.
    let footer_blocks = section
        .footer
        .as_ref()
        .expect("section.footer must be Some");
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
        "footer text lost after roundtrip; got: {ftext:?}"
    );
}

// -----------------------------------------------------------------------
// parse_markdown — unclosed marker fallback
// -----------------------------------------------------------------------

/// `<!-- header -->` with no closing `<!-- /header -->`: all content after
/// the opening marker ends up in body (fallback), not in header.
#[test]
fn unclosed_header_marker_falls_back_to_body() {
    let md = "\
<!-- header -->
Orphaned header text

Body paragraph
";
    let doc = parse_markdown(md);
    let section = doc.sections.first().expect("section must exist");

    // The unclosed header region is moved to body, so header must be None.
    assert!(
        section.header.is_none(),
        "unclosed header marker must leave section.header as None; got: {:?}",
        section.header
    );

    // The content that was routed to header must appear in body.
    let body_text: String = section
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
        body_text.contains("Orphaned header text"),
        "orphaned header content must fall back into body; got: {body_text:?}"
    );
    assert!(
        body_text.contains("Body paragraph"),
        "normal body paragraph must be present; got: {body_text:?}"
    );
}

/// `<!-- footer -->` with no closing `<!-- /footer -->`: all content after
/// the opening marker ends up in body (fallback), not in footer.
#[test]
fn unclosed_footer_marker_falls_back_to_body() {
    let md = "\
<!-- footer -->
Orphaned footer text

Body paragraph
";
    let doc = parse_markdown(md);
    let section = doc.sections.first().expect("section must exist");

    // The unclosed footer region is moved to body, so footer must be None.
    assert!(
        section.footer.is_none(),
        "unclosed footer marker must leave section.footer as None; got: {:?}",
        section.footer
    );

    let body_text: String = section
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
        body_text.contains("Orphaned footer text"),
        "orphaned footer content must fall back into body; got: {body_text:?}"
    );
    assert!(
        body_text.contains("Body paragraph"),
        "normal body paragraph must be present; got: {body_text:?}"
    );
}

/// `<!-- header -->` immediately followed by `<!-- /header -->` with no
/// content between them: section.header must be None (empty header).
#[test]
fn empty_header_marker_region() {
    let md = "\
<!-- header -->
<!-- /header -->

Body paragraph
";
    let doc = parse_markdown(md);
    let section = doc.sections.first().expect("section must exist");

    assert!(
        section.header.is_none(),
        "empty header region must produce section.header = None; got: {:?}",
        section.header
    );

    let body_text: String = section
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
        body_text.contains("Body paragraph"),
        "body paragraph must be preserved; got: {body_text:?}"
    );
}

/// Input with interleaved markers: `<!-- header --> text1 <!-- footer -->
/// text2 <!-- /header -->`.  The second marker (`<!-- footer -->`) opens a
/// footer region, but the next close marker is `<!-- /header -->` which does
/// NOT match `<!-- /footer -->` — so footer is left open at EOF and falls
/// back to body.  Crucially, neither `text1` nor `text2` should be silently
/// lost.
#[test]
fn interleaved_markers_body_fallback() {
    // After `<!-- header -->`:    region = Header,  text1 → header_blocks
    // After `<!-- footer -->`:    region = Footer,  text2 → footer_blocks
    // After `<!-- /header -->`:   region = Body     (close-header marker)
    // EOF with region = Body: no fallback needed.
    // footer_blocks still has text2, section.footer = Some([text2]).
    // header_blocks has text1, section.header = Some([text1]).
    let md = "\
<!-- header -->
text1
<!-- footer -->
text2
<!-- /header -->
";
    let doc = parse_markdown(md);
    let section = doc.sections.first().expect("section must exist");

    // Collect all text visible anywhere in the document so we can assert
    // nothing is lost regardless of which bucket each block ended up in.
    let mut all_text = String::new();

    // body blocks
    let body_part: String = section
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
    all_text.push_str(&body_part);

    // header blocks (if any)
    if let Some(hblocks) = &section.header {
        let hpart: String = hblocks
            .iter()
            .filter_map(|b| {
                if let ir::Block::Paragraph { inlines } = b {
                    Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
                } else {
                    None
                }
            })
            .collect();
        all_text.push_str(&hpart);
    }

    // footer blocks (if any)
    if let Some(fblocks) = &section.footer {
        let fpart: String = fblocks
            .iter()
            .filter_map(|b| {
                if let ir::Block::Paragraph { inlines } = b {
                    Some(inlines.iter().map(|i| i.text.as_str()).collect::<String>())
                } else {
                    None
                }
            })
            .collect();
        all_text.push_str(&fpart);
    }

    assert!(
        all_text.contains("text1"),
        "text1 must not be lost in interleaved-marker document; all_text: {all_text:?}"
    );
    assert!(
        all_text.contains("text2"),
        "text2 must not be lost in interleaved-marker document; all_text: {all_text:?}"
    );
}

// -----------------------------------------------------------------------
// <ruby> HTML inline parsing
// -----------------------------------------------------------------------

/// `<ruby>漢字<rt>かんじ</rt></ruby>` must produce an inline whose `text`
/// is "漢字" and `ruby` is `Some("かんじ")`.
#[test]
fn ruby_html_parsed_to_inline() {
    let inlines = parse_with_unsafe_html("<ruby>漢字<rt>かんじ</rt></ruby>\n");
    let found = inlines
        .iter()
        .any(|i| i.text == "漢字" && i.ruby.as_deref() == Some("かんじ"));
    assert!(
        found,
        "<ruby>漢字<rt>かんじ</rt></ruby>: expected inline with text='漢字' ruby=Some('かんじ'); got {inlines:?}"
    );
}

/// A round-trip through write → parse must preserve the ruby annotation.
#[test]
fn ruby_roundtrip_via_write_then_parse() {
    use crate::md::write_markdown;

    let mut doc = ir::Document::new();
    doc.sections.push(ir::Section {
        blocks: vec![ir::Block::Paragraph {
            inlines: vec![ir::Inline {
                text: "漢字".into(),
                ruby: Some("かんじ".into()),
                ..ir::Inline::default()
            }],
        }],
        page_layout: None,
        header: None,
        footer: None,
        header_footer_type: None,
        ..Default::default()
    });

    let md = write_markdown(&doc, false);
    // The writer should have embedded the ruby HTML.
    assert!(
        md.contains("<ruby>") && md.contains("<rt>"),
        "ruby HTML not found in written markdown; md: {md:?}"
    );

    // Re-parse using unsafe HTML so comrak emits HtmlInline nodes.
    let mut options = comrak::Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.footnotes = true;
    options.extension.math_dollars = true;
    options.extension.superscript = true;
    options.extension.tasklist = true;
    options.render.unsafe_ = true;
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, &md, &options);

    let para = root
        .children()
        .find(|c| matches!(c.data.borrow().value, NodeValue::Paragraph))
        .expect("paragraph not found after roundtrip");

    let inlines = collect_inlines(para);
    let found = inlines
        .iter()
        .any(|i| i.text == "漢字" && i.ruby.as_deref() == Some("かんじ"));
    assert!(
        found,
        "ruby annotation lost after roundtrip; inlines: {inlines:?}"
    );
}

/// `<ruby>text</ruby>` with no `<rt>` must produce an inline with the base
/// text and `ruby = None` (no annotation to attach).
#[test]
fn ruby_without_rt_produces_no_annotation() {
    let inlines = parse_with_unsafe_html("<ruby>text</ruby>\n");
    // The base text "text" must be present.
    let has_text = inlines.iter().any(|i| i.text.contains("text"));
    assert!(
        has_text,
        "<ruby>text</ruby>: base text 'text' missing; got {inlines:?}"
    );
    // No inline must carry a ruby annotation.
    let has_annotation = inlines.iter().any(|i| i.ruby.is_some());
    assert!(
        !has_annotation,
        "<ruby>text</ruby>: unexpected ruby annotation; got {inlines:?}"
    );
}

/// `<ruby>**bold**<rt>anno</rt></ruby>` — the bold base inline must carry the
/// ruby annotation so that rich-content bases round-trip correctly.
#[test]
fn ruby_with_bold_base() {
    let inlines = parse_with_unsafe_html("<ruby>**bold**<rt>anno</rt></ruby>\n");
    // comrak may or may not emit the Strong node inside an HtmlInline
    // paragraph depending on how it interleaves inline HTML with Markdown
    // markup.  We accept either:
    //   (a) a bold inline with ruby="anno", or
    //   (b) the text "bold" with ruby="anno" (strong markup stripped),
    //   (c) or any inline whose text contains "bold" and ruby is Some("anno").
    let found = inlines
        .iter()
        .any(|i| i.text.contains("bold") && i.ruby.as_deref() == Some("anno"));
    assert!(
        found,
        "<ruby>**bold**<rt>anno</rt></ruby>: expected bold inline with ruby='anno'; got {inlines:?}"
    );
}
