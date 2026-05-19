/// CLI integration tests — `to-hwpx` subcommand.
///
/// Each test spawns the compiled binary via `std::process::Command` and
/// verifies exit codes, output files, and ZIP structure.
use std::io::Write as _;
use tempfile::tempdir;

#[path = "common/mod.rs"]
mod common;

use common::cargo_bin;

// ---------------------------------------------------------------------------
// 5. to-hwpx --help → shows input/output/style options
// ---------------------------------------------------------------------------

#[test]
fn cli_to_hwpx_help() {
    let output = cargo_bin()
        .args(["to-hwpx", "--help"])
        .output()
        .expect("failed to execute hwp2md to-hwpx --help");
    assert!(
        output.status.success(),
        "expected zero exit; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("input") || stdout.contains("INPUT"),
        "input option missing: {stdout}"
    );
    assert!(
        stdout.contains("output") || stdout.contains("OUTPUT"),
        "output option missing: {stdout}"
    );
    assert!(stdout.contains("style"), "style option missing: {stdout}");
}

// ---------------------------------------------------------------------------
// 11. to-hwpx with a temp .md file → exit 0, output .hwpx exists
// ---------------------------------------------------------------------------

#[test]
fn cli_to_hwpx_from_md() {
    let dir = tempdir().expect("tempdir");
    let md_file = dir.path().join("hello.md");

    {
        let mut f = std::fs::File::create(&md_file).expect("create md");
        writeln!(f, "# Hello\n\nWorld paragraph.").expect("write md");
    }

    let hwpx_out = dir.path().join("hello.hwpx");
    let output = cargo_bin()
        .args([
            "to-hwpx",
            md_file.to_str().unwrap(),
            "--output",
            hwpx_out.to_str().unwrap(),
        ])
        .output()
        .expect("failed to execute hwp2md to-hwpx");
    assert!(
        output.status.success(),
        "to-hwpx failed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(hwpx_out.exists(), "expected .hwpx output file to exist");

    // Sanity-check that it is a ZIP (PK magic bytes).
    let bytes = std::fs::read(&hwpx_out).expect("read hwpx");
    assert!(
        bytes.starts_with(b"PK"),
        "output does not start with PK magic; got {:?}",
        &bytes[..4.min(bytes.len())]
    );
}
