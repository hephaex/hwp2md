/// CLI integration tests — `--style` template flag.
///
/// These tests verify that the `to-hwpx --style` flag correctly applies
/// custom page layout, margin, and font settings to the generated HWPX ZIP.
use std::io::Read as _;
use tempfile::tempdir;

#[path = "common/mod.rs"]
mod common;

use common::cargo_bin;

// ---------------------------------------------------------------------------
// Shared helper
// ---------------------------------------------------------------------------

/// Writes `md_content` and `style_yaml` into a fresh temp directory, runs
/// `hwp2md to-hwpx input.md -o output.hwpx --style style.yaml`, and returns
/// `(TempDir, PathBuf)` where the `PathBuf` points to the produced HWPX file.
///
/// The `TempDir` must be kept alive by the caller so the directory is not
/// deleted before the HWPX file is read.
fn run_to_hwpx_with_style(
    md_content: &str,
    style_yaml: &str,
) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().expect("tempdir");

    let md_file = dir.path().join("input.md");
    std::fs::write(&md_file, md_content).expect("write input.md");

    let style_file = dir.path().join("style.yaml");
    std::fs::write(&style_file, style_yaml).expect("write style.yaml");

    let hwpx_path = dir.path().join("output.hwpx");
    let result = cargo_bin()
        .args([
            "to-hwpx",
            md_file.to_str().unwrap(),
            "-o",
            hwpx_path.to_str().unwrap(),
            "--style",
            style_file.to_str().unwrap(),
        ])
        .output()
        .expect("execute to-hwpx --style");

    assert!(
        result.status.success(),
        "to-hwpx --style failed; stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(hwpx_path.exists(), "output.hwpx was not created");

    (dir, hwpx_path)
}

// ---------------------------------------------------------------------------
// Sprint 15 — `--style` flag applies custom page dimensions to HWPX output
// ---------------------------------------------------------------------------

#[test]
fn to_hwpx_style_template_applies_page_dimensions() {
    let (_dir, hwpx_path) = run_to_hwpx_with_style(
        "# Hello\n\nTest document\n",
        "page:\n  width: 70000\n  height: 90000\n",
    );

    let hwpx_file = std::fs::File::open(&hwpx_path).expect("open hwpx");
    let mut archive = zip::ZipArchive::new(hwpx_file).expect("parse ZIP");
    let mut section_entry = archive
        .by_name("Contents/section0.xml")
        .expect("Contents/section0.xml not found in HWPX ZIP");

    let mut xml = String::new();
    section_entry
        .read_to_string(&mut xml)
        .expect("read section0.xml");

    // The pageSize element must carry the custom dimensions.
    assert!(
        xml.contains("width=\"70000\""),
        "expected width=\"70000\" in section0.xml, got:\n{xml}"
    );
    assert!(
        xml.contains("height=\"90000\""),
        "expected height=\"90000\" in section0.xml, got:\n{xml}"
    );
}

// ---------------------------------------------------------------------------
// Sprint 16 — `--style` flag applies custom margins to HWPX output
// ---------------------------------------------------------------------------

#[test]
fn to_hwpx_style_template_applies_margins() {
    let (_dir, hwpx_path) = run_to_hwpx_with_style(
        "# Hello\n\nTest document\n",
        "page:\n  margin:\n    left: 8000\n    right: 8000\n    top: 6000\n    bottom: 6000\n",
    );

    let hwpx_file = std::fs::File::open(&hwpx_path).expect("open hwpx");
    let mut archive = zip::ZipArchive::new(hwpx_file).expect("parse ZIP");
    let mut section_entry = archive
        .by_name("Contents/section0.xml")
        .expect("Contents/section0.xml not found in HWPX ZIP");

    let mut xml = String::new();
    section_entry
        .read_to_string(&mut xml)
        .expect("read section0.xml");

    assert!(
        xml.contains("left=\"8000\""),
        "expected left=\"8000\" in section0.xml, got:\n{xml}"
    );
    assert!(
        xml.contains("right=\"8000\""),
        "expected right=\"8000\" in section0.xml, got:\n{xml}"
    );
    assert!(
        xml.contains("top=\"6000\""),
        "expected top=\"6000\" in section0.xml, got:\n{xml}"
    );
    assert!(
        xml.contains("bottom=\"6000\""),
        "expected bottom=\"6000\" in section0.xml, got:\n{xml}"
    );
}

// ---------------------------------------------------------------------------
// Sprint 16 — `--style` flag applies custom font name to HWPX output
// ---------------------------------------------------------------------------

#[test]
fn to_hwpx_style_template_applies_custom_font() {
    let (_dir, hwpx_path) = run_to_hwpx_with_style(
        "# Hello\n\nTest document\n",
        "font:\n  default: \"맑은 고딕\"\n",
    );

    let hwpx_file = std::fs::File::open(&hwpx_path).expect("open hwpx");
    let mut archive = zip::ZipArchive::new(hwpx_file).expect("parse ZIP");
    let mut header_entry = archive
        .by_name("Contents/header.xml")
        .expect("Contents/header.xml not found in HWPX ZIP");

    let mut xml = String::new();
    header_entry
        .read_to_string(&mut xml)
        .expect("read header.xml");

    assert!(
        xml.contains("맑은 고딕"),
        "expected \"맑은 고딕\" in header.xml face attributes, got:\n{xml}"
    );
}
