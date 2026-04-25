use super::*;
use crate::ir;

// -----------------------------------------------------------------------
// Shared helpers (re-declared here for standalone module use)
// -----------------------------------------------------------------------

fn plain(t: &str) -> ir::Inline {
    ir::Inline::plain(t)
}

// -----------------------------------------------------------------------
// render_inlines — link URL security (S1)
// -----------------------------------------------------------------------

// javascript: URL must be stripped — only the label text is emitted.
#[test]
fn render_inlines_javascript_link_emits_text_only() {
    let inlines = vec![ir::Inline {
        text: "click me".into(),
        link: Some("javascript:alert(1)".into()),
        ..Default::default()
    }];
    let out = render_inlines(&inlines);
    assert!(
        !out.contains("javascript:"),
        "javascript: scheme must be dropped; got: {out:?}"
    );
    assert!(
        out.contains("click me"),
        "label text must still be emitted; got: {out:?}"
    );
    assert!(
        !out.contains("]("),
        "link syntax must not be present; got: {out:?}"
    );
}

// data: URL must also be blocked.
#[test]
fn render_inlines_data_url_emits_text_only() {
    let inlines = vec![ir::Inline {
        text: "img".into(),
        link: Some("data:text/html,<script>alert(1)</script>".into()),
        ..Default::default()
    }];
    let out = render_inlines(&inlines);
    assert!(
        !out.contains("data:"),
        "data: scheme must be dropped; got: {out:?}"
    );
}

// Safe schemes must still produce link syntax.
#[test]
fn render_inlines_https_link_emitted() {
    let inlines = vec![ir::Inline {
        text: "safe".into(),
        link: Some("https://example.com".into()),
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "[safe](https://example.com)");
}

#[test]
fn render_inlines_mailto_link_emitted() {
    let inlines = vec![ir::Inline {
        text: "email".into(),
        link: Some("mailto:user@example.com".into()),
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "[email](mailto:user@example.com)");
}

// Case-insensitive: JAVASCRIPT: must also be rejected.
#[test]
fn render_inlines_javascript_link_case_insensitive() {
    let inlines = vec![ir::Inline {
        text: "xss".into(),
        link: Some("JAVASCRIPT:alert(1)".into()),
        ..Default::default()
    }];
    let out = render_inlines(&inlines);
    assert!(
        !out.contains("JAVASCRIPT:"),
        "uppercase JAVASCRIPT: must be dropped; got: {out:?}"
    );
}

// URL containing ')' must use angle-bracket syntax to avoid breaking link parsing.
#[test]
fn render_inlines_link_with_paren_uses_angle_bracket_syntax() {
    let inlines = vec![ir::Inline {
        text: "click".into(),
        link: Some("https://example.com/foo(bar)".into()),
        ..Default::default()
    }];
    assert_eq!(
        render_inlines(&inlines),
        "[click](<https://example.com/foo(bar)>)"
    );
}

// URL without ')' must still use plain parenthesis syntax.
#[test]
fn render_inlines_link_without_paren_uses_plain_syntax() {
    let inlines = vec![ir::Inline {
        text: "click".into(),
        link: Some("https://example.com/plain".into()),
        ..Default::default()
    }];
    assert_eq!(
        render_inlines(&inlines),
        "[click](https://example.com/plain)"
    );
}

// -----------------------------------------------------------------------
// render_inlines — inline text escaping
// -----------------------------------------------------------------------

// Markdown metacharacters in inline text must be escaped.
#[test]
fn render_inlines_escapes_asterisk() {
    assert_eq!(render_inlines(&[plain("a*b")]), r"a\*b");
}

#[test]
fn render_inlines_escapes_underscore() {
    assert_eq!(render_inlines(&[plain("a_b")]), r"a\_b");
}

#[test]
fn render_inlines_escapes_tilde() {
    assert_eq!(render_inlines(&[plain("a~b")]), r"a\~b");
}

#[test]
fn render_inlines_escapes_brackets() {
    assert_eq!(render_inlines(&[plain("[link]")]), r"\[link\]");
}

#[test]
fn render_inlines_escapes_backtick() {
    assert_eq!(render_inlines(&[plain("a`b")]), "a\\`b");
}

#[test]
fn render_inlines_escapes_backslash() {
    assert_eq!(render_inlines(&[plain(r"a\b")]), r"a\\b");
}

// Code spans bypass escaping — raw text is wrapped in backticks.
#[test]
fn render_inlines_code_no_escape() {
    let inlines = vec![ir::Inline {
        text: "a*b_c".into(),
        code: true,
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "`a*b_c`");
}

// -----------------------------------------------------------------------
// escape_inline unit tests
// -----------------------------------------------------------------------

#[test]
fn escape_inline_no_special_chars() {
    assert_eq!(escape_inline("hello world"), "hello world");
}

#[test]
fn escape_inline_all_special_chars() {
    // Input:  \  `  *  _  ~  [  ]
    // Output: \\ \` \* \_ \~ \[ \]
    let input = "\\`*_~[]";
    let expected = "\\\\\\`\\*\\_\\~\\[\\]";
    assert_eq!(escape_inline(input), expected);
}

#[test]
fn escape_inline_mixed() {
    // Asterisks in plain text must be escaped.
    assert_eq!(escape_inline("price: *$5*"), r"price: \*$5\*");
}

// -----------------------------------------------------------------------
// render_inlines — color field
// -----------------------------------------------------------------------

#[test]
fn render_inlines_color_wraps_in_span() {
    let inlines = vec![ir::Inline {
        text: "red".into(),
        color: Some("#FF0000".into()),
        ..Default::default()
    }];
    assert_eq!(
        render_inlines(&inlines),
        "<span style=\"color:#FF0000\">red</span>"
    );
}

#[test]
fn render_inlines_no_color_no_span() {
    let inlines = vec![ir::Inline {
        text: "plain".into(),
        color: None,
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "plain");
}

#[test]
fn render_inlines_color_with_bold_span_wraps_bold() {
    // Bold decoration is applied first; the span wraps the fully-decorated text.
    let inlines = vec![ir::Inline {
        text: "bold red".into(),
        bold: true,
        color: Some("#FF0000".into()),
        ..Default::default()
    }];
    assert_eq!(
        render_inlines(&inlines),
        "<span style=\"color:#FF0000\">**bold red**</span>"
    );
}

#[test]
fn render_inlines_empty_color_string_no_span() {
    // An empty string in color must not produce a <span>.
    let inlines = vec![ir::Inline {
        text: "text".into(),
        color: Some(String::new()),
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "text");
}

#[test]
fn render_inlines_color_applied_before_link() {
    // Color span wraps the label; the outer [label](url) form is applied after.
    let inlines = vec![ir::Inline {
        text: "click".into(),
        color: Some("#0000FF".into()),
        link: Some("https://example.com".into()),
        ..Default::default()
    }];
    let out = render_inlines(&inlines);
    assert_eq!(
        out,
        "[<span style=\"color:#0000FF\">click</span>](https://example.com)"
    );
}

// -----------------------------------------------------------------------
// render_inlines — ruby annotation field
// -----------------------------------------------------------------------

#[test]
fn render_inlines_ruby_annotation_produces_html_tags() {
    let inlines = vec![ir::Inline {
        text: "漢字".into(),
        ruby: Some("한자".into()),
        ..ir::Inline::default()
    }];
    assert_eq!(render_inlines(&inlines), "<ruby>漢字<rt>한자</rt></ruby>");
}

#[test]
fn render_inlines_ruby_annotation_none_no_ruby_tags() {
    let inlines = vec![ir::Inline {
        text: "漢字".into(),
        ruby: None,
        ..ir::Inline::default()
    }];
    let out = render_inlines(&inlines);
    assert!(
        !out.contains("<ruby>"),
        "no ruby tags when annotation is None; got: {out}"
    );
    assert!(
        out.contains("漢字"),
        "base text must still appear; got: {out}"
    );
}

#[test]
fn render_inlines_ruby_empty_annotation_no_ruby_tags() {
    let inlines = vec![ir::Inline {
        text: "漢字".into(),
        ruby: Some(String::new()),
        ..ir::Inline::default()
    }];
    let out = render_inlines(&inlines);
    assert!(
        !out.contains("<ruby>"),
        "empty annotation must not emit ruby tags; got: {out}"
    );
}

#[test]
fn render_inlines_ruby_annotation_html_special_chars_escaped() {
    let inlines = vec![ir::Inline {
        text: "base".into(),
        ruby: Some("a<b>&c".into()),
        ..ir::Inline::default()
    }];
    let out = render_inlines(&inlines);
    assert!(out.contains("&lt;"), "< must be escaped; got: {out}");
    assert!(out.contains("&gt;"), "> must be escaped; got: {out}");
    assert!(out.contains("&amp;"), "& must be escaped; got: {out}");
    assert!(
        !out.contains("<b>"),
        "raw < must not appear unescaped; got: {out}"
    );
}

#[test]
fn render_inlines_ruby_with_bold_base_text() {
    let inlines = vec![ir::Inline {
        text: "漢字".into(),
        bold: true,
        ruby: Some("한자".into()),
        ..ir::Inline::default()
    }];
    let out = render_inlines(&inlines);
    assert!(
        out.contains("<ruby>**漢字**<rt>한자</rt></ruby>"),
        "got: {out}"
    );
}

#[test]
fn write_markdown_paragraph_with_ruby_inline() {
    let mut doc = ir::Document::new();
    doc.sections.push(ir::Section {
        blocks: vec![ir::Block::Paragraph {
            inlines: vec![ir::Inline {
                text: "漢字".into(),
                ruby: Some("한자".into()),
                ..ir::Inline::default()
            }],
        }],
    });
    let md = write_markdown(&doc, false);
    assert!(
        md.contains("<ruby>漢字<rt>한자</rt></ruby>"),
        "paragraph must contain ruby HTML; got: {md}"
    );
}
