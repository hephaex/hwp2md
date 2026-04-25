use super::*;
use crate::ir;

// -----------------------------------------------------------------------
// Shared helpers (re-declared here for standalone module use)
// -----------------------------------------------------------------------

fn plain(t: &str) -> ir::Inline {
    ir::Inline::plain(t)
}

// -----------------------------------------------------------------------
// render_inlines — basic formatting
// -----------------------------------------------------------------------

#[test]
fn render_inlines_plain() {
    assert_eq!(render_inlines(&[plain("hello")]), "hello");
}

#[test]
fn render_inlines_bold() {
    let inlines = vec![ir::Inline {
        text: "bold".into(),
        bold: true,
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "**bold**");
}

#[test]
fn render_inlines_italic() {
    let inlines = vec![ir::Inline {
        text: "em".into(),
        italic: true,
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "*em*");
}

#[test]
fn render_inlines_bold_italic() {
    let inlines = vec![ir::Inline {
        text: "bi".into(),
        bold: true,
        italic: true,
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "***bi***");
}

#[test]
fn render_inlines_strikethrough() {
    let inlines = vec![ir::Inline {
        text: "del".into(),
        strikethrough: true,
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "~~del~~");
}

#[test]
fn render_inlines_underline() {
    let inlines = vec![ir::Inline {
        text: "ul".into(),
        underline: true,
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "<u>ul</u>");
}

#[test]
fn render_inlines_superscript() {
    let inlines = vec![ir::Inline {
        text: "sup".into(),
        superscript: true,
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "<sup>sup</sup>");
}

#[test]
fn render_inlines_subscript() {
    let inlines = vec![ir::Inline {
        text: "sub".into(),
        subscript: true,
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "<sub>sub</sub>");
}

#[test]
fn render_inlines_code() {
    let inlines = vec![ir::Inline {
        text: "code()".into(),
        code: true,
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "`code()`");
}

#[test]
fn render_inlines_link() {
    let inlines = vec![ir::Inline {
        text: "click".into(),
        link: Some("https://example.com".into()),
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "[click](https://example.com)");
}

#[test]
fn render_inlines_footnote_ref() {
    let inlines = vec![ir::Inline {
        text: String::new(),
        footnote_ref: Some("1".into()),
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "[^1]");
}

// -----------------------------------------------------------------------
// render_inlines — combos
// -----------------------------------------------------------------------

// Issue 1: bold + italic + strikethrough nesting.
// ~~***text***~~ is valid GFM and renders correctly.
#[test]
fn render_inlines_bold_italic_strikethrough() {
    let inlines = vec![ir::Inline {
        text: "all".into(),
        bold: true,
        italic: true,
        strikethrough: true,
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "~~***all***~~");
}

// Issue 2: underline + bold — Markdown bold applied first, HTML <u> wraps.
#[test]
fn render_inlines_underline_bold_order() {
    let inlines = vec![ir::Inline {
        text: "ub".into(),
        bold: true,
        underline: true,
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "<u>**ub**</u>");
}

// Issue 3: link with bold text — bold wraps the label, not the URL.
#[test]
fn render_inlines_bold_link() {
    let inlines = vec![ir::Inline {
        text: "click".into(),
        bold: true,
        link: Some("https://example.com".into()),
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "[**click**](https://example.com)");
}

// Issue 4: footnote_ref appended after formatted text.
#[test]
fn render_inlines_bold_then_footnote_ref() {
    let inlines = vec![ir::Inline {
        text: "note".into(),
        bold: true,
        footnote_ref: Some("fn1".into()),
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "**note**[^fn1]");
}

// Issue 5a: empty text with bold must NOT produce `****`.
#[test]
fn render_inlines_empty_text_bold_skips_markers() {
    let inlines = vec![ir::Inline {
        text: String::new(),
        bold: true,
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "");
}

// Issue 5b: empty text + bold + footnote_ref — formatting skipped,
// footnote reference still emitted.
#[test]
fn render_inlines_empty_text_bold_with_footnote_ref() {
    let inlines = vec![ir::Inline {
        text: String::new(),
        bold: true,
        footnote_ref: Some("2".into()),
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "[^2]");
}

// Issue 5c: empty text with link — [](url) is preserved as-is.
#[test]
fn render_inlines_empty_text_link() {
    let inlines = vec![ir::Inline {
        text: String::new(),
        link: Some("https://example.com".into()),
        ..Default::default()
    }];
    assert_eq!(render_inlines(&inlines), "[](https://example.com)");
}
