/// Integration tests for HWPX block-level elements: equations, horizontal rules,
/// and block quotes.
///
/// The equation tests cover the HWPX `<hp:equation>` handler (verbatim storage
/// design contract — does NOT call eqedit_to_latex).  The HR and BlockQuote
/// tests drive the IR directly since those blocks originate from the Markdown
/// parser rather than HWPX.
///
/// Extracted from integration.rs (Sprints 86-87) to keep each test file focused.
#[path = "fixtures/mod.rs"]
#[allow(dead_code)]
mod fixtures;

use fixtures::{read_fixture, HwpxFixture};
use hwp2md::{ir, md};

// ---------------------------------------------------------------------------
// Sprint 86 P3: HWPX equation element → Math block → LaTeX Markdown
// ---------------------------------------------------------------------------

/// `<hp:equation>` with EQEDIT content produces `ir::Block::Math { display: true }`
/// and renders as a display-math `$$..$$` fence in Markdown.
#[test]
fn hwpx_equation_element_produces_math_block_and_latex_markdown() {
    // NOTE: The HWPX <hp:equation> handler (handlers.rs) stores the raw
    // element text directly as `tex` — it does NOT call `eqedit_to_latex`.
    // That function is only wired into the HWP 5.0 binary reader path.
    // So `tex` is exactly the literal XML text content ("x + y" here).
    let eq_xml = r#"<hp:equation>x + y</hp:equation>"#;

    let (_dir, doc) = read_fixture(HwpxFixture::new().section(eq_xml));

    // IR layer: must contain a Math block with display=true.
    let math_block = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find(|b| matches!(b, ir::Block::Math { .. }));

    assert!(
        math_block.is_some(),
        "expected Block::Math from <hp:equation>; blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    let ir::Block::Math { display, tex } = math_block.unwrap() else {
        unreachable!()
    };
    assert!(*display, "HWPX equation must produce display=true Math block");
    assert!(
        tex.contains("x") && tex.contains("y"),
        "LaTeX output must contain 'x' and 'y'; got: {tex:?}"
    );

    // Markdown layer: display math → $$\n..\n$$ format.
    let markdown = md::write_markdown(&doc, false);
    assert!(
        markdown.contains("$$"),
        "markdown must contain $$ fence for display math; got: {markdown:?}"
    );
    assert!(
        markdown.contains("x") && markdown.contains("y"),
        "math content must appear in markdown; got: {markdown:?}"
    );
}

// ---------------------------------------------------------------------------
// Sprint 86 P4: HorizontalRule and BlockQuote IR → Markdown rendering
// ---------------------------------------------------------------------------
// These blocks originate from the Markdown parser (---/> syntax), not from HWPX.
// Tested here via direct IR → Markdown to pin the writer rendering contract.

/// `ir::Block::HorizontalRule` must render as `---` (thematic break) in Markdown.
#[test]
fn ir_horizontal_rule_renders_as_thematic_break_in_markdown() {
    let doc = ir::Document {
        metadata: ir::Metadata::default(),
        sections: vec![ir::Section {
            blocks: vec![
                ir::Block::Paragraph {
                    inlines: vec![ir::Inline::plain("before".to_string())],
                },
                ir::Block::HorizontalRule,
                ir::Block::Paragraph {
                    inlines: vec![ir::Inline::plain("after".to_string())],
                },
            ],
            ..Default::default()
        }],
        assets: Vec::new(),
    };

    let markdown = md::write_markdown(&doc, false);

    assert!(
        markdown.contains("---"),
        "HorizontalRule must render as '---'; got: {markdown:?}"
    );

    // Ordering: before < HR < after.
    let pos_before = markdown.find("before").expect("'before' in markdown");
    let pos_hr = markdown.find("---").expect("'---' in markdown");
    let pos_after = markdown.find("after").expect("'after' in markdown");
    assert!(
        pos_before < pos_hr && pos_hr < pos_after,
        "order must be before · --- · after; positions: before={pos_before} hr={pos_hr} after={pos_after}"
    );
}

/// `ir::Block::BlockQuote` must render with `> ` prefix in Markdown.
#[test]
fn ir_block_quote_renders_as_quoted_text_in_markdown() {
    let doc = ir::Document {
        metadata: ir::Metadata::default(),
        sections: vec![ir::Section {
            blocks: vec![ir::Block::BlockQuote {
                blocks: vec![ir::Block::Paragraph {
                    inlines: vec![ir::Inline::plain("Quoted text.".to_string())],
                }],
            }],
            ..Default::default()
        }],
        assets: Vec::new(),
    };

    let markdown = md::write_markdown(&doc, false);

    // The writer emits "> " + content on the same line.
    assert!(
        markdown.contains("> Quoted text."),
        "BlockQuote must render as '> Quoted text.' line; got: {markdown:?}"
    );
}

// ---------------------------------------------------------------------------
// Sprint 87 P3: HWPX equation verbatim storage design documentation test
// ---------------------------------------------------------------------------

/// Documents that <hp:equation> content is stored verbatim (not passed through
/// eqedit_to_latex). This is a design contract test: if the behavior changes,
/// this test must be reviewed along with any EQEDIT-specific syntax in existing
/// HWPX files. See comment in handlers.rs for rationale.
#[test]
fn hwpx_equation_eqedit_syntax_stored_verbatim_not_converted() {
    // "a over b" is EQEDIT syntax for a fraction. eqedit_to_latex would convert
    // this to "\frac{a}{b}". In HWPX, it is stored verbatim.
    let eq_xml = r#"<hp:equation>a over b</hp:equation>"#;
    let (_dir, doc) = read_fixture(HwpxFixture::new().section(eq_xml));

    let math_block = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .find(|b| matches!(b, ir::Block::Math { .. }));

    assert!(
        math_block.is_some(),
        "expected Math block; blocks: {:?}",
        doc.sections.iter().flat_map(|s| &s.blocks).collect::<Vec<_>>()
    );
    let ir::Block::Math { tex, .. } = math_block.unwrap() else {
        unreachable!()
    };
    // HWPX verbatim: stored as "a over b", NOT converted to "\frac{a}{b}".
    assert_eq!(
        tex.as_str(),
        "a over b",
        "HWPX equation text must be stored verbatim; if this fails, \
         the equation handler now calls eqedit_to_latex — update this test \
         and the design comment in handlers.rs"
    );
}
