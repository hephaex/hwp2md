/// CLI integration tests: exercise the `hwp2md` binary end-to-end.
///
/// Each test spawns the compiled binary via `std::process::Command` and
/// verifies exit codes and output text.  A minimal valid HWPX is produced by
/// first calling `to-hwpx` on a temp `.md` file, which guarantees a
/// well-formed ZIP that the `to-md` / `info` subcommands can read back.
use std::io::Write as _;
use std::process::Command;
use tempfile::tempdir;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn cargo_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_hwp2md"))
}

// ---------------------------------------------------------------------------
// 1. No arguments → non-zero exit, stderr contains usage hint
// ---------------------------------------------------------------------------

#[test]
fn cli_no_args_shows_help() {
    let output = cargo_bin().output().expect("failed to execute hwp2md");
    assert!(
        !output.status.success(),
        "expected non-zero exit with no args"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Usage") || stderr.contains("usage"),
        "expected 'Usage' in stderr, got: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// 2. --version → stdout contains version string
// ---------------------------------------------------------------------------

#[test]
fn cli_version_flag() {
    let output = cargo_bin()
        .arg("--version")
        .output()
        .expect("failed to execute hwp2md --version");
    assert!(
        output.status.success(),
        "expected zero exit for --version; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // The version is declared in Cargo.toml; clap renders it as "hwp2md X.Y.Z".
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "version number not found in stdout: {stdout}"
    );
}

// ---------------------------------------------------------------------------
// 3. --help → stdout contains binary name and all subcommand names
// ---------------------------------------------------------------------------

#[test]
fn cli_help_flag() {
    let output = cargo_bin()
        .arg("--help")
        .output()
        .expect("failed to execute hwp2md --help");
    assert!(
        output.status.success(),
        "expected zero exit for --help; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("hwp2md"), "binary name missing: {stdout}");
    assert!(
        stdout.contains("to-md"),
        "to-md subcommand missing: {stdout}"
    );
    assert!(
        stdout.contains("to-hwpx"),
        "to-hwpx subcommand missing: {stdout}"
    );
    assert!(stdout.contains("info"), "info subcommand missing: {stdout}");
}

// ---------------------------------------------------------------------------
// 4. to-md --help → shows input/output/assets-dir/frontmatter options
// ---------------------------------------------------------------------------

#[test]
fn cli_to_md_help() {
    let output = cargo_bin()
        .args(["to-md", "--help"])
        .output()
        .expect("failed to execute hwp2md to-md --help");
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
    assert!(
        stdout.contains("assets-dir") || stdout.contains("assets"),
        "assets-dir option missing: {stdout}"
    );
    assert!(
        stdout.contains("frontmatter"),
        "frontmatter option missing: {stdout}"
    );
}

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
// 6. info --help → shows input option
// ---------------------------------------------------------------------------

#[test]
fn cli_info_help() {
    let output = cargo_bin()
        .args(["info", "--help"])
        .output()
        .expect("failed to execute hwp2md info --help");
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
}

// ---------------------------------------------------------------------------
// 7. to-md nonexistent.hwp → non-zero exit, error about file not found
// ---------------------------------------------------------------------------

#[test]
fn cli_to_md_nonexistent_file() {
    let output = cargo_bin()
        .args(["to-md", "/nonexistent/path/file.hwp"])
        .output()
        .expect("failed to execute hwp2md to-md");
    assert!(
        !output.status.success(),
        "expected non-zero exit for missing file"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    // anyhow wraps OS errors; the message will contain the path or "No such file"
    assert!(
        stderr.contains("nonexistent")
            || stderr.contains("No such file")
            || stderr.contains("not found")
            || stderr.contains("os error"),
        "expected file-not-found error, got: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// 8. to-md invalid.txt → non-zero exit, error about unsupported format
// ---------------------------------------------------------------------------

#[test]
fn cli_to_md_unsupported_format() {
    let dir = tempdir().expect("tempdir");
    let txt_file = dir.path().join("document.txt");
    std::fs::write(&txt_file, b"some content").expect("write txt");

    let output = cargo_bin()
        .args(["to-md", txt_file.to_str().unwrap()])
        .output()
        .expect("failed to execute hwp2md to-md");
    assert!(
        !output.status.success(),
        "expected non-zero exit for unsupported extension"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Unsupported") || stderr.contains("unsupported"),
        "expected unsupported-format error, got: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// 9. info nonexistent.hwpx → non-zero exit, error message
// ---------------------------------------------------------------------------

#[test]
fn cli_info_nonexistent_file() {
    let output = cargo_bin()
        .args(["info", "/nonexistent/path/file.hwpx"])
        .output()
        .expect("failed to execute hwp2md info");
    assert!(
        !output.status.success(),
        "expected non-zero exit for missing file"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.is_empty(),
        "expected some error message on stderr, got empty"
    );
}

// ---------------------------------------------------------------------------
// 10. to-md with a real HWPX produced by to-hwpx
//     → exit 0, stdout contains markdown
// ---------------------------------------------------------------------------

#[test]
fn cli_to_md_from_hwpx() {
    let dir = tempdir().expect("tempdir");

    // Step 1: create a markdown source file.
    let md_src = dir.path().join("source.md");
    std::fs::write(&md_src, b"# Integration Test\n\nHello from CLI test.\n").expect("write md");

    // Step 2: use `to-hwpx` to produce a valid HWPX.
    let hwpx_path = dir.path().join("output.hwpx");
    let convert_out = cargo_bin()
        .args([
            "to-hwpx",
            md_src.to_str().unwrap(),
            "--output",
            hwpx_path.to_str().unwrap(),
        ])
        .output()
        .expect("failed to execute hwp2md to-hwpx");
    assert!(
        convert_out.status.success(),
        "to-hwpx failed; stderr: {}",
        String::from_utf8_lossy(&convert_out.stderr)
    );
    assert!(hwpx_path.exists(), "HWPX file was not created");

    // Step 3: run `to-md` on the produced HWPX; expect markdown on stdout.
    let md_out = cargo_bin()
        .args(["to-md", hwpx_path.to_str().unwrap()])
        .output()
        .expect("failed to execute hwp2md to-md");
    assert!(
        md_out.status.success(),
        "to-md failed; stderr: {}",
        String::from_utf8_lossy(&md_out.stderr)
    );
    let stdout = String::from_utf8_lossy(&md_out.stdout);
    assert!(
        stdout.contains("Integration Test") || stdout.contains('#'),
        "expected markdown content in stdout, got: {stdout}"
    );
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
