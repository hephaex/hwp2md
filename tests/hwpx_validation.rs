/// HWPX validation tests using polaris_dvc as a dev-dependency.
///
/// These tests verify that hwp2md's HWPX writer produces output that passes
/// polaris_dvc's container and structural integrity checks (JID 11000-12999).
#[path = "fixtures/mod.rs"]
mod fixtures;

use fixtures::{heading_xml, para_xml, table_2x2_xml, HwpxFixture};
use hwp2md::{hwpx, ir, md};
use polaris_dvc_core::engine::{CheckProfile, EngineOptions};
use polaris_dvc_core::rules::schema::RuleSpec;
use polaris_dvc_hwpx::open_bytes;

fn validate_hwpx_bytes(bytes: &[u8]) -> Vec<polaris_dvc_core::ViolationRecord> {
    let doc = match open_bytes(bytes) {
        Ok(d) => d,
        Err(e) => panic!("polaris_dvc failed to parse HWPX: {e}"),
    };
    let spec = RuleSpec::default();
    let opts = EngineOptions {
        stop_on_first: false,
        profile: CheckProfile::Extended,
        enable_schema: false,
    };
    let report = polaris_dvc_core::engine::validate(&doc, &spec, &opts);
    report.violations
}

fn validate_hwpx_bytes_with_schema(bytes: &[u8]) -> Vec<polaris_dvc_core::ViolationRecord> {
    let doc = match open_bytes(bytes) {
        Ok(d) => d,
        Err(e) => panic!("polaris_dvc failed to parse HWPX: {e}"),
    };
    let spec = RuleSpec::default();
    let opts = EngineOptions {
        stop_on_first: false,
        profile: CheckProfile::Extended,
        enable_schema: true,
    };
    let report = polaris_dvc_core::engine::validate(&doc, &spec, &opts);
    report.violations
}

fn container_integrity_violations(
    violations: &[polaris_dvc_core::ViolationRecord],
) -> Vec<&polaris_dvc_core::ViolationRecord> {
    violations
        .iter()
        .filter(|v| v.error_code.0 >= 11000 && v.error_code.0 < 13000)
        .collect()
}

// ---------------------------------------------------------------------------
// Phase 1: mimetype correctness
// ---------------------------------------------------------------------------

#[test]
fn writer_mimetype_is_hwp_plus_zip() {
    let doc = ir::Document::new();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    hwpx::write_hwpx(&doc, tmp.path(), None).unwrap();

    let bytes = std::fs::read(tmp.path()).unwrap();
    let content = std::str::from_utf8(&bytes[30..30 + 19]).ok();
    assert!(
        bytes.windows(19).any(|w| w == b"application/hwp+zip"),
        "mimetype should be application/hwp+zip; found: {content:?}"
    );
}

// ---------------------------------------------------------------------------
// Phase 2: polaris_dvc container validation
// ---------------------------------------------------------------------------

#[test]
fn writer_empty_doc_passes_polaris_dvc_parse() {
    let doc = ir::Document::new();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    hwpx::write_hwpx(&doc, tmp.path(), None).unwrap();

    let bytes = std::fs::read(tmp.path()).unwrap();
    let result = open_bytes(&bytes);
    assert!(
        result.is_ok(),
        "polaris_dvc should parse empty hwp2md output: {result:?}"
    );
}

#[test]
fn writer_paragraph_doc_passes_polaris_dvc_parse() {
    let md_text = "Hello, world!\n\nSecond paragraph.\n";
    let ir_doc = md::parse_markdown(md_text);

    let tmp = tempfile::NamedTempFile::new().unwrap();
    hwpx::write_hwpx(&ir_doc, tmp.path(), None).unwrap();

    let bytes = std::fs::read(tmp.path()).unwrap();
    let result = open_bytes(&bytes);
    assert!(result.is_ok(), "polaris_dvc parse failed: {result:?}");
}

#[test]
fn writer_heading_doc_passes_polaris_dvc_parse() {
    let md_text = "# Title\n\nBody text.\n\n## Section\n\nMore text.\n";
    let ir_doc = md::parse_markdown(md_text);

    let tmp = tempfile::NamedTempFile::new().unwrap();
    hwpx::write_hwpx(&ir_doc, tmp.path(), None).unwrap();

    let bytes = std::fs::read(tmp.path()).unwrap();
    let result = open_bytes(&bytes);
    assert!(result.is_ok(), "polaris_dvc parse failed: {result:?}");
}

#[test]
fn writer_table_doc_passes_polaris_dvc_parse() {
    let md_text = "| A | B |\n|---|---|\n| 1 | 2 |\n";
    let ir_doc = md::parse_markdown(md_text);

    let tmp = tempfile::NamedTempFile::new().unwrap();
    hwpx::write_hwpx(&ir_doc, tmp.path(), None).unwrap();

    let bytes = std::fs::read(tmp.path()).unwrap();
    let result = open_bytes(&bytes);
    assert!(result.is_ok(), "polaris_dvc parse failed: {result:?}");
}

#[test]
fn writer_mixed_doc_passes_polaris_dvc_parse() {
    let md_text = "# Doc Title\n\nIntro paragraph.\n\n## Data\n\n| X | Y |\n|---|---|\n| a | b |\n\n**Bold** and *italic* text.\n";
    let ir_doc = md::parse_markdown(md_text);

    let tmp = tempfile::NamedTempFile::new().unwrap();
    hwpx::write_hwpx(&ir_doc, tmp.path(), None).unwrap();

    let bytes = std::fs::read(tmp.path()).unwrap();
    let result = open_bytes(&bytes);
    assert!(result.is_ok(), "polaris_dvc parse failed: {result:?}");
}

// ---------------------------------------------------------------------------
// Phase 2: container + integrity validation (JID 11000-12999)
// ---------------------------------------------------------------------------

#[test]
fn writer_empty_doc_no_container_violations() {
    let doc = ir::Document::new();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    hwpx::write_hwpx(&doc, tmp.path(), None).unwrap();

    let bytes = std::fs::read(tmp.path()).unwrap();
    let violations = validate_hwpx_bytes(&bytes);
    let critical = container_integrity_violations(&violations);

    assert!(
        critical.is_empty(),
        "container/integrity violations on empty doc: {critical:#?}"
    );
}

#[test]
fn writer_paragraph_doc_no_container_violations() {
    let md_text = "Hello world.\n\nAnother paragraph.\n";
    let ir_doc = md::parse_markdown(md_text);

    let tmp = tempfile::NamedTempFile::new().unwrap();
    hwpx::write_hwpx(&ir_doc, tmp.path(), None).unwrap();

    let bytes = std::fs::read(tmp.path()).unwrap();
    let violations = validate_hwpx_bytes(&bytes);
    let critical = container_integrity_violations(&violations);

    assert!(
        critical.is_empty(),
        "container/integrity violations on paragraph doc: {critical:#?}"
    );
}

#[test]
fn writer_complex_doc_no_container_violations() {
    let md_text = "# Title\n\nText with **bold** and *italic*.\n\n## Table Section\n\n| Col1 | Col2 |\n|------|------|\n| A    | B    |\n| C    | D    |\n\n### Code\n\n```rust\nfn main() {}\n```\n\n---\n\nFinal paragraph.\n";
    let ir_doc = md::parse_markdown(md_text);

    let tmp = tempfile::NamedTempFile::new().unwrap();
    hwpx::write_hwpx(&ir_doc, tmp.path(), None).unwrap();

    let bytes = std::fs::read(tmp.path()).unwrap();
    let violations = validate_hwpx_bytes(&bytes);
    let critical = container_integrity_violations(&violations);

    assert!(
        critical.is_empty(),
        "container/integrity violations on complex doc: {critical:#?}"
    );
}

// ---------------------------------------------------------------------------
// Fixture-based validation (HWPX fixtures → polaris_dvc)
// ---------------------------------------------------------------------------

#[test]
fn fixture_passes_polaris_dvc_parse() {
    let body = format!(
        "{}{}{}",
        heading_xml(1, "Test"),
        para_xml("Content"),
        table_2x2_xml("A", "B", "C", "D"),
    );
    let bytes = HwpxFixture::new()
        .title("Fixture Test")
        .author("Mario")
        .section(&body)
        .build();

    let result = open_bytes(&bytes);
    assert!(
        result.is_ok(),
        "polaris_dvc failed to parse fixture: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Roundtrip: MD → HWPX → polaris_dvc validation
// ---------------------------------------------------------------------------

#[test]
fn markdown_to_hwpx_roundtrip_passes_validation() {
    let markdown = r#"# 한글 문서 제목

첫 번째 문단입니다.

## 두 번째 섹션

| 항목 | 값 |
|------|-----|
| A    | 100 |
| B    | 200 |

**굵은 텍스트**와 *기울임 텍스트*가 있습니다.
"#;

    let ir_doc = md::parse_markdown(markdown);
    let tmp = tempfile::NamedTempFile::new().unwrap();
    hwpx::write_hwpx(&ir_doc, tmp.path(), None).unwrap();

    let bytes = std::fs::read(tmp.path()).unwrap();
    let result = open_bytes(&bytes);
    assert!(
        result.is_ok(),
        "polaris_dvc parse failed on Korean content: {result:?}"
    );

    let violations = validate_hwpx_bytes(&bytes);
    let critical = container_integrity_violations(&violations);
    assert!(
        critical.is_empty(),
        "Korean doc container/integrity violations: {critical:#?}"
    );
}

// ---------------------------------------------------------------------------
// Phase 6: schema validation (JID 13000-13999)
// ---------------------------------------------------------------------------

/// Verify that an empty document produces no OWPML schema violations.
///
/// Schema violations use error codes in the 13000-13999 range (polaris_dvc
/// convention).  This test is marked `#[ignore]` if schema checks reveal
/// violations that require additional writer work beyond Phase 6 scope.
/// Remove the `#[ignore]` attribute once all schema violations are resolved.
#[test]
fn writer_empty_doc_no_schema_violations() {
    let doc = ir::Document::new();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    hwpx::write_hwpx(&doc, tmp.path(), None).unwrap();

    let bytes = std::fs::read(tmp.path()).unwrap();
    let violations = validate_hwpx_bytes_with_schema(&bytes);
    let schema_violations: Vec<_> = violations
        .iter()
        .filter(|v| v.error_code.0 >= 13000 && v.error_code.0 < 14000)
        .collect();

    assert!(
        schema_violations.is_empty(),
        "schema violations on empty doc: {schema_violations:#?}"
    );
}
