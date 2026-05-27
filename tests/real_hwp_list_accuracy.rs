//! Validates that `detect_list_kind` classifies ordered lists correctly in
//! real Korean government HWP documents.
//!
//! All moel_0{1,3,4,5} files contain numbered list items (법령 조문 정의 목록).
//! The reader must classify them as `ordered = true` (not unordered bullet).
//! moel_02 contains both ordered and unordered list items; the test verifies
//! ordered lists dominate (≥ 50 % of all lists) as a baseline regression guard.
//!
//! Added in Sprint 78 S78-01 to provide regression coverage for the
//! HWPTAG_NUMBERING binary-search tier introduced in Sprint 77.

use hwp2md::{hwp, ir};

/// Extract every `Block::List` from a real `.hwp` fixture, traversing all
/// sections. Returns `(total_lists, ordered_lists)`.
fn list_stats(stem: &str) -> (usize, usize) {
    let path = format!("tests/fixtures/real/{stem}.hwp");
    let doc = hwp::read_hwp(std::path::Path::new(&path))
        .unwrap_or_else(|e| panic!("read_hwp failed for {stem}.hwp: {e}"));

    let all_blocks: Vec<ir::Block> = doc.sections.into_iter().flat_map(|s| s.blocks).collect();

    count_lists_in_blocks(&all_blocks)
}

/// Recursively count `Block::List` entries, descending into `BlockQuote` and
/// `ListItem` children so nested lists are not missed.
///
/// **Scope limitation**: does not descend into `Block::Table` cells,
/// `Block::Footnote` content, or section header/footer block lists.
/// Those contexts may contain lists in complex documents; this walker
/// covers the common top-level + blockquote + list-item cases.
fn count_lists_in_blocks(blocks: &[ir::Block]) -> (usize, usize) {
    let mut total = 0usize;
    let mut ordered = 0usize;

    for block in blocks {
        match block {
            ir::Block::List {
                ordered: is_ordered,
                items,
                ..
            } => {
                total += 1;
                if *is_ordered {
                    ordered += 1;
                }
                // Descend into list item children.
                for item in items {
                    let (t, o) = count_lists_in_items(item);
                    total += t;
                    ordered += o;
                }
            }
            ir::Block::BlockQuote { blocks } => {
                let (t, o) = count_lists_in_blocks(blocks);
                total += t;
                ordered += o;
            }
            _ => {}
        }
    }

    (total, ordered)
}

/// Recursively count lists nested inside a `ListItem`.
fn count_lists_in_items(item: &ir::ListItem) -> (usize, usize) {
    let mut total = 0usize;
    let mut ordered = 0usize;

    // Blocks inside the item (e.g. a sub-list rendered as Block::List).
    let (t, o) = count_lists_in_blocks(&item.blocks);
    total += t;
    ordered += o;

    // Explicit children (nested ListItem tree).
    for child in &item.children {
        let (t, o) = count_lists_in_items(child);
        total += t;
        ordered += o;
    }

    (total, ordered)
}

// ---------------------------------------------------------------------------
// Tests for fixtures that must have ordered lists
// ---------------------------------------------------------------------------

/// moel_01 is a community service center specification document.
/// It uses numbered list items throughout (법령 정의 목록).
#[test]
fn moel_01_has_ordered_lists() {
    let (total, ordered) = list_stats("moel_01_goyang_center");
    assert!(
        total > 0,
        "moel_01: expected at least one list block, got 0"
    );
    assert!(
        ordered > 0,
        "moel_01: expected at least one ordered list; total={total}, ordered={ordered}"
    );
}

/// moel_03 is a livelihood loan guideline with numbered definitions.
#[test]
fn moel_03_has_ordered_lists() {
    let (total, ordered) = list_stats("moel_03_livelihood_loan");
    assert!(
        total > 0,
        "moel_03: expected at least one list block, got 0"
    );
    assert!(
        ordered > 0,
        "moel_03: expected at least one ordered list; total={total}, ordered={ordered}"
    );
}

/// moel_04 is an instructor education regulation with numbered clauses.
#[test]
fn moel_04_has_ordered_lists() {
    let (total, ordered) = list_stats("moel_04_instructor_education");
    assert!(
        total > 0,
        "moel_04: expected at least one list block, got 0"
    );
    assert!(
        ordered > 0,
        "moel_04: expected at least one ordered list; total={total}, ordered={ordered}"
    );
}

/// moel_05 is a quality management manual with numbered items.
#[test]
fn moel_05_has_ordered_lists() {
    let (total, ordered) = list_stats("moel_05_quality_management");
    assert!(
        total > 0,
        "moel_05: expected at least one list block, got 0"
    );
    assert!(
        ordered > 0,
        "moel_05: expected at least one ordered list; total={total}, ordered={ordered}"
    );
}

// ---------------------------------------------------------------------------
// Baseline test for moel_02 (mixed ordered/unordered content)
// ---------------------------------------------------------------------------

/// moel_02 is a vocational training regulation with both numbered clauses and
/// sub-items that use unordered markers in the source HWP.
///
/// The critical regression guard is: ordered lists must dominate — if a future
/// change flips most numbered clauses to unordered that is a `detect_list_kind`
/// regression.  The threshold (≥ 50 %) is deliberately loose because the exact
/// mix depends on the HWPTAG_NUMBERING heuristic, which may evolve.
///
/// Baseline measured in Sprint 78: total=67, ordered=42, unordered=25
/// (≈ 63 % ordered).  A drop below 50 % is a regression signal.
#[test]
fn moel_02_ordered_lists_dominate() {
    let (total, ordered) = list_stats("moel_02_vocational_training");

    // Must have at least some lists (sanity — document is non-trivial).
    assert!(total > 0, "moel_02: expected at least one list block, got 0");

    // Ordered lists must dominate (50–85 % band).
    // Lower bound: a drop below 50 % signals ordered detection regressed.
    // Upper bound: a rise above 85 % signals unordered bullets are being
    // misclassified as ordered.  Baseline (Sprint 78): 42/67 ≈ 62.7 %.
    let pct_ordered = (ordered as f64) * 100.0 / (total as f64);
    assert!(
        (50.0..=85.0).contains(&pct_ordered),
        "moel_02: ordered ratio {pct_ordered:.1} % outside expected band 50–85 %; \
         total={total}, ordered={ordered}"
    );
}
