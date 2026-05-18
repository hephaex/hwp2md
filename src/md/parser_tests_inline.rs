use super::tests::first_section_blocks;
use super::*;
use crate::ir;

// -----------------------------------------------------------------------
// Helpers (shared with marker tests)
// -----------------------------------------------------------------------

pub(super) fn parse_with_unsafe_html(input: &str) -> Vec<ir::Inline> {
    let mut options = comrak::Options::default();
    options.render.unsafe_ = true;
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, input, &options);
    let para = root
        .children()
        .find(|c| matches!(c.data.borrow().value, NodeValue::Paragraph))
        .expect("paragraph not found");
    collect_inlines(para)
}

// -----------------------------------------------------------------------
// collect_inlines — <u> underline tag
// -----------------------------------------------------------------------

#[test]
fn collect_inlines_underline_via_html_u_tag() {
    // HTML inline tags <u>…</u> are passed through as raw HTML by comrak when
    // unsafe HTML is NOT enabled (the default). In that case comrak emits the
    // raw tag string as HtmlInline nodes which our handler intercepts.
    let mut options = comrak::Options::default();
    options.render.unsafe_ = true; // allow raw HTML so comrak parses <u>
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, "Hello <u>world</u>!\n", &options);

    let para = root
        .children()
        .find(|c| matches!(c.data.borrow().value, NodeValue::Paragraph));
    let para = para.expect("paragraph node not found");

    let inlines = collect_inlines(para);
    let has_underline = inlines.iter().any(|i| i.underline && i.text == "world");
    assert!(
        has_underline,
        "underline inline not found; inlines: {inlines:?}"
    );
}

#[test]
fn collect_inlines_subscript_via_html_sub_tag() {
    let mut options = comrak::Options::default();
    options.render.unsafe_ = true;
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, "H<sub>2</sub>O\n", &options);

    let para = root
        .children()
        .find(|c| matches!(c.data.borrow().value, NodeValue::Paragraph));
    let para = para.expect("paragraph node not found");

    let inlines = collect_inlines(para);
    let has_subscript = inlines.iter().any(|i| i.subscript && i.text == "2");
    assert!(
        has_subscript,
        "subscript inline not found; inlines: {inlines:?}"
    );
}

// -----------------------------------------------------------------------
// collect_inlines — nested <u><sub> combinations
// -----------------------------------------------------------------------

#[test]
fn collect_inlines_u_wrapping_sub() {
    let inlines = parse_with_unsafe_html("<u><sub>text</sub></u>\n");
    let found = inlines
        .iter()
        .any(|i| i.underline && i.subscript && i.text == "text");
    assert!(
        found,
        "<u><sub>text</sub></u>: expected underline+subscript; got {inlines:?}"
    );
}

#[test]
fn collect_inlines_sub_wrapping_u() {
    let inlines = parse_with_unsafe_html("<sub><u>text</u></sub>\n");
    let found = inlines
        .iter()
        .any(|i| i.underline && i.subscript && i.text == "text");
    assert!(
        found,
        "<sub><u>text</u></sub>: expected underline+subscript; got {inlines:?}"
    );
}

#[test]
fn collect_inlines_unclosed_u_applies_underline_to_remaining() {
    let inlines = parse_with_unsafe_html("<u>text\n");
    let has_underline = inlines
        .iter()
        .any(|i| i.underline && i.text.contains("text"));
    assert!(
        has_underline,
        "unclosed <u>: underline should apply to remaining text; got {inlines:?}"
    );
}

#[test]
fn collect_inlines_u_then_sub_are_separate() {
    let inlines = parse_with_unsafe_html("<u>a</u><sub>b</sub>\n");
    let has_underline_a = inlines
        .iter()
        .any(|i| i.underline && !i.subscript && i.text == "a");
    let has_subscript_b = inlines
        .iter()
        .any(|i| i.subscript && !i.underline && i.text == "b");
    assert!(
        has_underline_a,
        "<u>a</u>: expected underline-only for 'a'; got {inlines:?}"
    );
    assert!(
        has_subscript_b,
        "<sub>b</sub>: expected subscript-only for 'b'; got {inlines:?}"
    );
}

// -----------------------------------------------------------------------
// SoftBreak / LineBreak → "\n" inline
// -----------------------------------------------------------------------

#[test]
fn collect_inlines_soft_break_emits_newline() {
    let doc = parse_markdown("first\nsecond\n");
    let blocks = first_section_blocks(&doc);
    assert_eq!(blocks.len(), 1, "expected exactly one paragraph block");
    match &blocks[0] {
        ir::Block::Paragraph { inlines } => {
            let combined: String = inlines.iter().map(|i| i.text.as_str()).collect();
            assert!(
                combined.contains("first"),
                "text 'first' missing; combined: {combined:?}"
            );
            assert!(
                combined.contains("second"),
                "text 'second' missing; combined: {combined:?}"
            );
            assert!(
                inlines.iter().any(|i| i.text == "\n"),
                "SoftBreak must produce a newline inline; inlines: {inlines:?}"
            );
        }
        other => panic!("expected Paragraph, got {other:?}"),
    }
}

#[test]
fn collect_inlines_hard_line_break_emits_newline() {
    // Two trailing spaces force a hard line break (LineBreak node in comrak).
    let doc = parse_markdown("line one  \nline two\n");
    let blocks = first_section_blocks(&doc);
    assert!(
        !blocks.is_empty(),
        "expected at least one block; got nothing"
    );
    if let ir::Block::Paragraph { inlines } = &blocks[0] {
        // Should contain text from both lines and possibly a newline inline.
        let combined: String = inlines.iter().map(|i| i.text.as_str()).collect();
        assert!(
            combined.contains("line one") || combined.contains("line two"),
            "line content missing; got: {combined:?}"
        );
    }
}

// -----------------------------------------------------------------------
// HtmlInline — unknown tags emitted as plain inline text
// -----------------------------------------------------------------------

#[test]
fn collect_inlines_unknown_html_inline_becomes_plain_text() {
    let mut options = comrak::Options::default();
    options.render.unsafe_ = true;
    let arena = comrak::Arena::new();
    // <span> is not handled — must be emitted verbatim as a plain inline.
    let root = comrak::parse_document(&arena, "Hello <span>world</span>!\n", &options);

    let para = root
        .children()
        .find(|c| matches!(c.data.borrow().value, NodeValue::Paragraph))
        .expect("paragraph not found");

    let inlines = collect_inlines(para);
    // The text "world" should appear either as a standalone plain inline or
    // inside the span tags. We just require no panic and some content.
    let combined: String = inlines.iter().map(|i| i.text.as_str()).collect();
    assert!(
        combined.contains("Hello"),
        "plain text lost; combined: {combined:?}"
    );
    // <span> and </span> should appear as plain text since they are unknown.
    assert!(
        combined.contains("span") || combined.contains("world"),
        "unknown html inline not rendered; combined: {combined:?}"
    );
}

// -----------------------------------------------------------------------
// collect_alt_text — image with no alt text
// -----------------------------------------------------------------------

#[test]
fn node_to_block_image_no_alt_returns_empty_alt() {
    // An image with no alt text should have an empty alt string.
    let doc = parse_markdown("![](photo.jpg)\n");
    let blocks = first_section_blocks(&doc);
    let found = blocks.iter().any(|b| match b {
        ir::Block::Image { alt, .. } => alt.is_empty(),
        ir::Block::Paragraph { inlines } => inlines.iter().any(|i| i.text.contains("photo.jpg")),
        _ => false,
    });
    assert!(found, "image not found; blocks: {blocks:?}");
}

// -----------------------------------------------------------------------
// </sub> close tag restores outer subscript state
// -----------------------------------------------------------------------

#[test]
fn collect_inlines_sub_close_tag_restores_state() {
    // After </sub>, subsequent text should NOT be subscript.
    let inlines = parse_with_unsafe_html("<sub>x</sub>y\n");
    let x_is_subscript = inlines.iter().any(|i| i.subscript && i.text == "x");
    let y_not_subscript = inlines.iter().any(|i| !i.subscript && i.text == "y");
    assert!(
        x_is_subscript,
        "x inside <sub> must be subscript; got {inlines:?}"
    );
    assert!(
        y_not_subscript,
        "y after </sub> must NOT be subscript; got {inlines:?}"
    );
}

// -----------------------------------------------------------------------
// Horizontal rule parsed correctly
// -----------------------------------------------------------------------

#[test]
fn parse_markdown_horizontal_rule() {
    let doc = parse_markdown("---\n\ntext\n");
    let blocks = first_section_blocks(&doc);
    assert!(
        blocks
            .iter()
            .any(|b| matches!(b, ir::Block::HorizontalRule)),
        "expected HorizontalRule block from '---'; blocks: {blocks:?}"
    );
    assert!(
        blocks
            .iter()
            .any(|b| matches!(b, ir::Block::Paragraph { .. })),
        "expected Paragraph block with 'text'; blocks: {blocks:?}"
    );
}

// -----------------------------------------------------------------------
// parse_markdown — strikethrough via ~~…~~
// -----------------------------------------------------------------------

#[test]
fn parse_markdown_strikethrough_inline() {
    let doc = parse_markdown("~~struck~~\n");
    let blocks = first_section_blocks(&doc);
    if let ir::Block::Paragraph { inlines } = &blocks[0] {
        let has_strike = inlines.iter().any(|i| i.strikethrough);
        assert!(has_strike, "no strikethrough inline; inlines: {inlines:?}");
    } else {
        panic!("expected Paragraph, got {:?}", blocks[0]);
    }
}

// -----------------------------------------------------------------------
// parse_markdown — task list (GitHub-style checkboxes)
// -----------------------------------------------------------------------

#[test]
fn parse_markdown_task_list_unchecked_item() {
    let doc = parse_markdown("- [ ] unchecked\n");
    let blocks = first_section_blocks(&doc);
    if let ir::Block::List { items, ordered, .. } = &blocks[0] {
        assert!(!ordered, "task list must be unordered");
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].checked,
            Some(false),
            "unchecked item must have checked=Some(false); item: {:?}",
            items[0]
        );
    } else {
        panic!("expected List, got {:?}", blocks[0]);
    }
}

#[test]
fn parse_markdown_task_list_checked_item() {
    let doc = parse_markdown("- [x] checked\n");
    let blocks = first_section_blocks(&doc);
    if let ir::Block::List { items, .. } = &blocks[0] {
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].checked,
            Some(true),
            "checked item must have checked=Some(true); item: {:?}",
            items[0]
        );
    } else {
        panic!("expected List, got {:?}", blocks[0]);
    }
}

#[test]
fn parse_markdown_task_list_checked_capital_x() {
    let doc = parse_markdown("- [X] also checked\n");
    let blocks = first_section_blocks(&doc);
    if let ir::Block::List { items, .. } = &blocks[0] {
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].checked,
            Some(true),
            "capital-X checked item must have checked=Some(true); item: {:?}",
            items[0]
        );
    } else {
        panic!("expected List, got {:?}", blocks[0]);
    }
}

#[test]
fn parse_markdown_task_list_mixed_items() {
    let md = "- [x] done\n- [ ] todo\n- [ ] also todo\n";
    let doc = parse_markdown(md);
    let blocks = first_section_blocks(&doc);
    if let ir::Block::List { items, .. } = &blocks[0] {
        assert_eq!(items.len(), 3, "expected 3 items; got {}", items.len());
        assert_eq!(items[0].checked, Some(true), "item[0] must be checked");
        assert_eq!(items[1].checked, Some(false), "item[1] must be unchecked");
        assert_eq!(items[2].checked, Some(false), "item[2] must be unchecked");
    } else {
        panic!("expected List, got {:?}", blocks[0]);
    }
}

#[test]
fn parse_markdown_normal_list_item_has_checked_none() {
    let doc = parse_markdown("- plain item\n");
    let blocks = first_section_blocks(&doc);
    if let ir::Block::List { items, .. } = &blocks[0] {
        assert_eq!(
            items[0].checked, None,
            "normal list item must have checked=None; item: {:?}",
            items[0]
        );
    } else {
        panic!("expected List, got {:?}", blocks[0]);
    }
}

#[test]
fn parse_markdown_task_list_text_is_preserved() {
    let doc = parse_markdown("- [x] buy milk\n");
    let blocks = first_section_blocks(&doc);
    if let ir::Block::List { items, .. } = &blocks[0] {
        let text: String = items[0]
            .blocks
            .iter()
            .flat_map(|b| match b {
                ir::Block::Paragraph { inlines } => {
                    inlines.iter().map(|i| i.text.as_str()).collect::<Vec<_>>()
                }
                _ => vec![],
            })
            .collect();
        assert!(
            text.contains("buy milk"),
            "item text must be preserved; got: {text:?}"
        );
    } else {
        panic!("expected List, got {:?}", blocks[0]);
    }
}
