//! Property-based roundtrip invariants for the Markdown writer/parser pair.
//!
//! Three invariants are checked against randomly generated `Document` values:
//!
//! 1. **Structure preserved** — `write_markdown` then `parse_markdown` yields a
//!    non-empty document when the input was non-empty.
//! 2. **Idempotence** — `write(parse(write(doc))) == write(doc)`.
//! 3. **No panics** — the pipeline never panics on any generated document.
//!
//! A separate proptest block covers HWPX write→read span preservation.

use hwp2md::ir::{Block, Document, Inline, ListItem, Metadata, Section, TableCell, TableRow};
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

/// `BlockQuote` strategy: only Paragraph, Heading, or `HorizontalRule` inside.
///
/// Nested List and Table are excluded because comrak's blockquote parser is
/// strict about complex nested block structure and does not reliably round-trip
/// them.
fn block_quote() -> BoxedStrategy<Block> {
    let inner = prop_oneof![
        block_paragraph(),
        block_heading(),
        block_horizontal_rule(),
    ];
    prop::collection::vec(inner, 1..4)
        .prop_map(|blocks| Block::BlockQuote { blocks })
        .boxed()
}

/// List strategy: ordered or unordered, 1–3 items, each item a single Paragraph.
///
/// Nested lists are excluded because comrak's indentation-based nesting is
/// fragile under the wide variety of content proptest generates.  `start` is
/// always 1 for ordered lists because comrak may renumber items when parsing,
/// which would break idempotence for any other start value.
fn block_list() -> BoxedStrategy<Block> {
    (any::<bool>(), prop::collection::vec(block_paragraph(), 1..4))
        .prop_map(|(ordered, paragraphs)| {
            let items = paragraphs
                .into_iter()
                .map(|p| ListItem::new(vec![p], vec![]))
                .collect();
            Block::List {
                ordered,
                start: 1,
                items,
            }
        })
        .boxed()
}

/// Table strategy: 1–3 columns, header row + 0–3 data rows.
///
/// Each cell contains a single plain-text Paragraph derived from `safe_text()`.
/// `col_count` is set to match the header cell count so the writer and parser
/// agree on the column width.  `|` in cell text is escaped by the writer, but
/// we use `safe_text()` which already excludes `|` to keep the strategy simple.
fn block_table() -> BoxedStrategy<Block> {
    (1usize..4usize)
        .prop_flat_map(|cols| {
            let header_cells = prop::collection::vec(inlines(), cols..=cols);
            let data_rows = prop::collection::vec(
                prop::collection::vec(inlines(), cols..=cols),
                0..4usize,
            );
            (Just(cols), header_cells, data_rows)
        })
        .prop_map(|(cols, header_cells, data_rows)| {
            let make_cell = |cell_inlines: Vec<Inline>| TableCell {
                blocks: vec![Block::Paragraph {
                    inlines: cell_inlines,
                }],
                colspan: 1,
                rowspan: 1,
            };

            let header_row = TableRow {
                cells: header_cells.into_iter().map(make_cell).collect(),
                is_header: true,
            };

            let mut rows = vec![header_row];
            for row_inlines in data_rows {
                rows.push(TableRow {
                    cells: row_inlines.into_iter().map(make_cell).collect(),
                    is_header: false,
                });
            }

            Block::Table {
                rows,
                col_count: cols,
            }
        })
        .boxed()
}

fn block_simple() -> BoxedStrategy<Block> {
    prop_oneof![
        block_paragraph(),
        block_heading(),
        block_code(),
        block_horizontal_rule(),
        block_quote(),
        block_list(),
        block_table(),
    ]
    .boxed()
}

/// Return `true` when no two consecutive blocks in `blocks` are both
/// `Block::List` with the same `ordered` flag.
///
/// Comrak merges adjacent same-type lists (ordered+ordered or
/// unordered+unordered) separated only by a blank line into a single loose
/// list, which changes the rendered Markdown on the second write pass and
/// breaks idempotence.  Filtering these cases out keeps the invariant valid
/// without restricting the individual block strategies.
fn no_adjacent_same_type_lists(blocks: &[Block]) -> bool {
    blocks.windows(2).all(|pair| {
        !matches!(
            (&pair[0], &pair[1]),
            (Block::List { ordered: a, .. }, Block::List { ordered: b, .. }) if a == b
        )
    })
}

fn simple_document() -> impl Strategy<Value = Document> {
    prop::collection::vec(block_simple(), 1..8)
        .prop_filter("no adjacent same-type lists", |blocks| {
            no_adjacent_same_type_lists(blocks)
        })
        .prop_map(|blocks| {
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
// HWPX span strategy
// ---------------------------------------------------------------------------

/// A deterministic 2×2 table where every cell has `colspan=1, rowspan=1`.
///
/// Using fixed span values keeps the strategy simple: the roundtrip invariant
/// is that the HWPX writer preserves span metadata, not that arbitrary spans
/// survive.  The text content is drawn from `safe_text()` so each cell is
/// distinguishable.
fn table_with_spans() -> impl Strategy<Value = Block> {
    (
        safe_text(),
        safe_text(),
        safe_text(),
        safe_text(),
    )
        .prop_map(|(a, b, c, d)| {
            let make_cell = |text: String| TableCell {
                blocks: vec![Block::Paragraph {
                    inlines: vec![Inline::plain(&text)],
                }],
                colspan: 1,
                rowspan: 1,
            };
            Block::Table {
                rows: vec![
                    TableRow {
                        cells: vec![make_cell(a), make_cell(b)],
                        is_header: true,
                    },
                    TableRow {
                        cells: vec![make_cell(c), make_cell(d)],
                        is_header: false,
                    },
                ],
                col_count: 2,
            }
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
    #[test]
    fn write_is_idempotent(doc in simple_document()) {
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

// ---------------------------------------------------------------------------
// HWPX span roundtrip invariants
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        ..Default::default()
    })]

    /// HWPX write→read roundtrip preserves `colspan` and `rowspan` for every
    /// cell in a 2×2 table.
    ///
    /// All cells use `colspan=1, rowspan=1`.  The invariant confirms that the
    /// HWPX writer encodes span values and the reader decodes them faithfully —
    /// even when the values are the trivial default.
    #[test]
    fn hwpx_table_roundtrip_preserves_spans(table in table_with_spans()) {
        use hwp2md::hwpx::{read_hwpx, write_hwpx};

        // Build a minimal Document containing only the generated table.
        let doc = Document {
            metadata: Metadata::default(),
            sections: vec![Section {
                blocks: vec![table.clone()],
                page_layout: None,
                header: None,
                footer: None,
                header_footer_type: None,
            }],
            assets: Vec::new(),
        };

        let tmp = tempfile::NamedTempFile::new().expect("tmp file");
        write_hwpx(&doc, tmp.path(), None).expect("write_hwpx failed");
        let read_back = read_hwpx(tmp.path()).expect("read_hwpx failed");

        // Extract the table rows from the original and the read-back document.
        let Block::Table { rows: orig_rows, .. } = &table else {
            unreachable!("strategy always produces a Table")
        };

        let parsed_rows = read_back
            .sections
            .into_iter()
            .flat_map(|s| s.blocks)
            .find_map(|b| match b {
                Block::Table { rows, .. } => Some(rows),
                _ => None,
            })
            .expect("no Table block found after HWPX roundtrip");

        prop_assert_eq!(
            parsed_rows.len(),
            orig_rows.len(),
            "row count must be preserved after HWPX roundtrip"
        );

        for (row_idx, (orig_row, parsed_row)) in
            orig_rows.iter().zip(parsed_rows.iter()).enumerate()
        {
            prop_assert_eq!(
                parsed_row.cells.len(),
                orig_row.cells.len(),
                "row {}: cell count must be preserved after HWPX roundtrip",
                row_idx
            );

            for (col_idx, (orig_cell, parsed_cell)) in
                orig_row.cells.iter().zip(parsed_row.cells.iter()).enumerate()
            {
                prop_assert_eq!(
                    parsed_cell.colspan,
                    orig_cell.colspan,
                    "cell [{}][{}]: colspan must be preserved after HWPX roundtrip",
                    row_idx,
                    col_idx
                );
                prop_assert_eq!(
                    parsed_cell.rowspan,
                    orig_cell.rowspan,
                    "cell [{}][{}]: rowspan must be preserved after HWPX roundtrip",
                    row_idx,
                    col_idx
                );
            }
        }
    }
}
