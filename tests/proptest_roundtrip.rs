//! Property-based roundtrip invariants for the Markdown writer/parser pair.
//!
//! Three invariants are checked against randomly generated `Document` values:
//!
//! 1. **Structure preserved** — `write_markdown` then `parse_markdown` yields a
//!    non-empty document when the input was non-empty.
//! 2. **Idempotence** — `write(parse(write(doc))) == write(doc)`.
//! 3. **No panics** — the pipeline never panics on any generated document.

use hwp2md::ir::{Block, Document, Inline, Section};
use hwp2md::md::{parse_markdown, write_markdown};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

/// Safe text: ASCII letters/digits/spaces and Korean syllables.
///
/// Excludes Markdown control characters (`*`, `_`, `` ` ``, `#`, `[`, `]`,
/// `(`, `)`, `\`, `~`, `|`, `>`, `-`, `!`) so that the rendered Markdown is
/// unambiguous for the parser to round-trip.
fn safe_text() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 가-힣]{1,40}"
        .prop_map(|s| s.trim().to_string())
        .prop_filter("non-empty after trim", |s| !s.is_empty())
}

fn inline_plain() -> impl Strategy<Value = Inline> {
    safe_text().prop_map(Inline::plain)
}

fn inlines() -> impl Strategy<Value = Vec<Inline>> {
    prop::collection::vec(inline_plain(), 1..5)
}

fn block_paragraph() -> BoxedStrategy<Block> {
    inlines()
        .prop_map(|inlines| Block::Paragraph { inlines })
        .boxed()
}

fn block_heading() -> BoxedStrategy<Block> {
    (1u8..=6u8, inlines())
        .prop_map(|(level, inlines)| Block::Heading { level, inlines })
        .boxed()
}

/// Code blocks: language tag restricted to `[a-z]{2,10}` (or absent),
/// body restricted to printable ASCII + newlines.  Backtick sequences inside
/// the body would break fenced-code parsing, so they are excluded.
fn block_code() -> BoxedStrategy<Block> {
    (
        proptest::option::of("[a-z]{2,10}"),
        "[a-zA-Z0-9 \n]{1,200}",
    )
        .prop_map(|(language, code)| Block::CodeBlock { language, code })
        .boxed()
}

fn block_horizontal_rule() -> BoxedStrategy<Block> {
    Just(Block::HorizontalRule).boxed()
}

fn block_simple() -> BoxedStrategy<Block> {
    prop_oneof![
        block_paragraph(),
        block_heading(),
        block_code(),
        block_horizontal_rule(),
    ]
    .boxed()
}

fn simple_document() -> impl Strategy<Value = Document> {
    prop::collection::vec(block_simple(), 1..8).prop_map(|blocks| {
        let section = Section {
            blocks,
            page_layout: None,
            header: None,
            footer: None,
            header_footer_type: None,
        };
        let mut doc = Document::new();
        doc.sections.push(section);
        doc
    })
}

// ---------------------------------------------------------------------------
// Invariants
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 128,
        ..Default::default()
    })]

    /// Invariant 1: write → parse → blocks roundtrip preserves content presence.
    ///
    /// At minimum, a non-empty input document must yield a non-empty parsed
    /// result.  Block counts are not required to match exactly because the
    /// Markdown writer may merge consecutive elements (e.g. two adjacent
    /// `HorizontalRule`s both render as `---`, and the parser may rejoin them).
    #[test]
    fn roundtrip_preserves_block_structure(doc in simple_document()) {
        let md = write_markdown(&doc, false);
        let parsed = parse_markdown(&md);

        prop_assert!(
            !parsed.sections.is_empty(),
            "parse_markdown returned no sections for input:\n{md}"
        );

        let orig_blocks = &doc.sections[0].blocks;
        let parsed_blocks = &parsed.sections[0].blocks;

        if !orig_blocks.is_empty() {
            prop_assert!(
                !parsed_blocks.is_empty(),
                "write_markdown produced output but parse_markdown yielded no blocks;\
                 \nMarkdown:\n{md}"
            );
        }
    }

    /// Invariant 2: write is idempotent — `write(parse(write(doc))) == write(doc)`.
    ///
    /// # Known divergences
    ///
    /// Code-block bodies are passed through verbatim by the writer, but the
    /// CommonMark parser normalises trailing whitespace inside fenced blocks.
    /// Concretely, `"code \n"` is written as-is but parsed back as `"code"`,
    /// producing a divergence on the second write pass.
    ///
    /// To keep the invariant testable without disabling it, code blocks whose
    /// body contains trailing whitespace on any line are skipped via
    /// `prop_assume!`.  All other document shapes must be fully idempotent.
    #[test]
    fn write_is_idempotent(doc in simple_document()) {
        // Skip documents where a code-block body has trailing whitespace on any
        // line, because CommonMark parsers normalise that and the second write
        // pass will differ from the first.
        let has_trailing_ws_in_code = doc.sections.iter().any(|sec| {
            sec.blocks.iter().any(|blk| {
                if let Block::CodeBlock { code, .. } = blk {
                    code.lines().any(|l| l != l.trim_end())
                } else {
                    false
                }
            })
        });
        prop_assume!(!has_trailing_ws_in_code);

        let md1 = write_markdown(&doc, false);
        let doc2 = parse_markdown(&md1);
        let md2 = write_markdown(&doc2, false);

        prop_assert_eq!(
            &md1,
            &md2,
            "write_markdown is not idempotent\nFirst pass:\n{}\nSecond pass:\n{}",
            md1,
            md2
        );
    }

    /// Invariant 3: no panics — `write_markdown` then `parse_markdown` never
    /// panics for any randomly-generated document.  Reaching the end of the
    /// test body without a panic is the only assertion.
    #[test]
    fn no_panics_on_random_documents(doc in simple_document()) {
        let md = write_markdown(&doc, false);
        let _parsed = parse_markdown(&md);
    }
}
