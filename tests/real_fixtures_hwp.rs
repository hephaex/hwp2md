//! Real-fixture comparison harness for HWP 5.0 documents.
//!
//! Each test loads one of the five real Ministry of Employment and Labour HWP
//! files from `tests/fixtures/real/`, converts it to Markdown via the HWP
//! reader pipeline, and compares the output against the paired `.md` golden
//! file.
//!
//! # Status
//!
//! All comparison tests are marked `#[ignore]`.  They will be activated after
//! the golden files are regenerated in Sprint 52 to reflect the post-Sprint-49
//! encoding fix.  The harness exists to define the testing contract, not to
//! assert correctness against the current (pre-fix) golden files.
//!
//! The one *non-ignored* test (`real_fixtures_no_garbled_chars`) is a live
//! regression guard: it asserts that none of the converted outputs contain the
//! garbled byte sequences (湰灧, 桤灧) introduced by the Sprint 49 bug.

use std::path::Path;

use hwp2md::{hwp, md};

/// Directory that holds the real HWP fixtures and their paired golden files.
const FIXTURES_DIR: &str = "tests/fixtures/real";

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Convert a real `.hwp` file to a Markdown `String`.
///
/// Panics with a descriptive message on I/O or parse failure so that test
/// output clearly identifies which fixture broke and why.
fn convert_hwp_to_md(stem: &str) -> String {
    let hwp_path = Path::new(FIXTURES_DIR).join(format!("{stem}.hwp"));
    let doc = hwp::read_hwp(&hwp_path)
        .unwrap_or_else(|e| panic!("read_hwp failed for {stem}.hwp: {e}"));
    md::write_markdown(&doc, false)
}

/// Load the golden Markdown file paired with `stem`.
fn load_golden(stem: &str) -> String {
    let md_path = Path::new(FIXTURES_DIR).join(format!("{stem}.md"));
    std::fs::read_to_string(&md_path)
        .unwrap_or_else(|e| panic!("could not read golden file {stem}.md: {e}"))
}

/// Structural comparison: counts lines that begin with one or more `#`
/// characters (ATX headings) and returns how many appear in `text`.
fn count_heading_lines(text: &str) -> usize {
    text.lines()
        .filter(|l| l.starts_with('#'))
        .count()
}

/// Returns `true` if `text` contains any of the known garbled byte sequences
/// produced by incorrect Latin-1 / EUC-KR misinterpretation of HWP streams.
fn contains_garbled_chars(text: &str) -> bool {
    // These sequences were the concrete manifestation of the Sprint 49 bug:
    // the raw bytes of a Korean string were decoded as Latin-1 and then
    // re-interpreted as UTF-8 CJK codepoints.
    const GARBLED: &[&str] = &[
        "湰灧", // \xC6\xC0 \xB1\xE7 misread
        "桤灧", // \xC7\xE1 \xB1\xE7 misread
        "潤灧", // variant seen in moel_02
        "汰灧", // variant seen in moel_03
        "浰灧", // variant seen in moel_04
    ];
    GARBLED.iter().any(|pat| text.contains(pat))
}

// ---------------------------------------------------------------------------
// Regression guard (NOT ignored — runs in every CI pass)
// ---------------------------------------------------------------------------

/// Verify that converting all five real HWP fixtures produces no garbled
/// character sequences from the Sprint 49 bug.
///
/// This test is intentionally *not* `#[ignore]` — it is a live regression
/// guard that must stay green at all times.
#[test]
fn real_fixtures_no_garbled_chars() {
    let fixtures = [
        "moel_01_goyang_center",
        "moel_02_vocational_training",
        "moel_03_livelihood_loan",
        "moel_04_instructor_education",
        "moel_05_quality_management",
    ];

    let mut failures: Vec<String> = Vec::new();

    for stem in fixtures {
        let md_output = convert_hwp_to_md(stem);
        if contains_garbled_chars(&md_output) {
            // Collect first offending line for diagnostics.
            let offending_line = md_output
                .lines()
                .find(|l| contains_garbled_chars(l))
                .unwrap_or("<no line found>");
            failures.push(format!(
                "  {stem}: first garbled line: {offending_line:?}"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "garbled character sequences found in converted output:\n{}",
        failures.join("\n")
    );
}

// ---------------------------------------------------------------------------
// Golden-file comparison tests (ignored until Sprint 52 golden regeneration)
// ---------------------------------------------------------------------------

/// Full exact-equality comparison between the converted Markdown and the
/// golden file.  Exact equality is the strongest possible assertion; it will
/// be activated once the golden files are regenerated to reflect the
/// post-Sprint-49 output.
///
/// If exact equality proves too fragile (whitespace differences, trailing
/// newlines, etc.) this can be replaced by the structural comparison below.
#[test]
#[ignore = "activate after golden file regeneration in Sprint 52"]
fn real_fixture_moel_01_goyang_center_exact() {
    let stem = "moel_01_goyang_center";
    let actual = convert_hwp_to_md(stem);
    let golden = load_golden(stem);
    assert_eq!(
        actual, golden,
        "converted markdown does not match golden file for {stem}"
    );
}

#[test]
#[ignore = "activate after golden file regeneration in Sprint 52"]
fn real_fixture_moel_02_vocational_training_exact() {
    let stem = "moel_02_vocational_training";
    let actual = convert_hwp_to_md(stem);
    let golden = load_golden(stem);
    assert_eq!(
        actual, golden,
        "converted markdown does not match golden file for {stem}"
    );
}

#[test]
#[ignore = "activate after golden file regeneration in Sprint 52"]
fn real_fixture_moel_03_livelihood_loan_exact() {
    let stem = "moel_03_livelihood_loan";
    let actual = convert_hwp_to_md(stem);
    let golden = load_golden(stem);
    assert_eq!(
        actual, golden,
        "converted markdown does not match golden file for {stem}"
    );
}

#[test]
#[ignore = "activate after golden file regeneration in Sprint 52"]
fn real_fixture_moel_04_instructor_education_exact() {
    let stem = "moel_04_instructor_education";
    let actual = convert_hwp_to_md(stem);
    let golden = load_golden(stem);
    assert_eq!(
        actual, golden,
        "converted markdown does not match golden file for {stem}"
    );
}

#[test]
#[ignore = "activate after golden file regeneration in Sprint 52"]
fn real_fixture_moel_05_quality_management_exact() {
    let stem = "moel_05_quality_management";
    let actual = convert_hwp_to_md(stem);
    let golden = load_golden(stem);
    assert_eq!(
        actual, golden,
        "converted markdown does not match golden file for {stem}"
    );
}

// ---------------------------------------------------------------------------
// Structural comparison tests (ignored until Sprint 52 golden regeneration)
//
// These are lighter-weight alternatives to exact equality: they verify that
// the heading structure is approximately preserved without being sensitive to
// whitespace or minor textual differences.
// ---------------------------------------------------------------------------

/// Structural comparison for moel_01: heading count must be within ±10% of
/// the golden file's heading count, and no garbled characters may appear.
#[test]
#[ignore = "activate after golden file regeneration in Sprint 52"]
fn real_fixture_moel_01_goyang_center_structural() {
    let stem = "moel_01_goyang_center";
    let actual = convert_hwp_to_md(stem);
    let golden = load_golden(stem);

    assert!(
        !contains_garbled_chars(&actual),
        "{stem}: converted output contains garbled characters"
    );

    let actual_headings = count_heading_lines(&actual);
    let golden_headings = count_heading_lines(&golden);
    let tolerance = (golden_headings as f64 * 0.10).ceil() as usize + 1;

    assert!(
        actual_headings.abs_diff(golden_headings) <= tolerance,
        "{stem}: heading count mismatch — actual {actual_headings}, golden {golden_headings}, tolerance ±{tolerance}"
    );
}

#[test]
#[ignore = "activate after golden file regeneration in Sprint 52"]
fn real_fixture_moel_02_vocational_training_structural() {
    let stem = "moel_02_vocational_training";
    let actual = convert_hwp_to_md(stem);
    let golden = load_golden(stem);

    assert!(
        !contains_garbled_chars(&actual),
        "{stem}: converted output contains garbled characters"
    );

    let actual_headings = count_heading_lines(&actual);
    let golden_headings = count_heading_lines(&golden);
    let tolerance = (golden_headings as f64 * 0.10).ceil() as usize + 1;

    assert!(
        actual_headings.abs_diff(golden_headings) <= tolerance,
        "{stem}: heading count mismatch — actual {actual_headings}, golden {golden_headings}, tolerance ±{tolerance}"
    );
}

#[test]
#[ignore = "activate after golden file regeneration in Sprint 52"]
fn real_fixture_moel_03_livelihood_loan_structural() {
    let stem = "moel_03_livelihood_loan";
    let actual = convert_hwp_to_md(stem);
    let golden = load_golden(stem);

    assert!(
        !contains_garbled_chars(&actual),
        "{stem}: converted output contains garbled characters"
    );

    let actual_headings = count_heading_lines(&actual);
    let golden_headings = count_heading_lines(&golden);
    let tolerance = (golden_headings as f64 * 0.10).ceil() as usize + 1;

    assert!(
        actual_headings.abs_diff(golden_headings) <= tolerance,
        "{stem}: heading count mismatch — actual {actual_headings}, golden {golden_headings}, tolerance ±{tolerance}"
    );
}

#[test]
#[ignore = "activate after golden file regeneration in Sprint 52"]
fn real_fixture_moel_04_instructor_education_structural() {
    let stem = "moel_04_instructor_education";
    let actual = convert_hwp_to_md(stem);
    let golden = load_golden(stem);

    assert!(
        !contains_garbled_chars(&actual),
        "{stem}: converted output contains garbled characters"
    );

    let actual_headings = count_heading_lines(&actual);
    let golden_headings = count_heading_lines(&golden);
    let tolerance = (golden_headings as f64 * 0.10).ceil() as usize + 1;

    assert!(
        actual_headings.abs_diff(golden_headings) <= tolerance,
        "{stem}: heading count mismatch — actual {actual_headings}, golden {golden_headings}, tolerance ±{tolerance}"
    );
}

#[test]
#[ignore = "activate after golden file regeneration in Sprint 52"]
fn real_fixture_moel_05_quality_management_structural() {
    let stem = "moel_05_quality_management";
    let actual = convert_hwp_to_md(stem);
    let golden = load_golden(stem);

    assert!(
        !contains_garbled_chars(&actual),
        "{stem}: converted output contains garbled characters"
    );

    let actual_headings = count_heading_lines(&actual);
    let golden_headings = count_heading_lines(&golden);
    let tolerance = (golden_headings as f64 * 0.10).ceil() as usize + 1;

    assert!(
        actual_headings.abs_diff(golden_headings) <= tolerance,
        "{stem}: heading count mismatch — actual {actual_headings}, golden {golden_headings}, tolerance ±{tolerance}"
    );
}
